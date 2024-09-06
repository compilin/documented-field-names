mod config;
pub(crate) mod util;

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DataEnum, DataStruct, DataUnion,
    DeriveInput, Error, Fields, Ident, Item,
};

#[cfg(feature = "customise")]
use crate::config::{attr::AttrCustomisations, derive::get_customisations_from_attrs};
use crate::{
    config::{attr::AttrConfig, derive::DeriveConfig},
    util::{crate_module_path, get_docs, get_vis_name_attrs},
};

/// Derive proc-macro for `Documented` trait.
///
/// # Example
///
/// ```rust
/// use documented::Documented;
///
/// /// Nice.
/// /// Multiple single-line doc comments are supported.
/// ///
/// /** Multi-line doc comments are supported too.
///     Each line of the multi-line block is individually trimmed by default.
///     Note the lack of spaces in front of this line.
/// */
/// #[doc = "Attribute-style documentation is supported too."]
/// #[derive(Documented)]
/// struct BornIn69;
///
/// let doc_str = "Nice.
/// Multiple single-line doc comments are supported.
///
/// Multi-line doc comments are supported too.
/// Each line of the multi-line block is individually trimmed by default.
/// Note the lack of spaces in front of this line.
///
/// Attribute-style documentation is supported too.";
/// assert_eq!(BornIn69::DOCS, doc_str);
/// ```
///
/// # Configuration
///
/// With the `customise` feature enabled, you can customise this macro's
/// behaviour using the `#[documented(...)]` attribute.
///
/// Currently, you can disable line-trimming like so:
///
/// ```rust
/// # use documented::Documented;
/// ///     Terrible.
/// #[derive(Documented)]
/// #[documented(trim = false)]
/// struct Frankly;
///
/// assert_eq!(Frankly::DOCS, "     Terrible.");
/// ```
///
/// If there are other configuration options you wish to have, please submit an
/// issue or a PR.
#[cfg_attr(not(feature = "customise"), proc_macro_derive(Documented))]
#[cfg_attr(
    feature = "customise",
    proc_macro_derive(Documented, attributes(documented))
)]
pub fn documented(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    #[cfg(not(feature = "customise"))]
    let config = DeriveConfig::default();
    #[cfg(feature = "customise")]
    let config = match get_customisations_from_attrs(&input.attrs, "documented") {
        Ok(Some(customisations)) => DeriveConfig::default().with_customisations(customisations),
        Ok(None) => DeriveConfig::default(),
        Err(err) => return err.into_compile_error().into(),
    };

    let docs = match get_docs(&input.attrs, config.trim) {
        Ok(Some(doc)) => doc,
        Ok(None) => {
            return Error::new(input.ident.span(), "Missing doc comments")
                .into_compile_error()
                .into()
        }
        Err(e) => return e.into_compile_error().into(),
    };

    quote! {
        #[automatically_derived]
        impl #impl_generics documented::Documented for #ident #ty_generics #where_clause {
            const DOCS: &'static str = #docs;
        }
    }
    .into()
}

/// Derive proc-macro for `DocumentedFields` trait.
///
/// # Example
///
/// ```rust
/// use documented::DocumentedFields;
///
/// #[derive(DocumentedFields)]
/// struct BornIn69 {
///     /// Cry like a grandmaster.
///     rawr: String,
///     explosive: usize,
/// };
///
/// assert_eq!(BornIn69::FIELD_DOCS, [Some("Cry like a grandmaster."), None]);
/// ```
///
/// You can also use [`get_field_docs`](Self::get_field_docs) to access the
/// fields' documentation using their names.
///
/// ```rust
/// # use documented::{DocumentedFields, Error};
/// #
/// # #[derive(DocumentedFields)]
/// # struct BornIn69 {
/// #     /// Cry like a grandmaster.
/// #     rawr: String,
/// #     explosive: usize,
/// # };
/// #
/// assert_eq!(BornIn69::get_field_docs("rawr"), Ok("Cry like a grandmaster."));
/// assert_eq!(
///     BornIn69::get_field_docs("explosive"),
///     Err(Error::NoDocComments("explosive".to_string()))
/// );
/// assert_eq!(
///     BornIn69::get_field_docs("gotcha"),
///     Err(Error::NoSuchField("gotcha".to_string()))
/// );
/// ```
///
/// # Configuration
///
/// With the `customise` feature enabled, you can customise this macro's
/// behaviour using the `#[documented_fields(...)]` attribute. Note that this
/// attribute works on both the container and each individual field, with the
/// per-field configurations overriding container configurations, which
/// override the default.
///
/// Currently, you can (selectively) disable line-trimming like so:
///
/// ```rust
/// # use documented::DocumentedFields;
/// #[derive(DocumentedFields)]
/// #[documented_fields(trim = false)]
/// struct Frankly {
///     ///     Delicious.
///     perrier: usize,
///     ///     I'm vegan.
///     #[documented_fields(trim = true)]
///     fried_liver: bool,
/// }
///
/// assert_eq!(Frankly::FIELD_DOCS, [Some("     Delicious."), Some("I'm vegan.")]);
/// ```
///
/// If there are other configuration options you wish to have, please
/// submit an issue or a PR.
#[cfg_attr(not(feature = "customise"), proc_macro_derive(DocumentedFields))]
#[cfg_attr(
    feature = "customise",
    proc_macro_derive(DocumentedFields, attributes(documented_fields))
)]
pub fn documented_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // `#[documented_fields(...)]` on container type
    #[cfg(not(feature = "customise"))]
    let base_config = DeriveConfig::default();
    #[cfg(feature = "customise")]
    let base_config = match get_customisations_from_attrs(&input.attrs, "documented_fields") {
        Ok(Some(customisations)) => DeriveConfig::default().with_customisations(customisations),
        Ok(None) => DeriveConfig::default(),
        Err(err) => return err.into_compile_error().into(),
    };

    let (field_idents, field_docs): (Vec<_>, Vec<_>) = {
        let fields_attrs: Vec<(Option<Ident>, Vec<Attribute>)> = match input.data.clone() {
            Data::Enum(DataEnum { variants, .. }) => variants
                .into_iter()
                .map(|v| (Some(v.ident), v.attrs))
                .collect(),
            Data::Struct(DataStruct { fields, .. }) => {
                fields.into_iter().map(|f| (f.ident, f.attrs)).collect()
            }
            Data::Union(DataUnion { fields, .. }) => fields
                .named
                .into_iter()
                .map(|f| (f.ident, f.attrs))
                .collect(),
        };

        match fields_attrs
            .into_iter()
            .map(|(i, attrs)| {
                #[cfg(not(feature = "customise"))]
                let config = base_config;
                #[cfg(feature = "customise")]
                let config =
                    if let Some(c) = get_customisations_from_attrs(&attrs, "documented_fields")? {
                        base_config.with_customisations(c)
                    } else {
                        base_config
                    };
                get_docs(&attrs, config.trim).map(|d| (i, d))
            })
            .collect::<syn::Result<Vec<_>>>()
        {
            Ok(t) => t.into_iter().unzip(),
            Err(e) => return e.into_compile_error().into(),
        }
    };

    // quote macro needs some help with `Option`s
    // see: https://github.com/dtolnay/quote/issues/213
    let field_docs_tokenised: Vec<_> = field_docs
        .into_iter()
        .map(|opt| match opt {
            Some(c) => quote! { Some(#c) },
            None => quote! { None },
        })
        .collect();

    let phf_match_arms: Vec<_> = field_idents
        .into_iter()
        .enumerate()
        .filter_map(|(i, o)| o.map(|ident| (i, ident.to_string())))
        .map(|(i, ident)| quote! { #ident => #i, })
        .collect();

    let documented_module_path = crate_module_path();

    quote! {
        #[automatically_derived]
        impl #impl_generics documented::DocumentedFields for #ident #ty_generics #where_clause {
            const FIELD_DOCS: &'static [Option<&'static str>] = &[#(#field_docs_tokenised),*];

            fn __documented_get_index<__Documented_T: AsRef<str>>(field_name: __Documented_T) -> Option<usize> {
                use #documented_module_path::_private_phf_reexport_for_macro as phf;

                static PHF: phf::Map<&'static str, usize> = phf::phf_map! {
                    #(#phf_match_arms)*
                };
                PHF.get(field_name.as_ref()).copied()
            }
        }
    }
    .into()
}

/// Derive proc-macro for `DocumentedVariants` trait.
///
/// # Example
///
/// ```rust
/// use documented::{DocumentedVariants, Error};
///
/// #[derive(DocumentedVariants)]
/// enum NeverPlay {
///     F3,
///     /// I fell out of my chair.
///     F6,
/// }
///
/// assert_eq!(
///     NeverPlay::F3.get_variant_docs(),
///     Err(Error::NoDocComments("F3".into()))
/// );
/// assert_eq!(
///     NeverPlay::F6.get_variant_docs(),
///     Ok("I fell out of my chair.")
/// );
/// ```
///
/// # Configuration
///
/// With the `customise` feature enabled, you can customise this macro's
/// behaviour using the `#[documented_variants(...)]` attribute. Note that this
/// attribute works on both the container and each individual variant, with the
/// per-variant configurations overriding container configurations, which
/// override the default.
///
/// Currently, you can (selectively) disable line-trimming like so:
///
/// ```rust
/// # use documented::DocumentedVariants;
/// #[derive(DocumentedVariants)]
/// #[documented_variants(trim = false)]
/// enum Always {
///     ///     Or the quality.
///     SacTheExchange,
///     ///     Like a Frenchman.
///     #[documented_variants(trim = true)]
///     Retreat,
/// }
/// assert_eq!(
///     Always::SacTheExchange.get_variant_docs(),
///     Ok("     Or the quality.")
/// );
/// assert_eq!(Always::Retreat.get_variant_docs(), Ok("Like a Frenchman."));
/// ```
///
/// If there are other configuration options you wish to have, please
/// submit an issue or a PR.
#[cfg_attr(not(feature = "customise"), proc_macro_derive(DocumentedVariants))]
#[cfg_attr(
    feature = "customise",
    proc_macro_derive(DocumentedVariants, attributes(documented_variants))
)]
pub fn documented_variants(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // `#[documented_variants(...)]` on container type
    #[cfg(not(feature = "customise"))]
    let base_config = DeriveConfig::default();
    #[cfg(feature = "customise")]
    let base_config = match get_customisations_from_attrs(&input.attrs, "documented_variants") {
        Ok(Some(customisations)) => DeriveConfig::default().with_customisations(customisations),
        Ok(None) => DeriveConfig::default(),
        Err(err) => return err.into_compile_error().into(),
    };

    let variants_docs = {
        let Data::Enum(DataEnum { variants, .. }) = input.data else {
            return Error::new(
                input.span(), // this targets the `struct`/`union` keyword
                "DocumentedVariants can only be used on enums.\n\
                For structs and unions, use DocumentedFields instead.",
            )
            .into_compile_error()
            .into();
        };
        match variants
            .into_iter()
            .map(|v| (v.ident, v.fields, v.attrs))
            .map(|(i, f, attrs)| {
                #[cfg(not(feature = "customise"))]
                let config = base_config;
                #[cfg(feature = "customise")]
                let config = if let Some(c) =
                    get_customisations_from_attrs(&attrs, "documented_variants")?
                {
                    base_config.with_customisations(c)
                } else {
                    base_config
                };
                get_docs(&attrs, config.trim).map(|d| (i, f, d))
            })
            .collect::<syn::Result<Vec<_>>>()
        {
            Ok(t) => t,
            Err(e) => return e.into_compile_error().into(),
        }
    };

    let match_arms: Vec<_> = variants_docs
        .into_iter()
        .map(|(ident, fields, docs)| {
            let pat = match fields {
                Fields::Unit => quote! { Self::#ident },
                Fields::Unnamed(_) => quote! { Self::#ident(..) },
                Fields::Named(_) => quote! { Self::#ident{..} },
            };
            match docs {
                Some(docs_str) => quote! { #pat => Ok(#docs_str), },
                None => {
                    let ident_str = ident.to_string();
                    quote! { #pat => Err(documented::Error::NoDocComments(#ident_str.into())), }
                }
            }
        })
        .collect();

    // IDEA: I'd like to use phf here, but it doesn't seem to be possible at the moment,
    // because there isn't a way to get an enum's discriminant at compile time
    // if this becomes possible in the future, or alternatively you have a good workaround,
    // improvement suggestions are more than welcomed
    quote! {
        #[automatically_derived]
        impl #impl_generics documented::DocumentedVariants for #ident #ty_generics #where_clause {
            fn get_variant_docs(&self) -> Result<&'static str, documented::Error> {
                match self {
                    #(#match_arms)*
                }
            }
        }
    }
    .into()
}

/// Macro to extract the documentation on any item that accepts doc comments
/// and store it in a const variable.
///
/// By default, this const variable inherits visibility from its parent item.
/// This can be manually configured; see configuration section below.
///
/// # Examples
///
/// ```rust
/// use documented::docs_const;
///
/// /// This is a test function
/// #[docs_const]
/// fn test_fn() {}
///
/// assert_eq!(TEST_FN_DOCS, "This is a test function");
/// ```
///
/// # Configuration
///
/// With the `customise` feature enabled, you can customise this macro's
/// behaviour using attribute arguments.
///
/// Currently, you can:
///
/// ## 1. set a custom constant visibility like so:
///
/// ```rust
/// mod submodule {
///     use documented::docs_const;
///     
///     /// Boo!
///     #[docs_const(vis = pub)]
///     struct Wooooo;
/// }
///
/// // notice how the constant can be seen from outside
/// assert_eq!(submodule::WOOOOO_DOCS, "Boo!");
/// ```
///
/// ## 2. set a custom constant name like so:
///
/// ```rust
/// use documented::docs_const;
///
/// /// If you have a question raise your hand
/// #[docs_const(name = "DONT_RAISE_YOUR_HAND")]
/// mod whatever {}
///
/// assert_eq!(DONT_RAISE_YOUR_HAND, "If you have a question raise your hand");
/// ```
///
/// ## 3. disable line-trimming like so:
///
/// ```rust
/// use documented::docs_const;
///
/// ///     This is a test constant
/// #[docs_const(trim = false)]
/// const test_const: u8 = 0;
///
/// assert_eq!(TEST_CONST_DOCS, "     This is a test constant");
/// ```
///
/// ---
///
/// Multiple option can be specified in a list like so:
/// `name = "FOO", trim = false`.
///
/// If there are other configuration options you wish to have, please
/// submit an issue or a PR.
#[proc_macro_attribute]
pub fn docs_const(#[allow(unused_variables)] attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as Item);

    #[cfg(not(feature = "customise"))]
    let config = AttrConfig::default();
    #[cfg(feature = "customise")]
    let config = AttrConfig::default()
        .with_customisations(syn::parse_macro_input!(attr as AttrCustomisations));

    let (item_vis, item_name, attrs) = match get_vis_name_attrs(&item) {
        Ok(pair) => pair,
        Err(e) => return e.into_compile_error().into(),
    };

    let docs = match get_docs(attrs, config.trim) {
        Ok(Some(docs)) => docs,
        Ok(None) => {
            // IDEA: customisation: allow_empty
            return Error::new(item.span(), "Missing doc comments")
                .into_compile_error()
                .into();
        }
        Err(e) => return e.into_compile_error().into(),
    };

    let const_vis = config.custom_vis.unwrap_or(item_vis);
    let const_name = config
        .custom_name
        .unwrap_or_else(|| format!("{}_DOCS", item_name.to_case(Case::ScreamingSnake)));
    let const_ident = Ident::new(&const_name, Span::call_site());

    // insert a const after the docs
    quote! {
        #item
        #const_vis const #const_ident: &'static str = #docs;
    }
    .into()
}
