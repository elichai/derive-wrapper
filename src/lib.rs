#![recursion_limit = "256"]

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate proc_macro;
extern crate proc_macro2;

use proc_macro2::*;
use syn::export::ToTokens;
use syn::punctuated::Pair;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, Lit, Meta, NestedMeta, Path, Result, Type,
    TypeSlice,
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

#[proc_macro_derive(Index, attributes(wrap))]
pub fn derive_index(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    index_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(LowerHex, attributes(wrap))]
pub fn derive_lowerhex(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    lowerhex_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(LowerHexIter, attributes(wrap))]
pub fn derive_lowerhex_iter(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    lowerhexiter_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Display, attributes(wrap, display_from))]
pub fn derive_display(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    display_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn display_inner(input: DeriveInput) -> Result<TokenStream> {
    let field = get_field(&input)?;
    let Details {
        struct_name, std, ..
    } = Details::from_input(&input.ident, field);

    let mut traits_found = Vec::with_capacity(input.attrs.len());
    for attr in &input.attrs {
        let mv = find_meta_value(attr, "display_from");
        if mv.multiple {
            return Err(Error::new_spanned(attr, "derive_wrapper: display_from doesn't nested attributes"));
        }
        if mv.found {
            if let Some(trait_name) = mv.name {
                traits_found.push(trait_name);
            } else {
                return Err(Error::new_spanned(attr, "derive_wrapper: when using the display_from attribute on the struct you must specify the trait you want to use to implement Display"));
            }
        }
    }

    let display_from = match traits_found.len() {
        1 => traits_found.pop().unwrap(),
        0 => return Err(Error::new_spanned(&input, "Deriving Display requires specifying which trait to use using the `display_from` attribute. Try: `#[display_from(Debug)]`")),
        _ => return Err(Error::new_spanned(&input, "Deriving Display supports only a single display_from attribute")),
    };

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #std::fmt::Display for #struct_name {
            #[inline]
            fn fmt(&self, f: &mut #std::fmt::Formatter) -> #std::fmt::Result {
                #std::fmt::#display_from::fmt(&self, f)
            }
        }
    })
}

fn lowerhexiter_inner(input: DeriveInput) -> Result<TokenStream> {
    let field = get_field(&input)?;
    let Details {
        struct_name,
        field_name,
        std,
        ..
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #std::fmt::LowerHex<> for #struct_name {
            #[inline]
            fn fmt(&self, f: &mut #std::fmt::Formatter) -> #std::fmt::Result {
                for ch in self.#field_name.iter() {
                    #std::fmt::LowerHex::fmt(&ch, f)?;
                }
                #std::result::Result::Ok(())
            }
        }
    })
}

fn lowerhex_inner(input: DeriveInput) -> Result<TokenStream> {
    let field = get_field(&input)?;
    let Details {
        struct_name,
        field_name,
        std,
        ..
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #std::fmt::LowerHex for #struct_name {
            #[inline]
            fn fmt(&self, f: &mut #std::fmt::Formatter) -> #std::fmt::Result {
                #std::fmt::LowerHex::fmt(&self.#field_name, f)
            }
        }
    })
}

fn index_inner(input: DeriveInput) -> Result<TokenStream> {
    let field = get_field(&input)?;
    let Details {
        struct_name,
        field_name,
        field_type,
        std,
    } = Details::from_input(&input.ident, field);
    let field_type = array_to_slice(field_type.clone());

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #std::ops::Index<usize> for #struct_name {
            type Output = <#field_type as #std::ops::Index<usize>>::Output;
            #[inline]
            fn index(&self, index: usize) -> &Self::Output {
                &self.#field_name[index]
            }
        }

        #[allow(unused_qualifications)]
        impl #std::ops::Index<#std::ops::Range<usize>> for #struct_name {
            type Output = <#field_type as #std::ops::Index<#std::ops::Range<usize>>>::Output;

            #[inline]
            fn index(&self, index: #std::ops::Range<usize>) -> &Self::Output {
                &self.#field_name[index]
            }
        }

        #[allow(unused_qualifications)]
        impl #std::ops::Index<#std::ops::RangeTo<usize>> for #struct_name {
            type Output = <#field_type as #std::ops::Index<#std::ops::RangeTo<usize>>>::Output;

            #[inline]
            fn index(&self, index: #std::ops::RangeTo<usize>) -> &Self::Output {
                &self.#field_name[index]
            }
        }

        #[allow(unused_qualifications)]
        impl #std::ops::Index<#std::ops::RangeFrom<usize>> for #struct_name {
            type Output = <#field_type as #std::ops::Index<#std::ops::RangeFrom<usize>>>::Output;

            #[inline]
            fn index(&self, index: #std::ops::RangeFrom<usize>) -> &Self::Output {
                &self.#field_name[index]
            }
        }

        #[allow(unused_qualifications)]
        impl #std::ops::Index<#std::ops::RangeFull> for #struct_name {
            type Output = <#field_type as #std::ops::Index<#std::ops::RangeFull>>::Output;

            #[inline]
            fn index(&self, index: #std::ops::RangeFull) -> &Self::Output {
                &self.#field_name[index]
            }
        }
    })
}

fn array_to_slice(ty: Type) -> Type {
    if let Type::Array(arr) = ty {
        Type::Slice(TypeSlice {
            bracket_token: arr.bracket_token,
            elem: arr.elem,
        })
    } else {
        ty
    }
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

#[derive(Default)]
struct MetaValue {
    pub found: bool,
    pub name: Option<Ident>,
    pub multiple: bool,
}

fn find_meta_value(attr: &Attribute, name: &str) -> MetaValue {
    let mut res = MetaValue::default();
    if let Ok(meta) = attr.parse_meta() {
        if meta.name() == name {
            res.found = true;
            match meta {
                Meta::NameValue(nv) => res.name = lit_to_ident(nv.lit),
                Meta::List(mut list) => {
                    res.multiple = list.nested.len() > 1;
                    res.name = list
                        .nested
                        .pop()
                        .map(Pair::into_value)
                        .and_then(|nestedmeta| match nestedmeta {
                            NestedMeta::Literal(lit) => lit_to_ident(lit),
                            NestedMeta::Meta(meta) => {
                                if let Meta::Word(ident) = meta {
                                    Some(ident)
                                } else {
                                    None
                                }
                            }
                        });
                }
                Meta::Word(_) => (),
            }
        }
    }
    res
}

fn lit_to_ident(lit: Lit) -> Option<Ident> {
    if let Lit::Str(l) = lit {
        Some(Ident::new(&l.value(), l.span()))
    } else {
        None
    }
}

fn parse_outer_attributes<'a>(attrs: &[Attribute], fields: &'a Fields) -> Result<Vec<&'a Field>> {
    let mut res = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let mv = find_meta_value(attr, "wrap");
        if mv.found {
            if let Some(lit_name) = mv.name {
                let mut found = false;
                for f in fields {
                    if let Some(ref field_name) = f.ident {
                        if lit_name == field_name.to_string() {
                            res.push(f);
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return Err(Error::new_spanned(
                        &fields,
                        format!("derive_wrapper: field {} doesn't exist", lit_name),
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
            let mv = find_meta_value(attr, "wrap");
            if mv.found {
                if let Some(ref ident) = field.ident {
                    let ident = ident.to_string();
                    if let Some(lit) = mv.name {
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

#[inline(always)]
fn std() -> Path {
    #[cfg(feature = "std")]
    return parse_quote!(::std);
    #[cfg(not(feature = "std"))]
    return parse_quote!(::core);
}
