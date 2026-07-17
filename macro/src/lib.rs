//! The derive macro for the Mmio crate.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned, ToTokens, TokenStreamExt};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput, Expr, Field,
    Fields, Ident, Lit, Meta, Path, Token, TypeArray, TypePath,
};

#[proc_macro_derive(Mmio, attributes(mmio))]
pub fn derive_mmio(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // validate our input
    let input = parse_macro_input!(input as DeriveInput);
    try_derive_mmio(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn try_derive_mmio(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let mut is_repr_c = false;
    let mut omit_ctor = false;
    let mut omit_owned = false;
    let mut const_ptr = false;
    let mut const_inner = false;
    'attr: for attr in input.attrs.iter() {
        if attr.path().is_ident("mmio") {
            if let Meta::List(list) = &attr.meta {
                if let Err(e) = list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("no_ctors") {
                        omit_ctor = true;
                        return Ok(());
                    }
                    if meta.path.is_ident("no_owned") {
                        omit_owned = true;
                        return Ok(());
                    }
                    if meta.path.is_ident("const_ptr") {
                        const_ptr = true;
                        return Ok(());
                    }
                    if meta.path.is_ident("const_inner") {
                        const_inner = true;
                        return Ok(());
                    }
                    Err(meta.error(
                        "invalid content of mmio attribute, allowed values: `no_ctors`, `no_owned`, `const_ptr`, `const_inner`"
                    ))
                }) {
                    return Err(syn::Error::new(input.span(), e));
                };
            }
        }
        if attr.path().is_ident("repr") {
            let nested = attr
                .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .unwrap();
            for meta in nested {
                if let Meta::Path(path) = meta {
                    if path.is_ident("C") {
                        is_repr_c = true;
                        break 'attr;
                    }
                }
            }
        }
    }
    if !is_repr_c {
        return Err(syn::Error::new(
            input.ident.span(),
            "`#[derive(Mmio)]` only works on repr(C) types",
        ));
    }
    let ident = &input.ident;
    let wrapper_ident = format_ident!("Mmio{}", ident);
    let owned_ident = format_ident!("Owned{}", ident);
    let Data::Struct(ref s) = input.data else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(Mmio)]` only supports struct",
        ));
    };
    let Fields::Named(ref fields) = &s.fields else {
        return Err(syn::Error::new(
            input.span(),
            "`#[derive(Mmio)]` only supports structs with named fields",
        ));
    };

    let config = FieldConfig {
        const_ptr,
        const_inner,
    };
    let mut field_parser = FieldParser::new(config);
    // process the input to create the fragments we want
    let access_methods = fields
        .named
        .iter()
        .map(|field| (field, field.ident.as_ref().unwrap()))
        .filter(|(_field, field_ident)| !field_ident.to_string().starts_with("_"))
        .map(|(field, field_ident)| field_parser.generate_field_methods(&ident, field, field_ident))
        .collect::<syn::Result<Vec<_>>>()?;

    let access_methods_quoted = quote! {
        #(#access_methods)*
    };
    let field_sizes = fields.named.iter().map(field_size);
    let bound_checks = &field_parser.bound_checks;
    let offset_checks = &field_parser.offset_checks;
    let mut bound_check_func = TokenStream::new();
    if !bound_checks.is_empty() {
        bound_check_func.append_all(quote! {
            #[doc(hidden)]
            const fn __bound_check_mmio() {
                #(#bound_checks;)*
            }
        });
    }

    let constructors = if omit_ctor {
        None
    } else {
        Some(quote! {
            /// Create a new handle to this peripheral given an address.
            ///
            /// # Safety
            ///
            /// See the safety notes for [Self::new_mmio].
            pub const unsafe fn new_mmio_at(addr: usize) -> #wrapper_ident<'static> {
                Self::new_mmio(addr as *mut #ident)
            }

            /// Create a new handle to this peripheral.
            ///
            /// # Safety
            ///
            /// The pointer given must have suitable alignment, and point to an object
            /// which matches the layout given by the structure pointed to.
            ///
            /// If you create multiple instances of this handle at the same time,
            /// you are responsible for ensuring that there are no read-modify-write
            /// races on any of the registers.
            ///
            /// The [core::marker::Send] trait is implemented for the MMIO handle.
            /// However, there are cases where this implementation might be incorrect, for example
            /// if an MMIO handle was created for a core-local private address.
            ///
            /// In that case, it it is recommended to [un-implement Send](https://doc.rust-lang.org/nomicon/send-and-sync.html).
            /// on the register block structure.
            #[inline]
            pub const unsafe fn new_mmio(ptr: *mut #ident) -> #wrapper_ident<'static> {
                #wrapper_ident {
                    ptr,
                    phantom: core::marker::PhantomData,
                }
            }
        })
    };

    let owned_handles = if omit_owned {
        None
    } else {
        Some(quote! {
            #[doc = "A zero-sized Owned wrapper for ["]
            #[doc = stringify!(#ident)]
            #[doc = "] at an address known at compile-time (`BASE_ADDR`)"]
            pub struct #owned_ident<const BASE_ADDR: usize> {
                _inner: ()
            }

            impl<const BASE_ADDR: usize> #owned_ident<BASE_ADDR> {
                /// Create a new object representing ownership of a peripheral at an
                /// address.
                ///
                /// # Safety
                ///
                /// The `BASE_ADDR` given must be an address that has suitable
                /// alignment, and must point to an object which matches the layout
                /// given by the structure pointed to.
                ///
                /// If you create multiple instances of this handle at the same
                /// time, you are responsible for ensuring that there are no
                /// read-modify-write races on any of the registers.
                ///
                /// The [core::marker::Send] trait is implemented for the Owned
                /// handle. However, there are cases where this implementation might
                /// be incorrect, for example if an Owned handle was created for a
                /// core-local private address.
                ///
                /// In that case, it it is recommended to [un-implement
                /// Send](https://doc.rust-lang.org/nomicon/send-and-sync.html). on
                /// the register block structure.
                pub const unsafe fn new() -> Self {
                    Self {
                        _inner: ()
                    }
                }

                /// Mutably borrow the owned handle so it can be accessed
                ///
                /// This turns a zero-sized object with an address known at
                /// compile-time into an object the size of a pointer, which has all
                /// the read, write and modify access methods.
                pub fn borrow_mut<'owner>(&'owner mut self) -> #wrapper_ident<'owner> {
                    #wrapper_ident {
                        ptr: BASE_ADDR as *mut _,
                        phantom: core::marker::PhantomData,
                    }
                }

                /// Immutably borrow the owned handle so it can be accessed
                ///
                /// This turns a zero-sized object with an address known at
                /// compile-time into an object the size of a pointer, which has all
                /// the read access methods.
                pub fn borrow<'owner>(&'owner self) -> derive_mmio::SharedInner<#wrapper_ident<'owner>> {
                    derive_mmio::SharedInner::__new_internal(
                        #wrapper_ident {
                            ptr: BASE_ADDR as *mut _,
                            phantom: core::marker::PhantomData,
                        }
                    )
                }
            }

            impl<const BASE_ADDR: usize> ::core::fmt::Debug for #owned_ident<BASE_ADDR> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_tuple(stringify!(#owned_ident))
                        .field(&BASE_ADDR)
                        .finish()
                }
            }
        })
    };

    let vis = input.vis;

    // combine the fragments into the desired output code
    let tokens = quote! {
        #[doc = "An MMIO wrapper for ["]
        #[doc = stringify!(#ident)]
        #[doc = "]"]
        #[repr(transparent)]
        pub struct #wrapper_ident<'a> {
            ptr: *mut #ident,
            phantom: core::marker::PhantomData<&'a ()>,
        }

        impl core::fmt::Debug for #wrapper_ident<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_tuple(stringify!(#wrapper_ident))
                    .field(&self.ptr)
                    .finish()
            }
        }

        impl #wrapper_ident<'_> {
            const _REG_BLOCK_SIZE: usize = {
                0 #( + #field_sizes )*
            };

            // Must match expected size
            const _SIZE_CHECK: [(); #wrapper_ident::_REG_BLOCK_SIZE] = [(); core::mem::size_of::<#ident>()];

            /// Unsafely clone the MMIO handle.
            ///
            /// # Safety
            ///
            /// This allows to create multiple instances of the same MMIO handle. The user must ensure
            /// that these handles are not used concurrently in a way that leads to data races.
            #[inline]
            pub const unsafe fn clone(&self) -> Self {
                Self {
                    ptr: self.ptr,
                    phantom: core::marker::PhantomData,
                }
            }

            /// Retrieve the base pointer for this MMIO handle.
            #[inline]
            #vis const unsafe fn ptr(&self) -> *mut #ident {
                self.ptr
            }

            #access_methods_quoted
        }

        unsafe impl derive_mmio::_MmioMarker for #wrapper_ident<'_> {}

        #(#offset_checks)*

        /// The [core::marker::Send] trait is unsafely implemented because sending a register block pointer to another
        /// thread should not be an issue for most use-cases.
        ///
        /// However, there are cases where this [core::marker::Send] implementation might be
        /// invalid, for example if an MMIO handle was created for a core-local private address.
        ///
        /// In that case, it it is recommended to [un-implement Send](https://doc.rust-lang.org/nomicon/send-and-sync.html).
        /// on the register block structure.
        unsafe impl core::marker::Send for #wrapper_ident<'_> where #ident: core::marker::Send {}

        impl #ident {
            #bound_check_func

            #constructors
        }

        #owned_handles
    };
    Ok(tokens)
}

/// Convert a field into code that returns the field size
fn field_size(field: &Field) -> TokenStream {
    let ty = &field.ty;
    quote! {
        core::mem::size_of::<#ty>()
    }
}

fn parse_offset_literal(expr: Expr) -> syn::Result<u64> {
    let Expr::Lit(expr_lit) = expr else {
        return Err(syn::Error::new(
            expr.span(),
            "offset must be an integer literal",
        ));
    };
    let Lit::Int(lit) = expr_lit.lit else {
        return Err(syn::Error::new(
            expr_lit.lit.span(),
            "offset must be an integer literal",
        ));
    };

    let mut s = lit.to_string();
    let suffix = lit.suffix();
    if !suffix.is_empty() {
        s = s
            .strip_suffix(suffix)
            .expect("Failed to strip suffix from integer literal")
            .to_string()
    }
    let s = s.replace('_', "");

    let value = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)
    } else {
        s.parse::<u64>()
    };

    value.map_err(|_| syn::Error::new(lit.span(), "invalid integer for `offset`"))
}

#[derive(Debug, Default, PartialEq, Eq)]
enum ReadAccess {
    // Pure reads, no side effects.
    Pure,
    // Normal read access with side effects.
    #[default]
    Normal,
}

#[derive(Debug, Default)]
struct FieldAttributes {
    offset_to_check: Option<u64>,
    read: Option<ReadAccess>,
    write: bool,
    modify: bool,
}

impl FieldAttributes {
    pub fn convert_unmodified(&mut self) -> bool {
        if self.read.is_none() && !self.write && !self.modify {
            self.read = Some(ReadAccess::Pure);
            self.write = true;
            self.modify = true;
            return true;
        }
        false
    }
}

struct FieldConfig {
    pub const_ptr: bool,
    pub const_inner: bool,
}

struct FieldParser {
    bound_checks: Vec<TokenStream>,
    offset_checks: Vec<TokenStream>,
    config: FieldConfig,
}

impl FieldParser {
    pub fn new(config: FieldConfig) -> Self {
        Self {
            bound_checks: Vec::new(),
            offset_checks: Vec::new(),
            config,
        }
    }
    /// Convert a field into a set of methods that operate on that field
    ///
    /// Also returns the size of the field, which can be larger than the register size for
    /// inner MMIO blocks.
    fn generate_field_methods(
        &mut self,
        ident: &Ident,
        field: &Field,
        field_ident: &Ident,
    ) -> syn::Result<TokenStream> {
        let mut field_attrs = FieldAttributes::default();
        let mut output = TokenStream::new();
        let mut inner_mmio_detected = false;
        for attr in field.attrs.iter() {
            if attr.path().is_ident("mmio") {
                let Ok(nested) =
                    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                else {
                    return Err(syn::Error::new(
                        attr.span(),
                        "`Failed to parse #[mmio(...)]`",
                    ));
                };
                let unexpected_meta_printout =
                    "`#[mmio(...)]` only supports 'Inner', 'Read', 'PureRead', 'Write', and 'Modify' options";
                for meta in nested {
                    if let Meta::Path(path) = meta {
                        if path.is_ident("Inner") {
                            inner_mmio_detected = true;
                            output.append_all(self.generate_access_method_for_inner_mmio_field(
                                ident,
                                field,
                                field_ident,
                            ));
                        } else if path.is_ident("Read") {
                            if field_attrs.read.is_some() {
                                return Err(syn::Error::new(
                                    attr.span(),
                                    "`#[mmio(...)]` found second read argument",
                                ));
                            }
                            field_attrs.read = Some(ReadAccess::Normal);
                        } else if path.is_ident("PureRead") {
                            if field_attrs.read.is_some() {
                                return Err(syn::Error::new(
                                    attr.span(),
                                    "`#[mmio(...)]` found second read argument",
                                ));
                            }
                            field_attrs.read = Some(ReadAccess::Pure);
                        } else if path.is_ident("Write") {
                            if field_attrs.write {
                                return Err(syn::Error::new(
                                    attr.span(),
                                    "`#[mmio(...)]` found second write argument",
                                ));
                            }
                            field_attrs.write = true;
                        } else if path.is_ident("Modify") {
                            if field_attrs.modify {
                                return Err(syn::Error::new(
                                    attr.span(),
                                    "`#[mmio(...)]` found second write argument",
                                ));
                            }
                            field_attrs.modify = true;
                        } else {
                            return Err(syn::Error::new(attr.span(), unexpected_meta_printout));
                        }
                    } else if let Meta::List(list) = meta {
                        if list.path.is_ident("offset_bytes") {
                            field_attrs.offset_to_check =
                                Some(parse_offset_literal(syn::parse2(list.tokens)?)? as u64);
                        }
                    } else {
                        return Err(syn::Error::new(attr.span(), unexpected_meta_printout));
                    }
                }
            }
        }

        if let Some(offset_to_check) = field_attrs.offset_to_check {
            self.update_offset_checks(ident, field, field_ident, offset_to_check);
        }

        // Need an early return here. Otherwise, some invalid methods for inner MMIO blocks
        // might be generated.
        if inner_mmio_detected {
            return Ok(output);
        }

        if field_attrs.modify && (field_attrs.read.is_none() || !field_attrs.write) {
            return Err(syn::Error::new(
                field.span(),
                "Detected Modify field attribute without read and/or write access specifiers",
            ));
        }
        field_attrs.convert_unmodified();

        match &field.ty {
            syn::Type::Array(type_array) => {
                self.generate_array_access_methods(
                    ident,
                    field_attrs,
                    field_ident,
                    type_array,
                    &mut output,
                );
            }
            syn::Type::Path(type_path) => {
                self.generate_field_access_methods(
                    ident,
                    field_attrs,
                    field_ident,
                    type_path,
                    &mut output,
                );
            }
            _ => (),
        }

        Ok(output)
    }

    pub fn update_offset_checks(
        &mut self,
        ident: &Ident,
        field: &Field,
        field_ident: &Ident,
        offset_to_check: u64,
    ) {
        let expected_offset = offset_to_check as usize;
        let span = field.span();
        self.offset_checks.push(quote_spanned! { span =>
            const _: () = const {
                assert!(
                    core::mem::offset_of!(#ident, #field_ident) == #expected_offset,
                    concat!(
                        "Calculated offset for field `",
                        stringify!(#field_ident),
                        "` does not match given offset_bytes value"
                    )
                );
            };
        });
    }

    /// Generate access methods for fields that are MMIO blocks.
    pub fn generate_access_method_for_inner_mmio_field(
        &mut self,
        ident: &Ident,
        field: &Field,
        field_ident: &Ident,
    ) -> syn::Result<TokenStream> {
        match &field.ty {
            syn::Type::Path(type_path) => Ok(self.generate_access_method_for_single_inner_mmio(
                ident,
                field_ident,
                type_path,
            )),
            syn::Type::Array(array_type) => {
                if let syn::Type::Path(element_type) = array_type.elem.as_ref() {
                    Ok(self.generate_access_method_for_inner_mmio_array(
                        ident,
                        field_ident,
                        array_type,
                        element_type,
                    ))
                } else {
                    return Err(syn::Error::new(
                        array_type.span(),
                        format!(
                            "inner field array {} does not have a valid array type",
                            field.to_token_stream()
                        ),
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new(
                    field.span(),
                    format!(
                        "inner field {} does not have a valid path",
                        field.to_token_stream()
                    ),
                ));
            }
        }
    }

    pub fn generate_access_method_for_inner_mmio_array(
        &mut self,
        ident: &Ident,
        field_ident: &Ident,
        array_type: &TypeArray,
        element_type: &TypePath,
    ) -> TokenStream {
        let mut const_token = TokenStream::new();
        if self.config.const_inner {
            const_token.extend(quote! { const });
        }
        let array_len = array_type.len.clone();
        // Get the segments of the type path
        let mut segments = element_type.path.segments.clone();

        if let Some(last_segment) = segments.last_mut() {
            // Prepend "Mmio" to the last segment's identifier
            let new_ident =
                syn::Ident::new(&format!("Mmio{}", last_segment.ident), last_segment.span());

            // Modify the last segment to be "Mmio<LastSegment>"
            last_segment.ident = new_ident;
        }

        // Create the new Path
        let inner_mmio_path = Path {
            segments,
            leading_colon: element_type.path.leading_colon,
        };
        self.bound_checks.push(quote! {
            derive_mmio::is_mmio::<#inner_mmio_path>();
        });
        let array_len_func = format_ident!("len_{}", field_ident);
        let field_ident_unchecked = format_ident!("{}_unchecked", field_ident);

        let field_ident_shared = format_ident!("{}_shared", field_ident);
        let field_ident_shared_unchecked = format_ident!("{}_shared_unchecked", field_ident);

        let steal_func_name = format_ident!("steal_{}", field_ident);
        let steal_func_name_unchecked = format_ident!("steal_{}_unchecked", field_ident);

        let steal_func_name_shared = format_ident!("steal_{}_shared", field_ident);
        let steal_func_name_shared_unchecked =
            format_ident!("steal_{}_shared_unchecked", field_ident);
        let private_steal_unchecked_func_name = format_ident!("__steal_{}_unchecked", field_ident);
        let error_type = quote! { derive_mmio::OutOfBoundsError };
        quote! {
            #[doc = "Obtain a reference to the inner MMIO field "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = ""]
            #[doc = "# Lifetime"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is tied to the"]
            #[doc = "lifetime of this structure"]
            #[inline]
            pub fn #field_ident(&mut self, index: usize) -> Result<#inner_mmio_path<'_>, #error_type> {
                if index >= self.#array_len_func() {
                    return Err(#error_type(index));
                }
                // Safety: Index was checked.
                Ok(unsafe { self.#field_ident_unchecked(index) })
            }

            #[doc = "Obtain a reference to the inner MMIO field "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = ""]
            #[doc = "# Lifetime"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is tied to the"]
            #[doc = "lifetime of this structure"]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = ""]
            #[doc = "This function does not perform bounds checking and creates a MMIO structure "]
            #[doc = "based on a raw pointer which might lead to undefined behaviour on invalid offsets."]
            #[doc = "Users MUST ensure that the offset is valid."]
            #[inline]
            pub #const_token unsafe fn #field_ident_unchecked(&mut self, index: usize) -> #inner_mmio_path<'_> {
                unsafe {
                    self.#steal_func_name_unchecked(index)
                }
            }

            #[doc = "Obtain a shared instace of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub fn #field_ident_shared(&self, index: usize) -> Result<derive_mmio::SharedInner<#inner_mmio_path<'_>>, #error_type> {
                if index >= self.#array_len_func() {
                    return Err(#error_type(index));
                }
                // Safety: Index was checked.
                Ok(unsafe { self.#field_ident_shared_unchecked(index) })
            }

            #[doc = "Obtain a shared instance of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[doc = ""]
            #[doc = "This function does not perform bounds checking and creates a MMIO structure "]
            #[doc = "based on a raw pointer which might lead to undefined behaviour on invalid offsets."]
            #[doc = "Users MUST ensure that the offset is valid."]
            #[inline]
            pub #const_token unsafe fn #field_ident_shared_unchecked(&self, index: usize) -> derive_mmio::SharedInner<#inner_mmio_path<'_>> {
                derive_mmio::SharedInner::__new_internal(
                    unsafe {
                        self.#private_steal_unchecked_func_name(index)
                    }
                )
            }

            #[doc = "Steal inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub unsafe fn #steal_func_name(&mut self, index: usize) -> Result<#inner_mmio_path<'static>, #error_type> {
                if index >= self.#array_len_func() {
                    return Err(#error_type(index));
                }
                Ok(unsafe { self.#steal_func_name_unchecked(index) })
            }

            #[doc = "Steal inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[doc = ""]
            #[doc = "This function does not perform bounds checking and creates a MMIO structure "]
            #[doc = "based on a raw pointer which might lead to undefined behaviour on invalid offsets."]
            #[doc = "Users MUST ensure that the offset is valid."]
            #[inline]
            pub #const_token unsafe fn #steal_func_name_unchecked(&mut self, index: usize) -> #inner_mmio_path<'static> {
                unsafe { self.#private_steal_unchecked_func_name(index) }
            }

            #[doc = "Steal a shared instance of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub unsafe fn #steal_func_name_shared(&self, index: usize) -> Result<derive_mmio::SharedInner<#inner_mmio_path<'static>>, #error_type> {
                if index >= self.#array_len_func() {
                    return Err(#error_type(index));
                }
                Ok(unsafe { self.#steal_func_name_shared_unchecked(index) })
            }

            #[doc = "Steal a shared instance of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[doc = ""]
            #[doc = "This function does not perform bounds checking and creates a MMIO structure "]
            #[doc = "based on a raw pointer which might lead to undefined behaviour on invalid offsets."]
            #[doc = "Users MUST ensure that the offset is valid."]
            #[inline]
            pub #const_token unsafe fn #steal_func_name_shared_unchecked(&self, index: usize) -> derive_mmio::SharedInner<#inner_mmio_path<'static>> {
                derive_mmio::SharedInner::__new_internal(
                    unsafe {
                        self.#private_steal_unchecked_func_name(index)
                    }
                )
            }

            #[doc = "Length of the inner MMIO array `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[inline]
            pub const fn #array_len_func(&self) -> usize {
                #array_len
            }

            #[doc(hidden)]
            #const_token unsafe fn #private_steal_unchecked_func_name(&self, index: usize) -> #inner_mmio_path<'static> {
                let ptr = unsafe {(*self.ptr).#field_ident.as_mut_ptr().add(index) };
                unsafe {
                    #element_type::new_mmio(ptr)
                }
            }

        }
    }

    pub fn generate_access_method_for_single_inner_mmio(
        &mut self,
        ident: &Ident,
        field_ident: &Ident,
        type_path: &TypePath,
    ) -> TokenStream {
        let mut const_token = TokenStream::new();
        if self.config.const_inner {
            const_token.extend(quote! { const });
        }
        // Get the segments of the type path
        let mut segments = type_path.path.segments.clone();

        if let Some(last_segment) = segments.last_mut() {
            // Prepend "Mmio" to the last segment's identifier
            let new_ident =
                syn::Ident::new(&format!("Mmio{}", last_segment.ident), last_segment.span());

            // Modify the last segment to be "Mmio<LastSegment>"
            last_segment.ident = new_ident;
        }

        // Create the new Path
        let inner_mmio_path = Path {
            segments,
            leading_colon: type_path.path.leading_colon,
        };
        self.bound_checks.push(quote! {
            derive_mmio::is_mmio::<#inner_mmio_path>();
        });
        let field_ident_shared = format_ident!("{}_shared", field_ident);
        let steal_func_name = format_ident!("steal_{}", field_ident);
        let steal_func_name_shared = format_ident!("steal_{}_shared", field_ident);
        let steal_func_unchecked_name = format_ident!("__steal_{}_unchecked", field_ident);
        quote! {
            #[doc = "Obtain a reference to the inner MMIO field "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = ""]
            #[doc = "# Lifetime"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is tied to the"]
            #[doc = "lifetime of this structure"]
            #[inline]
            pub #const_token fn #field_ident(&mut self) -> #inner_mmio_path<'_> {
                unsafe {
                    self.#steal_func_name()
                }
            }

            #[doc = "Obtain a shared instace of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub #const_token fn #field_ident_shared(&self) -> derive_mmio::SharedInner<#inner_mmio_path<'_>> {
                derive_mmio::SharedInner::__new_internal(
                    unsafe {
                        self.#steal_func_unchecked_name()
                    }
                )
            }

            #[doc = "Steal inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub #const_token unsafe fn #steal_func_name(&mut self) -> #inner_mmio_path<'static> {
                unsafe { self.#steal_func_unchecked_name() }
            }

            #[doc = "Steal a shared instance of the inner MMIO field `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[doc = "This variant only allow access to non-mutable methods of the MMIO block,"]
            #[doc = "but only requires a shared reference to the outer MMIO block."]
            #[doc = ""]
            #[doc = "# Lifetime and Safety"]
            #[doc = ""]
            #[doc = "The lifetime of the returned inner MMIO block is static which"]
            #[doc = "allows independent usage of the inner block and arbitrary"]
            #[doc = "creation of of multiple inner blocks for the same peripheral."]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub #const_token unsafe fn #steal_func_name_shared(&self) -> derive_mmio::SharedInner<#inner_mmio_path<'static>> {
                derive_mmio::SharedInner::__new_internal(
                    unsafe {
                        self.#steal_func_unchecked_name()
                    }
                )
            }

            #[doc(hidden)]
            #const_token unsafe fn #steal_func_unchecked_name(&self) -> #inner_mmio_path<'static> {
                let ptr = unsafe { core::ptr::addr_of_mut!((*self.ptr).#field_ident) };
                unsafe {
                    #type_path::new_mmio(ptr)
                }
            }
        }
    }

    fn generate_field_access_methods(
        &mut self,
        ident: &Ident,
        field_attributes: FieldAttributes,
        field_ident: &Ident,
        type_path: &TypePath,
        access_methods: &mut TokenStream,
    ) {
        let mut const_token = TokenStream::new();
        if self.config.const_ptr {
            const_token.extend(quote! { const });
        }
        let pointer_fn_name = format_ident!("pointer_to_{}", field_ident);
        let read_fn_name = format_ident!("read_{}", field_ident);
        let write_fn_name = format_ident!("write_{}", field_ident);
        let modify_fn_name = format_ident!("modify_{}", field_ident);

        access_methods.append_all(quote! {
            #[doc = "Obtain a pointer to the "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = " register."]
            #[doc = ""]
            #[doc = "This function allows modification through the pointer but does not require"]
            #[doc = "mutable access to the MMIO handle."]
            #[doc = "The caller should specify the correct mutability depending on how the pointer is used."]
            #[doc = ""]
            #[doc = "Never create a reference from this pointer - only use read/write/read_volatile/write_volatile methods on it."]
            #[inline(always)]
            pub #const_token fn #pointer_fn_name(&self) -> *mut #type_path{
                unsafe { core::ptr::addr_of_mut!((*self.ptr).#field_ident) }
            }
        });
        if let Some(read_access) = field_attributes.read {
            let opt_mut = (read_access == ReadAccess::Normal).then_some(quote! { mut });

            access_methods.append_all(quote! {
                #[doc = "Read the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[inline(always)]
                pub fn #read_fn_name(&#opt_mut self) -> #type_path {
                    let addr = unsafe { core::ptr::addr_of!((*self.ptr).#field_ident) };
                    unsafe {
                        addr.read_volatile()
                    }
                }
            });
        }
        if field_attributes.write {
            access_methods.append_all(quote! {
                #[doc = "Write the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[inline(always)]
                pub fn #write_fn_name(&mut self, value: #type_path) {
                    let addr = self.#pointer_fn_name();
                    unsafe {
                        addr.write_volatile(value)
                    }
                }
            });
        }
        if field_attributes.modify {
            access_methods.append_all(quote! {
            #[doc = "Read-Modify-Write the "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = " register."]
            #[inline]
            pub fn #modify_fn_name<F>(&mut self, f: F) where F: FnOnce(#type_path) -> #type_path {
                let value = self. #read_fn_name();
                let new_value = f(value);
                self. #write_fn_name(new_value);
            }
        });
        }
    }

    fn generate_array_access_methods(
        &self,
        ident: &Ident,
        access: FieldAttributes,
        field_ident: &Ident,
        type_array: &TypeArray,
        access_methods: &mut TokenStream,
    ) {
        let mut const_token = TokenStream::new();
        if self.config.const_ptr {
            const_token.extend(quote! { const });
        }
        let array_type = &type_array.elem;
        let array_len = &type_array.len;
        let pointer_fn_name = format_ident!("pointer_to_{}_start", field_ident);
        let read_fn_name = format_ident!("read_{}", field_ident);
        let unchecked_read_fn_name = format_ident!("read_{}_unchecked", field_ident);
        let write_fn_name = format_ident!("write_{}", field_ident);
        let unchecked_write_fn_name = format_ident!("write_{}_unchecked", field_ident);
        let unchecked_modify_fn_name = format_ident!("modify_{}_unchecked", field_ident);
        let modify_fn_name = format_ident!("modify_{}", field_ident);
        let array_len_func = format_ident!("len_{}", field_ident);
        let error_type = quote! { derive_mmio::OutOfBoundsError };

        access_methods.append_all(quote! {
            #[doc = "Obtain a pointer to the "]
            #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
            #[doc = " first entry register array."]
            #[doc = ""]
            #[doc = "This function allows modification through the pointer but does not require"]
            #[doc = "mutable access to the MMIO handle."]
            #[doc = "The caller should specify the correct mutability depending on how the pointer is used."]
            #[doc = ""]
            #[doc = "Never create a reference from this pointer - only use read/write/read_volatile/write_volatile methods on it."]
            #[doc = "The `add` method method of the pointer can be used to access entries of the array at higher indices."]
            #[inline(always)]
            pub #const_token fn #pointer_fn_name(&self) -> *mut #array_type{
                unsafe { (*self.ptr).#field_ident.as_mut_ptr() }
            }
        });

        access_methods.append_all(quote! {
            #[doc = "Length of the array `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "`."]
            #[inline]
            pub const fn #array_len_func(&self) -> usize {
                #array_len
            }
        });

        if let Some(read_access) = access.read {
            let mut opt_mut = TokenStream::new();
            if read_access == ReadAccess::Normal {
                opt_mut.append_all(quote! { mut });
            }
            access_methods.append_all(quote! {
                #[doc = "Read the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = ""]
                #[doc = "# Safety "]
                #[doc = ""]
                #[doc = "This function does not perform bounds checking and performs a volatile "]
                #[doc = "read on a raw pointer with the given offset which might lead to "]
                #[doc = "undefined behaviour. Users MUST ensure that the offset is valid."]
                #[inline(always)]
                pub unsafe fn #unchecked_read_fn_name(&#opt_mut self, index: usize) -> #array_type {
                    let ptr = unsafe { (*self.ptr).#field_ident.as_mut_ptr() };
                    // Safety: We're performing a volatile read from a valid memory location
                    unsafe {
                        core::ptr::read_volatile(ptr.add(index))
                    }
                }

                #[doc = "Read the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = ""]
                #[doc = "This function also performs bound checking."]
                #[inline]
                pub fn #read_fn_name(
                    &#opt_mut self,
                    index: usize
                ) -> Result<#array_type, #error_type> {
                    if index >= #array_len {
                        return Err(#error_type(index));
                    }

                    // Safety: Correct index was verified.
                    Ok(unsafe { self.#unchecked_read_fn_name(index) })
                }
            });
        }

        if access.write {
            access_methods.append_all(quote! {
                #[doc = "Write the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = "# Safety "]
                #[doc = ""]
                #[doc = "This function does not perform bounds checking and performs a volatile "]
                #[doc = "write on a raw pointer with the given offset which might lead to "]
                #[doc = "undefined behaviour. Users MUST ensure that the offset is valid."]
                #[inline(always)]
                pub unsafe fn #unchecked_write_fn_name(&mut self, index: usize, value: #array_type) {
                    // Safety: We're performing a volatile read from a valid memory location
                    unsafe {
                        core::ptr::write_volatile(self.#pointer_fn_name().add(index), value)
                    }
                }

                #[doc = "Write the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = ""]
                #[doc = "This function also performs bound checking."]
                #[inline]
                pub fn #write_fn_name(
                    &mut self,
                    index: usize,
                    value: #array_type
                ) -> Result<(), #error_type> {
                    if index >= #array_len {
                        return Err(#error_type(index));
                    }
                    // Safety: Bound check was performed.
                    unsafe { self.#unchecked_write_fn_name(index, value) };
                    Ok(())
                }
            });
        }

        if access.modify {
            access_methods.append_all(quote! {
                #[doc = "Read-Modify-Write the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = ""]
                #[doc = "This function does not perform bounds checking and performs a volatile "]
                #[doc = "read and a volatile write on a raw pointer with the given offset which might lead to "]
                #[doc = "undefined behaviour. Users MUST ensure that the offset is valid."]
                #[inline]
                pub unsafe fn #unchecked_modify_fn_name<F>(
                    &mut self,
                    index: usize,
                    f: F
                ) where F: FnOnce(#array_type) -> #array_type {
                    let value = self. #unchecked_read_fn_name(index);
                    let new_value = f(value);
                    self. #unchecked_write_fn_name(index, new_value);
                }

                #[doc = "Read-Modify-Write the "]
                #[doc = concat!("[", stringify!(#ident), "::", stringify!(#field_ident), "]")]
                #[doc = " register."]
                #[doc = ""]
                #[doc = "This function also performs bound checking."]
                #[inline]
                pub fn #modify_fn_name(
                    &mut self,
                    index: usize,
                    f: impl FnOnce(#array_type) -> #array_type,
                ) -> Result<(), #error_type> {
                    let value = self. #read_fn_name(index)?;
                    // Unwrap is okay here, the index is checked in the read call.
                    self.#write_fn_name(index, f(value)).unwrap();
                    Ok(())
                }
            });
        }
    }
}
