extern crate proc_macro;

use proc_macro::TokenStream;

use proc_macro2::{Ident, Literal};
use quote::{quote, quote_spanned, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, ImplGenerics};

#[proc_macro_derive(TypeLayout)]
pub fn derive_type_layout(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;
    let name_str = Literal::string(&name.to_string());

    let layout = layout_of_type(&name, &input.data, &impl_generics);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl #impl_generics ::type_layout::TypeLayout for #name #ty_generics #where_clause {
            fn type_layout() -> ::type_layout::TypeLayoutInfo {
                // Need to specify type since it's possible for the struct
                // to have no fields, thus making "#layout" empty, resulting
                // in inference failure.
                let mut fields = Vec::<::type_layout::Field>::new();

                #layout

                fields.sort_by_key(|f| f.offset);

                ::type_layout::TypeLayoutInfo {
                    name: ::std::borrow::Cow::Borrowed(#name_str),
                    size: std::mem::size_of::<#name #impl_generics>(),
                    alignment: ::std::mem::align_of::<#name #impl_generics>(),
                    fields,
                }
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

fn layout_of_type(struct_name: &Ident, data: &Data, impl_generics: &ImplGenerics) -> proc_macro2::TokenStream {
    match data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => {
                let values = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    let field_name_str = Literal::string(&field_name.to_string());
                    let field_ty = &field.ty;
                    let field_ty_str = Literal::string(&field_ty.to_token_stream().to_string());

                    quote_spanned! { field.span() =>
                        #[allow(unused_assignments)]
                        {
                            let size = ::std::mem::size_of::<#field_ty>();
                            let offset = ::type_layout::memoffset::offset_of!(#struct_name #impl_generics, #field_name);

                            fields.push(::type_layout::Field {
                                name: ::std::borrow::Cow::Borrowed(#field_name_str),
                                ty: ::std::borrow::Cow::Borrowed(#field_ty_str),
                                size,
                                offset,
                            });
                        }
                    }
                });

                quote! {
                    #(#values)*
                }
            }
            Fields::Unnamed(fields) => {
                let values = fields.unnamed.iter().enumerate().map(|(index, field)| {
                    let field_ty = &field.ty;
                    let field_ty_str = Literal::string(&field_ty.to_token_stream().to_string());

                    let index_string = index.to_string();
                    let index = syn::Index::from(index);
                    quote_spanned! { field.span() =>
                        #[allow(unused_assignments)]
                        {
                            let size = ::std::mem::size_of::<#field_ty>();
                            let offset = ::type_layout::memoffset::offset_of!(#struct_name #impl_generics, #index);

                            fields.push(::type_layout::Field {
                                name: ::std::borrow::Cow::Borrowed(#index_string),
                                ty: ::std::borrow::Cow::Borrowed(#field_ty_str),
                                size,
                                offset,
                            });
                        }
                    }
                });

                quote! {
                    #(#values)*
                }
            },
            // Unit structs doesn't really have any fields
            Fields::Unit => {
                proc_macro2::TokenStream::new()
            },
        },
        Data::Enum(_) | Data::Union(_) => unimplemented!("type-layout only supports structs"),
    }
}
