#![recursion_limit = "256"]
#![cfg_attr(test, deny(warnings))]

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

use syn::export::{Span, ToTokens, TokenStream, TokenStream2};
use syn::punctuated::Pair;
use syn::{
    Attribute, Data, DataEnum, DeriveInput, Error, Field, Fields, Ident, Index, Lit, Member, Meta,
    NestedMeta, Path, Result, Type, TypeSlice,
};

struct Details<'a> {
    struct_name: &'a Ident,
    field_name: TokenStream2,
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
pub fn derive_asref(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    aserf_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Index, attributes(wrap, index_output))]
pub fn derive_index(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    index_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(LowerHex, attributes(wrap))]
pub fn derive_lowerhex(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    lowerhex_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(LowerHexIter, attributes(wrap))]
pub fn derive_lowerhex_iter(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    lowerhexiter_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Display, attributes(wrap, display_from))]
pub fn derive_display(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    display_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(From, attributes(wrap, derive_from))]
pub fn derive_from(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    from_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_derive(Error)]
pub fn derive_error(input: TokenStream) -> TokenStream {
    let derive_input = parse_macro_input!(input as DeriveInput);
    error_inner(derive_input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn error_inner(input: DeriveInput) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let type_name = &input.ident;
    let std = std();

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::error::Error for #type_name #ty_generics #where_clause {
            #[inline]
            fn description(&self) -> &str {
                "description() is deprecated; use Display"
            }
        }
    })
}

fn from_inner(input: DeriveInput) -> Result<TokenStream2> {
    match input.data {
        Data::Struct(_) => from_inner_struct(&input),
        Data::Enum(ref data) => from_inner_enum(&input, &data),
        Data::Union(_) => Err(Error::new_spanned(
            &input,
            "Deriving From is not supported in unions",
        )),
    }
}

fn from_inner_enum(input: &DeriveInput, data: &DataEnum) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let std = std();
    let mut res = TokenStream2::default();
    let enum_name = &input.ident;

    for variant in &data.variants {
        let variant_name = &variant.ident;
        for attr in &variant.attrs {
            let mv = find_meta_value(attr, "derive_from");
            if mv.found {
                if variant.fields.iter().len() > 1 {
                    return Err(Error::new_spanned(
                        &variant,
                        "Deriving From for a enum variant with multiple fields isn't supported",
                    ));
                }
                let field =
                    match variant.fields.iter().next() {
                        Some(field) => field,
                        None => return Err(Error::new_spanned(
                            &variant,
                            "Deriving From for a enum variant without any fields isn't supported",
                        )),
                    };
                let field_type = &field.ty;
                let ret_value = match field.ident {
                    Some(ref field_name) => quote! {#enum_name::#variant_name{#field_name: inner}},
                    None => quote! {#enum_name::#variant_name(inner)},
                };

                res = quote! {
                    #res
                    #[allow(unused_qualifications)]
                    impl #impl_generics #std::convert::From<#field_type> for #enum_name #ty_generics #where_clause {
                        #[inline]
                        fn from(inner: #field_type) -> Self {
                            #ret_value
                        }
                    }
                };
                break;
            }
        }
    }
    Ok(res)
}

fn from_inner_struct(input: &DeriveInput) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let field = get_field(&input, "From")?;
    let Details {
        struct_name,
        field_name,
        field_type,
        std,
        ..
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::convert::From<#field_type> for #struct_name #ty_generics #where_clause {
            #[inline]
            fn from(wrap: #field_type) -> Self {
                #struct_name {#field_name: wrap}
            }
        }
    })
}

fn display_inner(input: DeriveInput) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let struct_name = &input.ident;
    let std = std();
    let display_from = get_meta_value(
        &input.attrs,
        "Display",
        "display_from",
        Some("#[display_from(Debug)]`"),
    )?
    .expect("provided example, should always return a value if succeeded.");

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::fmt::Display for #struct_name #ty_generics #where_clause {
            #[inline]
            fn fmt(&self, f: &mut #std::fmt::Formatter) -> #std::fmt::Result {
                #std::fmt::#display_from::fmt(&self, f)
            }
        }
    })
}

fn lowerhexiter_inner(input: DeriveInput) -> Result<TokenStream2> {
    let field = get_field(&input, "LowerHexIter")?;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let Details {
        struct_name,
        field_name,
        std,
        ..
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::fmt::LowerHex<> for #struct_name #ty_generics #where_clause {
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

fn lowerhex_inner(input: DeriveInput) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let field = get_field(&input, "LowerHex")?;
    let Details {
        struct_name,
        field_name,
        std,
        ..
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::fmt::LowerHex for #struct_name #ty_generics #where_clause {
            #[inline]
            fn fmt(&self, f: &mut #std::fmt::Formatter) -> #std::fmt::Result {
                #std::fmt::LowerHex::fmt(&self.#field_name, f)
            }
        }
    })
}

#[allow(non_snake_case)]
fn generate_index_from_T(
    output: Option<TokenStream2>,
    T: TokenStream2,
    input: &DeriveInput,
    field: &Field,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let Details {
        struct_name,
        field_name,
        field_type,
        std,
    } = Details::from_input(&input.ident, field);
    let field_type = array_to_slice(field_type.clone());
    let output = output.unwrap_or_else(|| quote!(<#field_type as #std::ops::Index<#T>>::Output));
    quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::ops::Index<#T> for #struct_name #ty_generics #where_clause {
            type Output = #output;
            #[inline]
            fn index(&self, index: #T) -> &Self::Output {
                &self.#field_name[index]
            }
        }
    }
}

fn index_inner(input: DeriveInput) -> Result<TokenStream2> {
    let field = get_field(&input, "Index")?;
    let index_output = get_meta_value(&input.attrs, "Index", "index_output", None)?
        .map(ToTokens::into_token_stream);
    let std = std();

    let slice_output = index_output.clone().map(|index| quote!([#index]));

    let index_usize = generate_index_from_T(index_output, quote!(usize), &input, field);
    let index_range_usize = generate_index_from_T(
        slice_output.clone(),
        quote!(#std::ops::Range<usize>),
        &input,
        field,
    );
    let index_range_to_usize = generate_index_from_T(
        slice_output.clone(),
        quote!(#std::ops::RangeTo<usize>),
        &input,
        field,
    );
    let index_range_from_usize = generate_index_from_T(
        slice_output.clone(),
        quote!(#std::ops::RangeFrom<usize>),
        &input,
        field,
    );
    let index_range_full =
        generate_index_from_T(slice_output, quote!(#std::ops::RangeFull), &input, field);

    Ok(quote! {
        #index_usize
        #index_range_usize
        #index_range_to_usize
        #index_range_from_usize
        #index_range_full
    })
}

fn get_meta_value(
    attrs: &[Attribute],
    trait_name: &str,
    attribute_name: &str,
    example_if_required: Option<&str>,
) -> Result<Option<Member>> {
    let mut traits_found = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let mv = find_meta_value(attr, attribute_name);
        if mv.multiple {
            return Err(Error::new_spanned(
                attr,
                format!(
                    "derive_wrapper: {} doesn't nested attributes",
                    attribute_name
                ),
            ));
        }
        if mv.found {
            if let Some(trait_name) = mv.name {
                traits_found.push(trait_name);
            } else {
                return Err(Error::new_spanned(attr, format!("derive_wrapper: when using the {} attribute on the struct you must specify the trait you want to use to implement {}", attribute_name, trait_name)));
            }
        }
    }

    match traits_found.len() {
        1 => Ok(traits_found.pop()),
        0 => {
            if let Some(example) = example_if_required {
                Err(Error::new(Span::call_site(), format!("Deriving {} requires specifying which trait to use using the `{}` attribute. Try: `{}`", trait_name, attribute_name, example)))
            } else {
                Ok(None)
            }
        }
        _ => Err(Error::new(
            Span::call_site(),
            format!(
                "Deriving {} supports only a single {} attribute",
                trait_name, attribute_name
            ),
        )),
    }
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

fn aserf_inner(input: DeriveInput) -> Result<TokenStream2> {
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let field = get_field(&input, "AsRef")?;
    let Details {
        struct_name,
        field_name,
        field_type,
        std,
    } = Details::from_input(&input.ident, field);

    Ok(quote! {
        #[allow(unused_qualifications)]
        impl #impl_generics #std::convert::AsRef<#field_type> for #struct_name #ty_generics #where_clause {
            #[inline]
            fn as_ref(&self) -> &#field_type {
                &self.#field_name
            }
        }
    })
}

fn get_field<'a>(input: &'a DeriveInput, trait_name: &str) -> Result<&'a Field> {
    let fields = match input.data {
        Data::Struct(ref data) => &data.fields,
        _ => {
            return Err(Error::new_spanned(
                &input,
                format!("Deriving {} is supported only in structs", trait_name),
            ))
        }
    };

    if fields.iter().len() > 1 {
        let mut marked_fields = parse_outer_attributes(&input.attrs, &fields)?;
        marked_fields.extend(parse_field_attributes(&fields)?);
        match marked_fields.len() {
            1 => Ok(marked_fields.pop().unwrap()),
            0 => Err(Error::new_spanned(&input, format!("Deriving {} for a struct with multiple fields requires specifying a wrap attribute", trait_name))),
            _ => Err(Error::new_spanned(&input, format!("Deriving {} supports only a single wrap attribute", trait_name))),
        }
    } else {
        fields.iter().next().ok_or_else(|| {
            Error::new_spanned(
                &input,
                format!(
                    "Deriving {} for an empty struct isn't supported",
                    trait_name
                ),
            )
        })
    }
}

#[derive(Default)]
struct MetaValue {
    pub found: bool,
    pub name: Option<Member>,
    pub multiple: bool,
}

impl MetaValue {
    pub fn set_name_ident(&mut self, ident: Ident) {
        self.name = Some(Member::Named(ident));
    }

    pub fn set_name_index(&mut self, index: u32, span: Span) {
        self.name = Some(Member::Unnamed(Index { index, span }));
    }

    pub fn set_name_from_lit(&mut self, lit: Lit) {
        match lit {
            Lit::Str(l) => {
                if let Ok(index) = l.value().parse::<u32>() {
                    self.set_name_index(index, l.span());
                } else {
                    self.set_name_ident(Ident::new(&l.value(), l.span()));
                }
            }
            Lit::Int(int) => self.set_name_index(int.value() as u32, int.span()),
            _ => (),
        }
    }

    pub fn get_name(&self) -> Option<String> {
        self.name.as_ref().map(|name| match *name {
            Member::Unnamed(ref index) => index.index.to_string(),
            Member::Named(ref ident) => ident.to_string(),
        })
    }

    pub fn get_index(&self) -> Option<u32> {
        self.name.as_ref().and_then(|n| match *n {
            Member::Unnamed(ref i) => Some(i.index),
            Member::Named(_) => None,
        })
    }
}

fn find_meta_value(attr: &Attribute, name: &str) -> MetaValue {
    let mut res = MetaValue::default();
    if let Ok(meta) = attr.parse_meta() {
        if meta.name() == name {
            res.found = true;
            match meta {
                Meta::NameValue(nv) => res.set_name_from_lit(nv.lit),
                Meta::List(mut list) => {
                    res.multiple = list.nested.len() > 1;

                    if let Some(nestedmeta) = list.nested.pop().map(Pair::into_value) {
                        match nestedmeta {
                            NestedMeta::Literal(lit) => res.set_name_from_lit(lit),
                            NestedMeta::Meta(meta) => {
                                if let Meta::Word(ident) = meta {
                                    res.set_name_ident(ident)
                                }
                            }
                        }
                    }
                }
                Meta::Word(_) => (),
            }
        }
    }
    res
}

fn parse_outer_attributes<'a>(attrs: &[Attribute], fields: &'a Fields) -> Result<Vec<&'a Field>> {
    let mut res = Vec::with_capacity(attrs.len());
    for attr in attrs {
        let mv = find_meta_value(attr, "wrap");
        if mv.found {
            if let Some(index) = mv.get_index() {
                if let Some(field) = fields.iter().nth(index as usize) {
                    res.push(field);
                } else {
                    return Err(Error::new_spanned(&fields, format!("derive_wrapper: there's no field no. {} in the struct or it's not a tuple", index)));
                }
            } else if let Some(lit_name) = mv.get_name() {
                let mut found = false;
                for f in fields {
                    if let Some(ref field_name) = f.ident {
                        if field_name == &lit_name {
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
                    if let Some(lit) = mv.get_name() {
                        if ident != &lit {
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
