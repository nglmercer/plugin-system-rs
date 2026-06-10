use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemImpl};

#[proc_macro_attribute]
pub fn plugin_interface(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn plugin_export(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    match generate_plugin_export(input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn define_plugin(item: TokenStream) -> TokenStream {
    let struct_type = syn::parse_macro_input!(item as syn::TypePath);
    generate_define_plugin(struct_type)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

fn generate_plugin_export(input: ItemImpl) -> syn::Result<proc_macro2::TokenStream> {
    let (_, trait_path, self_ty) = input.trait_.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &input.self_ty,
            "#[plugin_export] must be on a trait impl block",
        )
    })?;

    let trait_last = trait_path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(trait_path, "empty trait path"))?;

    if trait_last.ident != "Plugin" {
        return Err(syn::Error::new_spanned(
            &trait_last.ident,
            "#[plugin_export] must be on `impl Plugin for YourType`",
        ));
    }

    let impl_items = &input.items;
    let impl_attrs = &input.attrs;

    Ok(quote! {
        #(#impl_attrs)*
        impl #trait_path for #self_ty {
            #(#impl_items)*
        }

        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn plugin_create() -> *mut dyn plugin_system::Plugin {
            let boxed: Box<dyn plugin_system::Plugin> = Box::new(<#self_ty>::new());
            Box::into_raw(boxed)
        }

        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub unsafe extern "C" fn plugin_destroy(ptr: *mut dyn plugin_system::Plugin) {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
        }

        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn plugin_metadata() -> plugin_system::PluginMetadata {
            plugin_system::Plugin::metadata(&<#self_ty>::new())
        }
    })
}

fn generate_define_plugin(struct_type: syn::TypePath) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote! {
        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn plugin_create() -> *mut dyn plugin_system::Plugin {
            let boxed: Box<dyn plugin_system::Plugin> = Box::new(<#struct_type>::new());
            Box::into_raw(boxed)
        }

        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub unsafe extern "C" fn plugin_destroy(ptr: *mut dyn plugin_system::Plugin) {
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
            }
        }

        #[no_mangle]
        #[allow(improper_ctypes_definitions)]
        pub extern "C" fn plugin_metadata() -> plugin_system::PluginMetadata {
            plugin_system::Plugin::metadata(&<#struct_type>::new())
        }
    })
}
