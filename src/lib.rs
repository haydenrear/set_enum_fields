// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

//! # enum-fields
//! Grabbed from enum-fields and updated.
//! Mutable swap is used to replace without option.
//! ```

use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn;

#[proc_macro_derive(SetEnumFields)]
pub fn enum_fields_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    self::impl_for_input(&ast)
}

fn collect_available_fields<'input>(enum_data: &'input syn::DataEnum) -> HashMap<String, Vec<&'input syn::Field>> {
    let mut fields = HashMap::new();

    for variant in &enum_data.variants {
        for field in &variant.fields {
            if let Some(field_ident) = &field.ident {
                let ident = field_ident.to_string();
                fields.entry(ident)
                    .or_insert(Vec::new())
                    .push(field);
            }
        }
    }

    fields
}

fn impl_for_input(ast: &syn::DeriveInput) -> TokenStream {
    let fail_message = "`EnumFields` is only applicable to `enum`s";
    match &ast.data {
        syn::Data::Enum(data_enum) => impl_for_enum(ast, &data_enum),
        syn::Data::Union(data_union) => syn::Error::new(data_union.union_token.span, fail_message).to_compile_error().into(),
        syn::Data::Struct(data_struct) => syn::Error::new(data_struct.struct_token.span, fail_message).to_compile_error().into(),
    }
}

fn impl_for_enum(ast: &syn::DeriveInput, enum_data: &syn::DataEnum) -> TokenStream {
    let name = &ast.ident;

    // Collect available fields
    let fields = collect_available_fields(enum_data);

    let mut data = proc_macro2::TokenStream::new();

    let mut field_idents: Vec<Ident> = vec![];

    for (field_name, fields) in fields {
        let field_present_everywhere = fields.len() == enum_data.variants.len();

        let generics = &ast.generics;
        let field_type = &fields[0].ty;
        let field_name_ident = Ident::new(&field_name, Span::call_site());

        let mut variants = proc_macro2::TokenStream::new();
        let mut mut_set_variances = proc_macro2::TokenStream::new();


        for variant in &enum_data.variants {
            let name = &variant.ident;

            let variant_field_ident = variant.fields.iter()
                .find(|variant_field| {
                    if let Some(variant_field_ident) = &variant_field.ident {
                        if variant_field_ident.to_string() == field_name {
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                })
                .map(|field| {
                    field.ident.as_ref().unwrap()
                });

            match variant_field_ident {
                Some(variant_field_ident) => {
                    if field_present_everywhere {
                        variants.extend(quote! {
                            Self::#name{ #variant_field_ident, .. } =>  {
                                std::mem::swap(#variant_field_ident, to_set);
                            }
                        });
                    } else {
                        variants.extend(quote! {
                            Self::#name{ #variant_field_ident, .. } =>  {
                                std::mem::swap(#variant_field_ident, to_set);
                            }
                        });
                    }

                    if field_present_everywhere {
                        mut_set_variances.extend(quote! {
                        Self::#name{ #variant_field_ident, .. } => #variant_field_ident,
                    });
                    } else {
                        mut_set_variances.extend(quote! {
                        Self::#name{ #variant_field_ident, .. } => Some(#variant_field_ident),
                    });
                    }

                }
                None => {
                    // Field not present in field list.
                    if let Some(first_field) = variant.fields.iter().next() {
                        if first_field.ident.is_some() {
                            mut_set_variances.extend(quote! {
                                Self::#name{ .. } => None,
                            });
                        } else {
                            mut_set_variances.extend(quote! {
                                Self::#name(..) => None,
                            });
                        }
                    } else {
                        mut_set_variances.extend(quote! {
                            Self::#name => None,
                        });
                    }
                }
            }
        }

        let variant_field_ident = fields[0].ident.as_ref();
        if variant_field_ident.is_some() {
            let set_value = Ident::new(format!("set_{}", variant_field_ident.as_ref().unwrap().to_string()).as_str(), Span::call_site());
            data.extend(quote! {
                impl #generics #name #generics {
                    pub fn #set_value(&mut self, to_set: &mut #field_type) {
                        //! Get the property of this enum discriminant if it's available
                        match self {
                            #variants
                            _ => {}
                        };
                    }
                }
            });
        }

        let ty = if field_present_everywhere {
            quote! {
                &mut #field_type
            }
        } else {
            quote! {
                Option<&mut #field_type>
            }
        };

        let field_name_mut = Ident::new(format!("{}_mut", variant_field_ident.unwrap()).as_str(), Span::call_site());
        data.extend(quote! {
            impl #generics #name #generics {
                pub fn #field_name_mut(&mut self) -> #ty {
                    //! Get the property of this enum discriminant if it's available
                    match self {
                        #mut_set_variances
                    }
                }
            }
        });


    }

    data.into()
}
