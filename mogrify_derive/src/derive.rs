use crate::attrs::{extract_mogrify_meta, MogrifyStructAttrs, MogrifyVariantAttrs};
use crate::fields::MogrifyFieldInfo;
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error, Fields, GenericArgument, PathArguments, Type,
    TypePath,
};

pub(crate) fn derive_inner(input: DeriveInput) -> Result<TokenStream, Error> {
    let ident = input.ident;
    let ident_span = ident.span();
    let sources: Vec<MogrifyStructAttrs> = extract_mogrify_meta(input.attrs)
        .into_iter()
        .map(|attr| attr.try_into())
        .collect::<Result<_, _>>()?;

    if sources.is_empty() {
        return Err(Error::new(
            Span::call_site(),
            "Mogrify expected at least one top level #[mogrify()] attribute",
        ));
    }

    match input.data {
        Data::Struct(data) => derive_struct(ident, sources, data),
        Data::Enum(data) => derive_enum(ident, sources, data),
        Data::Union(_) => Err(Error::new(ident_span, "Mogrify does not support unions")),
    }
}

fn turbofish_match_pattern(type_path: &TypePath) -> TokenStream {
    let path = &type_path.path;
    let segments = &path.segments;

    let mut tokens = TokenStream::new();
    for (i, segment) in segments.iter().enumerate() {
        if i > 0 {
            tokens.extend(quote! {::});
        }
        let ident = &segment.ident;
        tokens.extend(quote! {#ident});

        if let PathArguments::AngleBracketed(ref args) = segment.arguments {
            let args_tokens: Vec<TokenStream> = args
                .args
                .iter()
                .map(|arg| match arg {
                    GenericArgument::Lifetime(lifetime) => {
                        quote! {#lifetime}
                    }
                    GenericArgument::Type(Type::Path(type_path)) => {
                        let nested_tokens = turbofish_match_pattern(type_path);
                        quote! {#nested_tokens}
                    }
                    GenericArgument::Const(constant) => {
                        quote! {#constant}
                    }
                    GenericArgument::AssocType(assoc) => {
                        let ident = &assoc.ident;
                        let ty = &assoc.ty;
                        quote! {#ident = #ty}
                    }
                    GenericArgument::AssocConst(assoc) => {
                        let ident = &assoc.ident;
                        let ty = &assoc.value;
                        quote! {#ident = #ty}
                    }
                    GenericArgument::Constraint(constraint) => {
                        let ident = &constraint.ident;
                        let bounds = &constraint.bounds;
                        quote! {#ident: #bounds}
                    }
                    _ => panic!("Unexpected generic argument"),
                })
                .collect();

            tokens.extend(quote! {::<#(#args_tokens),*>});
        }
    }
    tokens
}

pub(crate) fn derive_struct(
    ident: Ident,
    sources: Vec<MogrifyStructAttrs>,
    data: DataStruct,
) -> Result<TokenStream, Error> {
    let fields = data
        .fields
        .into_iter()
        .enumerate()
        .map(|f| f.try_into())
        .collect::<Result<Vec<MogrifyFieldInfo>, _>>()?;

    let destructure_instr = fields
        .iter()
        .map(|field| field.destructure_expr())
        .collect::<Vec<_>>();

    let capture_instr = fields
        .iter()
        .map(|field| {
            let local_ident = &field.local_ident;
            let mogrify = field.conversion(fields.len());
            quote!(let #local_ident = ::mogrify::util::capture_error(&mut errors, #mogrify);)
        })
        .collect::<Vec<_>>();

    let assign_instr = fields
        .iter()
        .map(|field| field.assignment_expr())
        .collect::<Vec<_>>();

    let mut tokens = TokenStream::new();

    for MogrifyStructAttrs { source, grpc: _ } in sources {
        let match_expr = turbofish_match_pattern(&source);
        tokens.extend(quote! {
            impl TryFrom<#source> for #ident {
                type Error = ::mogrify::MogrificationError;

                fn try_from(from: #source) -> Result<Self, Self::Error> {
                    use ::mogrify::Pathed;
                    let mut errors = ::std::vec::Vec::new();

                    let #match_expr { #(#destructure_instr),* } = from;

                    #(#capture_instr)*

                    ::mogrify::MogrificationError::condense(errors)?;
                    Ok(Self {
                        #(#assign_instr),*
                    })
                }
            }
        });
    }
    Ok(tokens)
}

fn build_match_instr(
    source: MogrifyStructAttrs,
    data: DataEnum,
) -> Result<Vec<TokenStream>, Error> {
    let mut variant_matches = Vec::<TokenStream>::new();

    for variant in data.variants {
        let variant_attrs: MogrifyVariantAttrs = extract_mogrify_meta(variant.attrs).try_into()?;
        let source_name = variant_attrs
            .source
            .unwrap_or_else(|| variant.ident.clone());
        let variant_name = &variant.ident;
        match variant.fields {
            Fields::Named(fields) => {
                let fields = fields
                    .named
                    .into_iter()
                    .enumerate()
                    .map(|f| f.try_into())
                    .collect::<Result<Vec<MogrifyFieldInfo>, _>>()?;
                let destructure_instr = fields
                    .iter()
                    .map(|field| field.destructure_expr())
                    .collect::<Vec<_>>();

                let capture_instr = fields
                    .iter()
                    .map(|field| {
                        let local_ident = &field.local_ident;
                        let mogrify = field.conversion(fields.len());
                        quote!(let #local_ident = ::mogrify::util::capture_error(&mut errors, #mogrify);)
                    })
                    .collect::<Vec<_>>();

                let assign_instr = fields
                    .iter()
                    .map(|field| field.assignment_expr())
                    .collect::<Vec<_>>();

                let source_name_string = source_name.to_string();
                variant_matches.push(quote! {
                    #source_name { #(#destructure_instr),* } => {
                        #(#capture_instr)*
                        ::mogrify::MogrificationError::condense(errors).at_field(#source_name_string)?;
                        Self::#variant_name {
                            #(#assign_instr),*
                        }
                    }
                })
            }
            Fields::Unnamed(fields) => {
                let fields = fields
                    .unnamed
                    .into_iter()
                    .enumerate()
                    .map(|f| f.try_into())
                    .collect::<Result<Vec<MogrifyFieldInfo>, _>>()?;

                let destructure_instr = fields
                    .iter()
                    .map(|field| field.destructure_expr())
                    .collect::<Vec<_>>();

                let capture_instr = fields
                    .iter()
                    .map(|field| {
                        let local_ident = &field.local_ident;
                        let mogrify = field.conversion(fields.len());
                        quote!(let #local_ident = ::mogrify::util::capture_error(&mut errors, #mogrify);)
                    })
                    .collect::<Vec<_>>();

                let assign_instr = fields
                    .iter()
                    .map(|field| field.assignment_expr())
                    .collect::<Vec<_>>();

                variant_matches.push(quote! {
                    #source_name ( #(#destructure_instr),* ) => {
                        #(#capture_instr)*
                        ::mogrify::MogrificationError::condense(errors)?;
                        Self::#variant_name (
                            #(#assign_instr),*
                        )
                    }
                })
            }
            Fields::Unit => {
                // can use source_name directly because we'll bring all variants in scope in the try_from body
                if source.grpc {
                    variant_matches.push(quote! {
                        #source_name(()) => Self::#variant_name
                    })
                } else {
                    variant_matches.push(quote! {
                        #source_name => Self::#variant_name
                    })
                }
            }
        }
    }
    Ok(variant_matches)
}

pub(crate) fn derive_enum(
    ident: Ident,
    attrs: Vec<MogrifyStructAttrs>,
    data: DataEnum,
) -> Result<TokenStream, Error> {
    let mut tokens = TokenStream::new();

    for attr in attrs {
        let match_expr = turbofish_match_pattern(&attr.source);
        let source = attr.source.clone();
        let variant_matches = build_match_instr(attr, data.clone())?;
        tokens.extend(quote! {
            impl TryFrom<#source> for #ident {
                type Error = ::mogrify::MogrificationError;

                fn try_from(from: #source) -> Result<Self, Self::Error> {
                    use ::mogrify::Pathed;
                    let mut errors = ::std::vec::Vec::new();

                    Ok(match from {
                        #(#match_expr :: #variant_matches),*
                    })
                }
            }
        });
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example() {
        let good_input = r#"#[derive(Mogrify)]
#[mogrify(RawFoo)]
pub struct Foo {
    bar: bool,
    #[mogrify(default = 32)]
    baz: i64,
    #[mogrify(require)]
    fizz: OtherStruct,
    buzz: Vec<RepeatedStruct>,
    #[mogrify(raw)]
    data: VecLikeStruct
}"#;

        let parsed = syn::parse_str(good_input).unwrap();
        let receiver = derive_inner(parsed).unwrap();

        println!(
            r#"
INPUT:

{}

OUTPUT:

{}
"#,
            good_input, receiver,
        );
    }
}
