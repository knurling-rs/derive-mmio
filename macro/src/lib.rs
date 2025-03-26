//! The derive macro for the Mmio crate.

use proc_macro2::TokenStream;
use proc_macro_error2::{abort, abort_call_site, proc_macro_error};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, Data, DeriveInput, Field, Fields,
    Ident, Meta, Path, Token, TypeArray, TypePath,
};

#[proc_macro_error]
#[proc_macro_derive(Mmio, attributes(mmio))]
pub fn derive_mmio(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // validate our input
    let input = parse_macro_input!(input as DeriveInput);
    let mut is_repr_c = false;
    let mut omit_ctor = false;
    'attr: for attr in input.attrs.iter() {
        if attr.path().is_ident("mmio") {
            if let Meta::List(list) = &attr.meta {
                if let Err(e) = list.parse_nested_meta(|meta| {
                    if meta.path.is_ident("no_ctors") {
                        omit_ctor = true;
                        return Ok(());
                    }
                    Err(meta.error("invalid content of mmio attribute, only expected `no_ctors`"))
                }) {
                    abort!(e);
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
        abort_call_site!("`#[derive(Mmio)]` only works on repr(C) types");
    }
    let ident = input.ident;
    let wrapper_ident = format_ident!("Mmio{}", ident);
    let Data::Struct(ref s) = input.data else {
        abort_call_site!("`#[derive(Mmio)]` only supports struct");
    };
    let Fields::Named(ref fields) = &s.fields else {
        abort_call_site!("`#[derive(Mmio)]` only supports structs with named fields");
    };

    let mut field_parser = FieldParser::default();
    // process the input to create the fragments we want
    let access_methods = fields
        .named
        .iter()
        .map(|field| (field, field.ident.as_ref().unwrap()))
        .filter(|(_field, field_ident)| !field_ident.to_string().starts_with("_"))
        .map(|(field, field_ident)| field_parser.generate_access_methods(field, field_ident));

    let access_methods_quoted = quote! {
        #(#access_methods)*
    };
    let field_sizes = fields.named.iter().map(field_size);
    let bound_checks = &field_parser.bound_checks;
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
            #[doc = "Create a new handle to this peripheral given an address."]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = ""]
            #[doc = "See the safety notes for [`new_mmio`]."]
            pub const unsafe fn new_mmio_at(addr: usize) -> #wrapper_ident<'static> {
                Self::new_mmio(addr as *mut #ident)
            }

            #[doc = "Create a new handle to this peripheral."]
            #[doc = ""]
            #[doc = "# Safety"]
            #[doc = ""]
            #[doc = "The pointer given must have suitable alignment, and point to an object"]
            #[doc = "which matches the layout given by the structure pointed to."]
            #[doc = ""]
            #[doc = "If you create multiple instances of this handle at the same time,"]
            #[doc = "you are responsible for ensuring that there are no read-modify-write"]
            #[doc = "races on any of the registers."]
            #[inline]
            pub const unsafe fn new_mmio(ptr: *mut #ident) -> #wrapper_ident<'static> {
                #wrapper_ident {
                    ptr,
                    phantom: core::marker::PhantomData,
                }
            }
        })
    };

    // combine the fragments into the desired output code
    proc_macro::TokenStream::from(quote! {
        #[doc = "An MMIO wrapper for [`"]
        #[doc = stringify!(#ident)]
        #[doc = "`]"]
        pub struct #wrapper_ident<'a> {
            ptr: *mut #ident,
            phantom: core::marker::PhantomData<&'a ()>,
        }

        impl #wrapper_ident<'_> {
            const _FIELD_SIZE: usize = {
                0 #( + #field_sizes )*
            };

            // Must match expected size
            const _SIZE_CHECK: [(); #wrapper_ident::_FIELD_SIZE] = [(); core::mem::size_of::<#ident>()];

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
            pub const unsafe fn ptr(&self) -> *mut #ident {
                self.ptr
            }

            #access_methods_quoted
        }

        unsafe impl derive_mmio::_MmioMarker for #wrapper_ident<'_> {}

        impl #ident {
            #bound_check_func

            #constructors
        }

    })
}

/// Convert a field into code that returns the field size
fn field_size(field: &Field) -> TokenStream {
    let ty = &field.ty;
    quote! {
        core::mem::size_of::<#ty>()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Access {
    ReadOnly,
    ReadWrite,
}

#[derive(Default)]
struct FieldParser {
    bound_checks: Vec<TokenStream>,
}

impl FieldParser {
    /// Convert a field into a set of methods that operate on that field
    fn generate_access_methods(&mut self, field: &Field, field_ident: &Ident) -> TokenStream {
        let mut access = None;
        for attr in field.attrs.iter() {
            if attr.path().is_ident("mmio") {
                let Ok(nested) =
                    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                else {
                    abort_call_site!("`Failed to parse #[mmio(...)]`");
                };
                for meta in nested {
                    if let Meta::Path(path) = meta {
                        if path.is_ident("inner") {
                            return self.generate_access_method_for_inner_block(field, field_ident);
                        } else if path.is_ident("RO") {
                            if access.replace(Access::ReadOnly).is_some() {
                                abort_call_site!("`#[mmio(...)]` found second access argument");
                            }
                        } else if path.is_ident("RW") {
                            if access.replace(Access::ReadWrite).is_some() {
                                abort_call_site!("`#[mmio(...)]` found second access argument");
                            }
                        } else {
                            abort_call_site!(
                                "`#[mmio(...)]` only supports 'RO', 'RW' and 'inner' options"
                            );
                        }
                    } else {
                        abort_call_site!("`#[mmio(...)]` only supports 'RO' and 'RW' options");
                    }
                }
            }
        }

        // use ReadWrite for anything not otherwise marked
        let access = access.unwrap_or(Access::ReadWrite);

        let mut output = TokenStream::new();
        match &field.ty {
            syn::Type::Array(type_array) => {
                self.generate_array_access_methods(access, field_ident, type_array, &mut output);
            }
            syn::Type::Path(type_path) => {
                self.generate_field_access_methods(access, field_ident, type_path, &mut output);
            }
            _ => (),
        }

        output
    }

    /// Generate access methods for fields that are MMIO blocks.
    pub fn generate_access_method_for_inner_block(
        &mut self,
        field: &Field,
        field_ident: &Ident,
    ) -> TokenStream {
        match &field.ty {
            syn::Type::Path(type_path) => {
                // Get the segments of the type path
                let mut segments = type_path.path.segments.clone();

                if let Some(last_segment) = segments.last_mut() {
                    // Prepend "Mmio" to the last segment's identifier
                    let new_ident = syn::Ident::new(
                        &format!("Mmio{}", last_segment.ident),
                        last_segment.span(),
                    );

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
                let steal_func_name = format_ident!("steal_{}", field_ident);
                quote! {
                    #[doc = "Obtain a reference to the inner MMIO field `"]
                    #[doc = stringify!(#field_ident)]
                    #[doc = "`."]
                    #[doc = "# Lifetime"]
                    #[doc = ""]
                    #[doc = "The lifetime of the returned inner MMIO block is tied to the "]
                    #[doc = "lifetime of this structure"]
                    #[inline]
                    pub fn #field_ident(&mut self) -> #inner_mmio_path<'_> {
                        unsafe {
                            self.#steal_func_name()
                        }
                    }

                    #[doc = "Obtain a reference to the inner MMIO field `"]
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
                    pub unsafe fn #steal_func_name(&mut self) -> #inner_mmio_path<'static> {
                        let ptr = unsafe { core::ptr::addr_of_mut!((*self.ptr).#field_ident) };
                        unsafe {
                            #type_path::new_mmio(ptr)
                        }
                    }
                }
            }
            _ => {
                abort!(
                    "inner field {} does not have a valid path",
                    field.to_token_stream()
                );
            }
        }
    }
    fn generate_field_access_methods(
        &self,
        access: Access,
        field_ident: &Ident,
        type_path: &TypePath,
        access_methods: &mut TokenStream,
    ) {
        let pointer_fn_name = format_ident!("pointer_to_{}", field_ident);
        let read_fn_name = format_ident!("read_{}", field_ident);
        let write_fn_name = format_ident!("write_{}", field_ident);
        let modify_fn_name = format_ident!("modify_{}", field_ident);

        access_methods.append_all(quote! {
            #[doc = "Obtain a pointer to the `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "` register."]
            #[doc = ""]
            #[doc = "Never create a reference from this pointer - only use read/write/read_volatile/write_volatile methods on it."]
            #[inline(always)]
            pub fn #pointer_fn_name(&mut self) -> *mut #type_path{
                unsafe { core::ptr::addr_of_mut!((*self.ptr).#field_ident) }
            }

            #[doc = "Read the `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "` register."]
            #[inline(always)]
            pub fn #read_fn_name(&mut self) -> #type_path {
                let addr = self.#pointer_fn_name();
                unsafe {
                    addr.read_volatile()
                }
            }
        });

        if access == Access::ReadWrite {
            access_methods.append_all(quote! {
                #[doc = "Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
                #[inline(always)]
                pub fn #write_fn_name(&mut self, value: #type_path) {
                    let addr = self.#pointer_fn_name();
                    unsafe {
                        addr.write_volatile(value)
                    }
                }

                #[doc = "Read-Modify-Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
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
        access: Access,
        field_ident: &Ident,
        type_array: &TypeArray,
        access_methods: &mut TokenStream,
    ) {
        let array_type = &type_array.elem;
        let array_len = &type_array.len;
        let pointer_fn_name = format_ident!("pointer_to_{}_start", field_ident);
        let read_fn_name = format_ident!("read_{}", field_ident);
        let unchecked_read_fn_name = format_ident!("read_{}_unchecked", field_ident);
        let write_fn_name = format_ident!("write_{}", field_ident);
        let unchecked_write_fn_name = format_ident!("write_{}_unchecked", field_ident);
        let unchecked_modify_fn_name = format_ident!("modify_{}_unchecked", field_ident);
        let modify_fn_name = format_ident!("modify_{}", field_ident);
        let error_type = quote! { derive_mmio::OutOfBoundsError };

        access_methods.append_all(quote! {
            #[doc = "Obtain a pointer to the `"]
            #[doc = stringify!(#field_ident)]
            #[doc = "` first entry register array."]
            #[doc = ""]
            #[doc = "Never create a reference from this pointer - only use read/write/read_volatile/write_volatile methods on it."]
            #[doc = "The `add` method method of the pointer can be used to access entries of the array at higher indices."]
            #[inline(always)]
            pub fn #pointer_fn_name(&mut self) -> *mut #array_type{
                unsafe { (*self.ptr).#field_ident.as_mut_ptr() }
            }

            #[doc = "Read the `"]
            #[doc = stringify!(#field_ident)]
            #[doc = ""]
            #[doc = "` register."]
            #[doc = ""]
            #[doc = "# Safety "]
            #[doc = ""]
            #[doc = "This function does not perform bounds checking and performs a volatile "]
            #[doc = "read on a raw pointer with the given offset which might lead to "]
            #[doc = "undefined behaviour. Users MUST ensure that the offset is valid."]
            #[inline(always)]
            pub unsafe fn #unchecked_read_fn_name(&mut self, index: usize) -> #array_type {
                // Safety: We're performing a volatile read from a valid memory location
                unsafe {
                    core::ptr::read_volatile(self.#pointer_fn_name().add(index))
                }
            }

            #[doc = "Read the `"]
            #[doc = stringify!(#field_ident)]
            #[doc = ""]
            #[doc = "` register."]
            #[doc = ""]
            #[doc = "This function also peforms bound checking."]
            #[inline]
            pub fn #read_fn_name(
                &mut self,
                index: usize
            ) -> Result<#array_type, #error_type> {
                if index >= #array_len {
                    return Err(#error_type(index));
                }
                // Safety: Correct index was verified.
                Ok(unsafe { self.#unchecked_read_fn_name(index) })
            }
        });

        if access == Access::ReadWrite {
            access_methods.append_all(quote! {
                #[doc = "Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
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

                #[doc = "Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
                #[doc = ""]
                #[doc = "This function also peforms bound checking."]
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

                #[doc = "Read-Modify-Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
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

                #[doc = "Read-Modify-Write the `"]
                #[doc = stringify!(#field_ident)]
                #[doc = "` register."]
                #[doc = ""]
                #[doc = "This function also peforms bound checking."]
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
