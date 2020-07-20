use proc_macro2::{Ident, TokenStream};
use syn::spanned::Spanned;

use std::collections::HashMap;
use std::iter::FromIterator;

pub mod class_factory;
pub mod co_class_impl;
pub mod com_struct;
pub mod com_struct_impl;
pub mod iunknown_impl;

pub struct CoClass {
    name: Ident,
    docs: Vec<syn::Attribute>,
    visibility: syn::Visibility,
    interfaces: std::collections::HashMap<syn::Path, Interface>,
    methods: std::collections::HashMap<syn::Path, Vec<syn::ImplItemMethod>>,
    fields: Vec<syn::Field>,
}

struct Interface {
    path: syn::Path,
    parent: Option<Box<Interface>>,
}

impl Interface {
    /// Creates an intialized VTable for the interface
    fn to_initialized_vtable_tokens(&self, co_class: &CoClass) -> TokenStream {
        let vtable_ident = self.vtable_ident();
        let vtable_type = self.to_vtable_type_tokens();
        let parent = self
            .parent
            .as_ref()
            .map(|p| p.to_initialized_vtable_tokens(co_class))
            .unwrap_or_else(|| Self::iunknown_tokens(co_class));
        let fields = co_class.methods.get(&self.path).unwrap().iter().map(|m| {
            let name = &m.sig.ident;
            let params = m.sig.inputs.iter().filter_map(|p| 
                match p {
                    syn::FnArg::Receiver(_) => None,
                    syn::FnArg::Typed(p) => Some(p),
                }
            );
            let ret = &m.sig.output;
            let method = quote::quote! {
                unsafe extern "stdcall" fn #name(this: ::std::ptr::NonNull<::std::ptr::NonNull<#vtable_ident>>, #(#params)*) #ret {
                    todo!()
                }
            };
            quote::quote! {
                #name: {
                    #method
                    #name
                }

            }
        });
        quote::quote! {
            {
                #vtable_type
                #vtable_ident {
                    parent: #parent,
                    #(#fields)*,
                }
            }
        }
    }

    fn to_vtable_type_tokens(&self) -> TokenStream {
        let name = &self.path;
        let vtable_ident = self.vtable_ident();
        quote::quote! {
            type #vtable_ident = <#name as ::com::ComInterface>::VTable;
        }
    }

    fn vtable_ident(&self) -> proc_macro2::Ident {
        let name = &self.path;
        quote::format_ident!("{}VTable", name.get_ident().unwrap())
    }

    fn iunknown_tokens(co_class: &CoClass) -> TokenStream {
        let interfaces = &co_class.interfaces.keys().collect::<Vec<_>>();
        let add_ref = iunknown_impl::gen_add_ref(&co_class.name);
        let release = iunknown_impl::gen_release(interfaces, &co_class.name);
        let query_interface = iunknown_impl::gen_query_interface(&co_class.name, interfaces);
        quote::quote! {
            {
                type IUknownVTable = <::com::interfaces::IUnknown as ::com::ComInterface>::VTable;
                #add_ref
                #release
                #query_interface
                IUknownVTable {
                    AddRef: add_ref,
                    Release: release,
                    QueryInterface: query_interface,
                }
            }
        }
    }
}

impl syn::parse::Parse for CoClass {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut co_class = None;
        while !input.is_empty() {
            let docs = input.call(syn::Attribute::parse_outer)?;
            //TODO: ensure only docs attributes
            if !input.peek(syn::Token!(impl)) {
                let mut interfaces = HashMap::new();
                let visibility = input.parse::<syn::Visibility>()?;
                let _ = input.parse::<keywords::coclass>()?;
                let name = input.parse::<Ident>()?;
                let _ = input.parse::<syn::Token!(:)>()?;
                while !input.peek(syn::token::Brace) {
                    let path = input.parse::<syn::Path>()?;
                    let interface = Interface {
                        path: path.clone(),
                        parent: None,
                    };
                    interfaces.insert(path.clone(), interface);

                    let mut current = interfaces.get_mut(&path).unwrap();
                    while input.peek(syn::token::Paren) {
                        let contents;
                        syn::parenthesized!(contents in input);
                        let path = contents.parse::<syn::Path>()?;
                        let interface = Interface { path, parent: None };
                        current.parent = Some(Box::new(interface));
                        current = current.parent.as_mut().unwrap().as_mut();
                    }

                    if !input.peek(syn::token::Brace) {
                        let _ = input.parse::<syn::Token!(,)>()?;
                    }
                }
                let fields;
                syn::braced!(fields in input);
                let fields =
                    syn::punctuated::Punctuated::<syn::Field, syn::Token!(,)>::parse_terminated_with(
                        &fields,
                        syn::Field::parse_named
                    )?;
                let fields = fields.into_iter().collect();
                co_class = Some(CoClass {
                    name,
                    docs,
                    visibility,
                    interfaces,
                    methods: HashMap::new(),
                    fields,
                });
            } else {
                let item = input.parse::<syn::ItemImpl>()?;
                // TODO: ensure that co_class idents line up
                let (_, interface, _) = item.trait_.unwrap();
                let methods = item
                    .items
                    .into_iter()
                    .map(|i| match i {
                        syn::ImplItem::Method(m) => m,
                        _ => panic!(""),
                    })
                    .collect::<Vec<_>>();
                let co_class = co_class.as_mut().unwrap();
                // ensure not already there
                co_class.methods.insert(interface, methods);
            }
        }
        Ok(co_class.unwrap())
    }
}

impl CoClass {
    pub fn to_tokens(&self) -> TokenStream {
        // let base_interface_idents = crate::utils::base_interface_idents(attr_args);

        let mut out: Vec<TokenStream> = Vec::new();
        out.push(com_struct::generate(self));

        out.push(com_struct_impl::generate(self));

        // out.push(co_class_impl::generate(self));

        // out.push(iunknown_impl::generate(self));
        // out.push(class_factory::generate(input).into());

        TokenStream::from_iter(out)
    }
}

mod keywords {
    syn::custom_keyword!(coclass);
}
