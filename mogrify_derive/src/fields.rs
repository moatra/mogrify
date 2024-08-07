use crate::attrs::{extract_mogrify_meta, MogrifyFieldAttrs};
use proc_macro2::Span;
use quote::{quote, TokenStreamExt};
use syn::{Field, GenericArgument, Ident, PathArguments, Type, TypePath};

pub(crate) struct MogrifyFieldInfo {
    pub(crate) idx: usize,
    pub(crate) local_ident: Ident,
    pub(crate) source_ident: Option<Ident>,
    pub(crate) attrs: MogrifyFieldAttrs,
    pub(crate) specialization: MogrifyFieldSpecialization,
}

#[allow(dead_code)]
pub(crate) enum MogrifyFieldSpecialization {
    None {
        ty: Type,
    },
    Option {
        ty: GenericArgument,
    },
    Vec {
        ty: GenericArgument,
    },
    Map {
        key_ty: GenericArgument,
        value_ty: GenericArgument,
    },
}
impl MogrifyFieldInfo {
    pub(crate) fn destructure_expr(&self) -> proc_macro2::TokenStream {
        let Self {
            local_ident,
            source_ident,
            ..
        } = self;
        match source_ident {
            None => quote!(#local_ident),
            Some(source_ident) => quote!(#source_ident: #local_ident),
        }
    }
    pub(crate) fn assignment_expr(&self) -> proc_macro2::TokenStream {
        let Self {
            local_ident,
            source_ident,
            ..
        } = self;
        match source_ident {
            None => quote!(#local_ident.unwrap()),
            Some(source_ident) => quote!(#source_ident: #local_ident.unwrap()),
        }
    }
    pub(crate) fn conversion(&self, field_count: usize) -> proc_macro2::TokenStream {
        let Self {
            idx,
            local_ident,
            source_ident,
            attrs,
            specialization,
        } = self;
        let mut conversion_expr = match &attrs.default {
            None => quote!(Ok(#local_ident)),
            Some(None) => {
                quote!(Ok(#local_ident.unwrap_or_default()))
            }
            Some(Some(expr)) => {
                quote!(Ok(#local_ident.unwrap_or(#expr)))
            }
        };
        if attrs.require {
            conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_require)));
        }
        if attrs.raw {
            match &attrs.parse {
                None => conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_raw))),
                Some(parse) => conversion_expr.append_all(
                    quote!(.and_then(|value| ::mogrify::util::mogrify_raw_with(#parse, value))),
                ),
            }
        } else {
            match specialization {
                MogrifyFieldSpecialization::None { .. } => match &attrs.parse {
                    None => {
                        conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_raw)));
                    }
                    Some(parse) => {
                        conversion_expr.append_all(quote!(.and_then(|value| ::mogrify::util::mogrify_raw_with(#parse, value))));
                    }
                },
                MogrifyFieldSpecialization::Option { .. } => match &attrs.parse {
                    None => {
                        conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_opt)));
                    }
                    Some(parse) => {
                        conversion_expr.append_all(quote!(.and_then(|value| ::mogrify::util::mogrify_opt_with(#parse, value))));
                    }
                },
                MogrifyFieldSpecialization::Vec { .. } => match &attrs.parse {
                    None => {
                        conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_vec)));
                    }
                    Some(parse) => {
                        conversion_expr.append_all(quote!(.and_then(|value| ::mogrify::util::mogrify_vec_with(#parse, value))));
                    }
                },
                MogrifyFieldSpecialization::Map { .. } => match &attrs.parse {
                    None => {
                        conversion_expr.append_all(quote!(.and_then(::mogrify::util::mogrify_map)));
                    }
                    Some(parse) => {
                        conversion_expr.append_all(quote!(.and_then(|value| ::mogrify::util::mogrify_map_with(#parse, value))));
                    }
                },
            }
        }
        if let Some(and_then) = &attrs.and_then {
            conversion_expr.append_all(quote!(.and_then(|r| #and_then(r).map_err(::mogrify::MogrificationError::wrapping))))
        }

        match (source_ident, field_count) {
            (Some(source_ident), _) => {
                let field = source_ident.to_string();
                conversion_expr.append_all(quote!(.at_field(#field)));
            }
            (None, 1) => {
                // skip tracking the index if there's only a single item in the tuple
            }
            (None, _) => {
                conversion_expr.append_all(quote!(.at_index(#idx)));
            }
        }

        conversion_expr
    }
}

fn type_shape_check(path: &TypePath, name: &'static str, generic_count: usize) -> bool {
    let last = path.path.segments.last().expect("paths can't be empty");
    path.qself.is_none()
        && last.ident == name
        && matches!(&last.arguments, PathArguments::AngleBracketed(args) if args.args.len() == generic_count)
}
fn extract_single_generic(path: TypePath) -> GenericArgument {
    match path
        .path
        .segments
        .into_iter()
        .last()
        .expect("expectation failed")
        .arguments
    {
        PathArguments::AngleBracketed(inner) => {
            inner.args.into_iter().next().expect("expectation failed")
        }
        _ => panic!("expectation failed"),
    }
}
fn extract_double_generic(path: TypePath) -> (GenericArgument, GenericArgument) {
    match path
        .path
        .segments
        .into_iter()
        .last()
        .expect("expectation failed")
        .arguments
    {
        PathArguments::AngleBracketed(inner) => {
            let mut iter = inner.args.into_iter();
            (
                iter.next().expect("expectation failed"),
                iter.next().expect("expectation failed"),
            )
        }
        _ => panic!("expectation failed"),
    }
}

impl TryFrom<(usize, Field)> for MogrifyFieldInfo {
    type Error = syn::Error;

    fn try_from((idx, value): (usize, Field)) -> Result<Self, Self::Error> {
        let attrs: MogrifyFieldAttrs = extract_mogrify_meta(value.attrs).try_into()?;
        let specialization = match value.ty {
            Type::Path(path) if type_shape_check(&path, "Option", 1) => {
                let ty = extract_single_generic(path);
                MogrifyFieldSpecialization::Option { ty }
            }
            Type::Path(path) if type_shape_check(&path, "Vec", 1) => {
                let ty = extract_single_generic(path);
                MogrifyFieldSpecialization::Vec { ty }
            }
            Type::Path(path) if type_shape_check(&path, "HashMap", 2) => {
                let (key_ty, value_ty) = extract_double_generic(path);
                MogrifyFieldSpecialization::Map { key_ty, value_ty }
            }
            _ => MogrifyFieldSpecialization::None { ty: value.ty },
        };
        let local_ident = Ident::new(&format!("local{idx}"), Span::mixed_site());
        Ok(MogrifyFieldInfo {
            idx,
            local_ident,
            source_ident: value.ident,
            attrs,
            specialization,
        })
    }
}
