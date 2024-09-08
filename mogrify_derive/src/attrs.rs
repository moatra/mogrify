use proc_macro2::Ident;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{parse_quote, Attribute, Error, Expr, Path, Token, TypePath};

pub(crate) struct MogrifyFieldAttrs {
    // Option::ok()
    pub(crate) require: bool,
    // Replace TryFrom::try_from. If no TypePath specified, defaults to FromStr::from_str;
    pub(crate) parse: Option<TypePath>,
    // Disable Option/Vec/HashMap specialization
    pub(crate) raw: bool,
    // Option::unwrap_or()
    pub(crate) default: Option<Option<Expr>>,
    // Final check
    pub(crate) and_then: Option<TypePath>,
}

pub(crate) struct MogrifyStructAttrs {
    pub(crate) source: TypePath,
    // For enums, map any unit variants `Unit` to an empty tuple `Unit(())`
    pub(crate) grpc: bool,
}

pub(crate) struct MogrifyVariantAttrs {
    pub(crate) source: Option<Ident>,
}

pub(crate) fn extract_mogrify_meta(attrs: Vec<Attribute>) -> Vec<Attribute> {
    attrs
        .into_iter()
        .filter(|attr| attr.path().is_ident("mogrify"))
        .collect()
}

impl TryFrom<Vec<Attribute>> for MogrifyFieldAttrs {
    type Error = Error;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        if value.len() > 1 {
            return Err(Error::new(
                value[1].span(),
                "multiple #[mogrify()] attributes not supported",
            ));
        }
        if let Some(attr) = value.first() {
            let value = &attr.meta;
            let list = value.require_list()?;
            let mut require = false;
            let mut raw = false;
            let mut default = None;
            let mut and_then = None;
            let mut parse = None;
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("require") {
                    require = true;
                    return Ok(());
                }
                if meta.path.is_ident("raw") {
                    raw = true;
                    return Ok(());
                }
                if meta.path.is_ident("default") {
                    if meta.input.is_empty() {
                        default = Some(None);
                    } else {
                        let value = meta.value()?;
                        let expr: Expr = value.parse()?;
                        default = Some(Some(expr));
                    }
                    return Ok(());
                }
                if meta.path.is_ident("and_then") {
                    let value = meta.value()?;
                    let path: TypePath = value.parse()?;
                    and_then = Some(path);
                    return Ok(());
                }
                if meta.path.is_ident("parse") {
                    if meta.input.is_empty() {
                        parse = Some(parse_quote!(::mogrify::util::force_parse))
                    } else {
                        let value = meta.value()?;
                        let path: TypePath = value.parse()?;
                        parse = Some(path);
                    }
                    return Ok(());
                }
                Err(meta.error(
                    r#"expected either "require", "raw", "parse=...", "default = ...", or "and_then = ...""#,
                ))
            })?;
            Ok(MogrifyFieldAttrs {
                require,
                raw,
                parse,
                default,
                and_then,
            })
        } else {
            Ok(MogrifyFieldAttrs {
                raw: false,
                require: false,
                parse: None,
                default: None,
                and_then: None,
            })
        }
    }
}

impl TryFrom<Attribute> for MogrifyStructAttrs {
    type Error = Error;

    fn try_from(value: Attribute) -> Result<Self, Self::Error> {
        value.parse_args_with(|input: ParseStream| {
            // Source type is first argument, always
            let source: TypePath = input.parse()?;
            let mut grpc = false;

            // from here on out, we're basically emulating `syn::meta::ParsedNestedMeta`, but without the "accept keywords in the path" logic
            // because for some reason `parse_meta_path` is not a public function.
            while !input.is_empty() {
                input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
                let path = input.parse::<Path>()?;

                if path.is_ident("grpc") {
                    grpc = true;
                } else {
                    return Err(Error::new_spanned(path, "unrecognized argument"));
                }
            }

            Ok(MogrifyStructAttrs { source, grpc })
        })
    }
}

impl TryFrom<Vec<Attribute>> for MogrifyVariantAttrs {
    type Error = Error;

    fn try_from(value: Vec<Attribute>) -> Result<Self, Self::Error> {
        if value.len() > 1 {
            return Err(Error::new(
                value[1].span(),
                "multiple #[mogrify()] attributes not supported",
            ));
        }
        if let Some(attr) = value.first() {
            let value = &attr.meta;
            let list = value.require_list()?;
            let mut source = None;
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("source") {
                    let value = meta.value()?;
                    let ident: Ident = value.parse()?;
                    source = Some(ident);
                    return Ok(());
                }
                Err(meta.error(r#"expected "source=...""#))
            })?;
            Ok(MogrifyVariantAttrs { source })
        } else {
            Ok(MogrifyVariantAttrs { source: None })
        }
    }
}
