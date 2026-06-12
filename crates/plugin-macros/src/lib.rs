use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Expr, ExprArray, FnArg, Ident, ImplItem, ItemImpl, ReturnType, Type};

#[proc_macro_attribute]
pub fn plugin_interface(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn plugin_export(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    match generate_plugin_export(attr, input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn plugin_export_all(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    match generate_plugin_export_all(attr, input) {
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

struct ExportArgs {
    prefix: Option<String>,
    interfaces: Vec<syn::Path>,
}

impl syn::parse::Parse for ExportArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(ExportArgs {
                prefix: None,
                interfaces: Vec::new(),
            });
        }

        // Try to parse as a string literal first
        if input.peek(syn::LitStr) {
            let lit: syn::LitStr = input.parse()?;
            let prefix = lit.value();

            // Check for trailing ", interfaces = [...]"
            if input.peek(syn::Token![,]) {
                let _comma: syn::Token![,] = input.parse()?;
                let ident: Ident = input.parse()?;
                if ident != "interfaces" {
                    return Err(syn::Error::new(ident.span(), "expected `interfaces`"));
                }
                let _eq: syn::Token![=] = input.parse()?;
                let arr: ExprArray = input.parse()?;
                let interfaces = arr
                    .elems
                    .into_iter()
                    .filter_map(|expr| {
                        if let Expr::Path(ep) = expr {
                            Some(ep.path)
                        } else {
                            None
                        }
                    })
                    .collect();
                return Ok(ExportArgs {
                    prefix: Some(prefix),
                    interfaces,
                });
            }

            return Ok(ExportArgs {
                prefix: Some(prefix),
                interfaces: Vec::new(),
            });
        }

        // Try to parse as `interfaces = [...]`
        let ident: Ident = input.parse()?;
        if ident == "interfaces" {
            let _eq: syn::Token![=] = input.parse()?;
            let arr: ExprArray = input.parse()?;
            let interfaces = arr
                .elems
                .into_iter()
                .filter_map(|expr| {
                    if let Expr::Path(ep) = expr {
                        Some(ep.path)
                    } else {
                        None
                    }
                })
                .collect();
            return Ok(ExportArgs {
                prefix: None,
                interfaces,
            });
        }

        Err(syn::Error::new(
            ident.span(),
            "expected string prefix or `interfaces = [...]`",
        ))
    }
}

/// Parse the attribute tokens.
fn parse_export_args(attr: TokenStream) -> syn::Result<ExportArgs> {
    syn::parse(attr)
}

fn metadata_exports(self_ty: &Type, prefix: Option<&str>) -> proc_macro2::TokenStream {
    let meta_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_metadata_json", p),
        None => format_ident!("plugin_metadata_json"),
    };
    let free_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_free_string", p),
        None => format_ident!("plugin_free_string"),
    };

    quote! {
        #[no_mangle]
        pub extern "C" fn #meta_fn_name() -> *mut std::ffi::c_char {
            match plugin_system::serde_json::to_vec(&plugin_system::Plugin::metadata(&<#self_ty>::new())) {
                Ok(json) => match std::ffi::CString::new(json) {
                    Ok(c_string) => c_string.into_raw(),
                    Err(_) => std::ptr::null_mut(),
                },
                Err(_) => std::ptr::null_mut(),
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn #free_fn_name(ptr: *mut std::ffi::c_char) {
            if !ptr.is_null() {
                drop(std::ffi::CString::from_raw(ptr));
            }
        }
    }
}

fn generate_plugin_export(
    attr: TokenStream,
    input: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    let args = parse_export_args(attr)?;

    let (_, trait_path, _) = input.trait_.as_ref().ok_or_else(|| {
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
    let self_ty = input.self_ty.as_ref();
    let prefix = args.prefix.as_deref();
    let metadata_export_tokens = metadata_exports(self_ty, prefix);

    let create_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_create", p),
        None => format_ident!("plugin_create"),
    };
    let destroy_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_destroy", p),
        None => format_ident!("plugin_destroy"),
    };

    Ok(quote! {
        impl #trait_path for #self_ty {
            #(#impl_items)*
        }

        #[no_mangle]
        pub extern "C" fn #create_fn_name() -> *mut () {
            let boxed: Box<dyn plugin_system::Plugin> = Box::new(<#self_ty>::new());
            let outer = Box::new(boxed);
            Box::into_raw(outer) as *mut ()
        }

        #[no_mangle]
        pub unsafe extern "C" fn #destroy_fn_name(ptr: *mut ()) {
            if !ptr.is_null() {
                let outer = Box::from_raw(ptr as *mut Box<dyn plugin_system::Plugin>);
                drop(outer);
            }
        }

        #metadata_export_tokens
    })
}

fn generate_plugin_export_all(
    attr: TokenStream,
    input: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    let args = parse_export_args(attr)?;

    let (_, trait_path, _) = input.trait_.as_ref().ok_or_else(|| {
        syn::Error::new_spanned(
            &input.self_ty,
            "#[plugin_export_all] must be on a trait impl block",
        )
    })?;
    let self_ty = input.self_ty.as_ref();

    let trait_last = trait_path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new_spanned(trait_path, "empty trait path"))?;

    if trait_last.ident != "Plugin" {
        return Err(syn::Error::new_spanned(
            &trait_last.ident,
            "#[plugin_export_all] must be on `impl Plugin for YourType`",
        ));
    }

    let impl_items = &input.items;
    let impl_attrs = &input.attrs;
    let prefix = args.prefix.as_deref();

    let mut method_exports = Vec::new();
    let mut method_names = Vec::new();

    for item in impl_items {
        if let ImplItem::Fn(method) = item {
            let method_name = &method.sig.ident;
            let method_name_str = method_name.to_string();

            if method_name_str == "metadata"
                || method_name_str == "on_load"
                || method_name_str == "on_unload"
                || method_name_str == "plugin_type_name"
            {
                continue;
            }

            let export_fn_name = match prefix {
                Some(p) => format_ident!("plugin_{}_method_{}", p, method_name),
                None => format_ident!("plugin_method_{}", method_name),
            };
            method_names.push(method_name_str);

            let mut params = Vec::new();
            let mut param_conversions = Vec::new();
            let mut has_mut_self = false;

            for param in &method.sig.inputs {
                match param {
                    FnArg::Receiver(r) => {
                        has_mut_self = r.mutability.is_some();
                    }
                    FnArg::Typed(pat_type) => {
                        let pat = &pat_type.pat;
                        let ty = &pat_type.ty;
                        params.push(quote! { #pat: *const std::ffi::c_void });
                        param_conversions.push((pat.clone(), ty.clone()));
                    }
                }
            }

            let receiver_type = if has_mut_self {
                quote! { *mut std::ffi::c_void }
            } else {
                quote! { *const std::ffi::c_void }
            };

            let ret_type = match &method.sig.output {
                ReturnType::Default => quote! { () },
                ReturnType::Type(_, ty) => match &**ty {
                    Type::Path(p) => {
                        let ident = &p.path.segments.last().unwrap().ident;
                        match ident.to_string().as_str() {
                            "String" => quote! { *mut std::ffi::c_char },
                            "u64" | "u32" | "u16" | "u8" | "i64" | "i32" | "i16" | "i8" | "f64"
                            | "f32" | "bool" => quote! { #ty },
                            _ => quote! { *const std::ffi::c_void },
                        }
                    }
                    Type::Reference(r) => {
                        if let Type::Path(p) = &*r.elem {
                            let ident = &p.path.segments.last().unwrap().ident;
                            if ident == "str" {
                                quote! { *const std::ffi::c_char }
                            } else {
                                quote! { *const std::ffi::c_void }
                            }
                        } else {
                            quote! { *const std::ffi::c_void }
                        }
                    }
                    _ => quote! { *const std::ffi::c_void },
                },
            };

            let param_args: Vec<_> = param_conversions
                .iter()
                .map(|(pat, ty)| match &**ty {
                    Type::Path(p) => {
                        let ident = &p.path.segments.last().unwrap().ident;
                        match ident.to_string().as_str() {
                            "String" => {
                                quote! {
                                    {
                                        let c_str = #pat as *const std::ffi::c_char;
                                        std::ffi::CStr::from_ptr(c_str)
                                            .to_str()
                                            .unwrap()
                                            .to_string()
                                    }
                                }
                            }
                            _ => quote! { #pat },
                        }
                    }
                    _ => quote! { #pat },
                })
                .collect();

            let return_conversion = match &method.sig.output {
                ReturnType::Default => quote! { let _ = __result; },
                ReturnType::Type(_, ty) => match &**ty {
                    Type::Path(p) => {
                        let ident = &p.path.segments.last().unwrap().ident;
                        match ident.to_string().as_str() {
                            "String" => {
                                quote! {
                                    std::ffi::CString::new(__result)
                                        .unwrap()
                                        .into_raw()
                                }
                            }
                            _ => quote! { __result },
                        }
                    }
                    Type::Reference(r) => {
                        if let Type::Path(p) = &*r.elem {
                            let ident = &p.path.segments.last().unwrap().ident;
                            if ident == "str" {
                                quote! {
                                    std::ffi::CString::new(__result)
                                        .unwrap()
                                        .into_raw()
                                }
                            } else {
                                quote! { __result as *const std::ffi::c_void }
                            }
                        } else {
                            quote! { __result as *const std::ffi::c_void }
                        }
                    }
                    _ => quote! { __result as *const std::ffi::c_void },
                },
            };

            let method_call = if has_mut_self {
                quote! {
                    let __plugin = __raw as *mut #self_ty;
                    (*__plugin).#method_name(#(#param_args),*)
                }
            } else {
                quote! {
                    let __plugin = __raw as *const #self_ty;
                    (*__plugin).#method_name(#(#param_args),*)
                }
            };

            method_exports.push(quote! {
                #[no_mangle]
                pub extern "C" fn #export_fn_name(
                    __raw: #receiver_type,
                    #(#params),*
                ) -> #ret_type {
                    let __result = unsafe { #method_call };
                    #return_conversion
                }
            });
        }
    }

    let _interface_names: Vec<_> = args
        .interfaces
        .iter()
        .map(|p| {
            p.segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default()
        })
        .collect();

    let _all_method_names = method_names;
    let metadata_export_tokens = metadata_exports(self_ty, prefix);

    let create_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_create", p),
        None => format_ident!("plugin_create"),
    };
    let destroy_fn_name = match prefix {
        Some(p) => format_ident!("plugin_{}_destroy", p),
        None => format_ident!("plugin_destroy"),
    };

    Ok(quote! {
        #(#impl_attrs)*
        impl #trait_path for #self_ty {
            #(#impl_items)*
        }

        #(#method_exports)*

        #[no_mangle]
        pub extern "C" fn #create_fn_name() -> *mut () {
            let boxed: Box<dyn plugin_system::Plugin> = Box::new(<#self_ty>::new());
            let outer = Box::new(boxed);
            Box::into_raw(outer) as *mut ()
        }

        #[no_mangle]
        pub unsafe extern "C" fn #destroy_fn_name(ptr: *mut ()) {
            if !ptr.is_null() {
                let outer = Box::from_raw(ptr as *mut Box<dyn plugin_system::Plugin>);
                drop(outer);
            }
        }

        #metadata_export_tokens
    })
}

fn generate_define_plugin(struct_type: syn::TypePath) -> syn::Result<proc_macro2::TokenStream> {
    let self_ty = syn::Type::Path(struct_type);
    let metadata_export_tokens = metadata_exports(&self_ty, None);

    Ok(quote! {
        #[no_mangle]
        pub extern "C" fn plugin_create() -> *mut () {
            let boxed: Box<dyn plugin_system::Plugin> = Box::new(<#self_ty>::new());
            let outer = Box::new(boxed);
            Box::into_raw(outer) as *mut ()
        }

        #[no_mangle]
        pub unsafe extern "C" fn plugin_destroy(ptr: *mut ()) {
            if !ptr.is_null() {
                let outer = Box::from_raw(ptr as *mut Box<dyn plugin_system::Plugin>);
                drop(outer);
            }
        }

        #metadata_export_tokens
    })
}
