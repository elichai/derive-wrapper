#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::*;
use syn::export::ToTokens;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, Lit, LitStr, Meta, Path, Result, Type,
};

struct Details<'a> {
    struct_name: &'a Ident,
    field_name: TokenStream,
    field_type: &'a Type,
    std: Path,
}

impl<'a> Details<'a> {
    pub fn from_input(struct_name: &'a Ident, field: &'a Field) -> Self {
        let field_name = field
            .ident
            .as_ref()
            .map_or_else(|| quote!(0), ToTokens::into_token_stream);

        Details {
            struct_name,
            field_name,
            field_type: &field.ty,
            std: std(),
        }
    }
}

#[proc_macro_derive(AsRef, attributes(wrap))]
pub fn derive_asref(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    aserf_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn aserf_inner(input: DeriveInput) -> Result<TokenStream> {
    let field = get_field(&input)?;
    let Details {
        struct_name,
        field_name,
        field_type,
        std,
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[automatically_derived]
        #[allow(unused_qualifications)]
        impl #std::convert::AsRef<#field_type> for #struct_name {
            #[inline]
            fn as_ref(&self) -> &#field_type {
                &self.#field_name
            }
        }
    })
}

fn get_field(input: &DeriveInput) -> Result<&Field> {
    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(Error::new_spanned(
                &input,
                "Deriving AsRef is supported only in structs",
            ))
        }
    };

    if fields.iter().len() > 1 {
        let mut marked_fields = parse_outer_attributes(&input.attrs, &fields)?;
        marked_fields.extend(parse_field_attributes(&fields)?);
        match marked_fields.len() {
            1 => Ok(marked_fields.pop().unwrap()),
            0 => Err(Error::new_spanned(&input, "Deriving AsRef for a struct with multiple fields requires specifying a wrap attribute")),
            _ => Err(Error::new_spanned(&input, "Deriving AsRef supports only a single wrap attribute")),
        }
    } else {
        fields.iter().next().ok_or_else(|| {
            Error::new_spanned(&input, "Deriving AsRef for an empty struct isn't supported")
        })
    }
}

fn is_wrap(attr: &Attribute) -> (bool, Option<LitStr>) {
    let mut found = false;
    let mut lit = None;
    if let Ok(meta) = attr.parse_meta() {
        if meta.name() == "wrap" {
            found = true;
            if let Meta::NameValue(nv) = meta {
                if let Lit::Str(l) = nv.lit {
                    lit = Some(l);
                }
            }
        }
    }
    (found, lit)
}

fn parse_outer_attributes<'a>(attrs: &[Attribute], fields: &'a Fields) -> Result<Vec<&'a Field>> {
    let mut res = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let (wrap, lit) = is_wrap(attr);
        if wrap {
            if let Some(lit_name) = lit {
                let mut found = false;
                for f in fields {
                    if let Some(ref field_name) = f.ident {
                        if lit_name.value() == field_name.to_string() {
                            res.push(f);
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return Err(Error::new_spanned(
                        &fields,
                        format!("derive_wrapper: field {} doesn't exist", lit_name.value()),
                    ));
                }
            } else {
                return Err(Error::new_spanned(&fields, "derive_wrapper: when using the wrap attribute on the struct you must specify the field name"));
            }
        }
    }
    Ok(res)
}

fn parse_field_attributes(fields: &Fields) -> Result<Vec<&Field>> {
    let mut res = Vec::with_capacity(fields.iter().len());
    for field in fields.iter() {
        for attr in &field.attrs {
            let (wrap, lit) = is_wrap(attr);
            if wrap {
                if let Some(ref ident) = field.ident {
                    let ident = ident.to_string();
                    if let Some(lit) = lit {
                        let lit = lit.value();
                        if lit != ident {
                            return Err(Error::new_spanned(&field, format!("derive_wrapper: The provided field name doesn't match the field name it's above: `{} != {}`", lit, ident)));
                        }
                    }
                    res.push(field)
                } else {
                    return Err(Error::new_spanned(&field, "derive_wrapper doesn't yet support attributes on unnamed fields (Please file an issue)"));
                }
            }
        }
    }
    Ok(res)
}

fn std() -> Path {
    #[cfg(feature = "std")]
        return parse_quote!(::std);
    #[cfg(not(feature = "std"))]
        return parse_quote!(::core);
}
