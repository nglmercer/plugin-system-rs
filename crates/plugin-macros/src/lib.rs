use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Expr, ExprArray, FnArg, Ident, ImplItem, ItemImpl, ReturnType, Type};

#[proc_macro_attribute]
pub fn plugin_interface(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Attribute macro for marking methods as command handlers.
///
/// Place on methods inside an `impl Plugin for X` block or inherent impl block:
/// ```ignore
/// impl MyPlugin {
///     #[command("connect")]
///     fn connect(&mut self, host: String, port: u16) -> CommandResult {
///         // host and port are extracted from JSON args in handle_command
///     }
/// }
/// ```
///
/// This is a marker attribute for documentation and future tooling.
/// The actual command dispatch is handled manually in `handle_command`.
#[proc_macro_attribute]
pub fn command(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // Just pass through unchanged - this is a marker attribute
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

        if input.peek(syn::LitStr) {
            let lit: syn::LitStr = input.parse()?;
            let prefix = lit.value();

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

fn parse_export_args(attr: TokenStream) -> syn::Result<ExportArgs> {
    syn::parse(attr)
}

fn derive_prefix_from_type(ty: &Type) -> String {
    let type_name = match ty {
        Type::Path(p) => p
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default(),
        _ => return String::new(),
    };

    let name = type_name.strip_suffix("Plugin").unwrap_or(&type_name);

    if name.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                let prev = name.chars().nth(i - 1).unwrap_or(' ');
                if prev.is_lowercase()
                    || (prev.is_uppercase()
                        && name.chars().nth(i + 1).is_some_and(|c| c.is_lowercase()))
                {
                    result.push('_');
                }
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }

    result
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

/// Check if a method has a #[command] attribute (via __command_marker) and extract the command name.
fn get_command_name(_method: &syn::ImplItemFn) -> Option<String> {
    // No longer used - handle_command is implemented manually
    None
}

/// Generate the handle_command implementation from #[command] methods.
fn generate_handle_command(
    methods: &[&syn::ImplItemFn],
) -> proc_macro2::TokenStream {
    let mut match_arms = Vec::new();

    for method in methods {
        let command_name = match get_command_name(method) {
            Some(name) => name,
            None => continue,
        };

        let method_ident = &method.sig.ident;
        let mut arg_extractions = Vec::new();
        let mut method_args = Vec::new();

        for input_arg in &method.sig.inputs {
            match input_arg {
                FnArg::Receiver(_) => {
                    // self is handled separately
                }
                FnArg::Typed(pat_type) => {
                    let pat = &pat_type.pat;
                    let ty = &pat_type.ty;

                    let ty_str = match ty.as_ref() {
                        Type::Path(p) => p
                            .path
                            .segments
                            .last()
                            .map(|s| s.ident.to_string())
                            .unwrap_or_default(),
                        _ => String::new(),
                    };

                    let extraction = match ty_str.as_str() {
                        "String" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                        },
                        "u64" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0);
                        },
                        "u32" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32;
                        },
                        "u16" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u16;
                        },
                        "u8" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u8;
                        },
                        "i64" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0);
                        },
                        "i32" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                        },
                        "i16" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i16;
                        },
                        "i8" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i8;
                        },
                        "f64" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0);
                        },
                        "f32" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_f64())
                                .unwrap_or(0.0) as f32;
                        },
                        "bool" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                        },
                        "Option" => quote! {
                            let #pat = __args.get(stringify!(#pat)).cloned();
                        },
                        "Vec" => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(String::from))
                                        .collect()
                                })
                                .unwrap_or_default();
                        },
                        _ => quote! {
                            let #pat = __args.get(stringify!(#pat))
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);
                        },
                    };

                    arg_extractions.push(extraction);
                    method_args.push(quote! { #pat });
                }
            }
        }

        match_arms.push(quote! {
            #command_name => {
                #(#arg_extractions)*
                let __result = self.#method_ident(#(#method_args),*);
                plugin_system::command_to_json(__result)
            }
        });
    }

    if match_arms.is_empty() {
        quote! {
            fn handle_command(
                &mut self,
                _method: &str,
                _args: serde_json::Value,
            ) -> Option<serde_json::Value> {
                None
            }
        }
    } else {
        quote! {
            fn handle_command(
                &mut self,
                method: &str,
                args: serde_json::Value,
            ) -> Option<serde_json::Value> {
                let __args = args;
                match method {
                    #(#match_arms)*
                    _ => None,
                }
            }
        }
    }
}

/// Get all methods that have #[command] attribute.
fn get_command_methods(input: &ItemImpl) -> Vec<&syn::ImplItemFn> {
    input
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                if get_command_name(method).is_some() {
                    Some(method)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect()
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

    let self_ty = input.self_ty.as_ref();
    let resolved_prefix = match args.prefix.as_deref() {
        Some(p) => p.to_string(),
        None => derive_prefix_from_type(self_ty),
    };
    let prefix_opt = if resolved_prefix.is_empty() {
        None
    } else {
        Some(resolved_prefix.as_str())
    };
    let metadata_export_tokens = metadata_exports(self_ty, prefix_opt);

    let create_fn_name = match prefix_opt {
        Some(p) => format_ident!("plugin_{}_create", p),
        None => format_ident!("plugin_create"),
    };
    let destroy_fn_name = match prefix_opt {
        Some(p) => format_ident!("plugin_{}_destroy", p),
        None => format_ident!("plugin_destroy"),
    };

    // Check for #[command] methods and generate handle_command
    let command_methods = get_command_methods(&input);
    let handle_command_impl = generate_handle_command(&command_methods);

    // Keep all items including #[command] methods (they're the trait method implementations)
    let impl_items: Vec<_> = input.items.to_vec();

    // Check if handle_command is already defined in the impl block
    let has_handle_command = impl_items.iter().any(|item| {
        if let ImplItem::Fn(method) = item {
            method.sig.ident == "handle_command"
        } else {
            false
        }
    });

    // Build the impl block items
    let mut final_items = impl_items;

    // If there are command methods and no explicit handle_command, add the generated one
    if !command_methods.is_empty() && !has_handle_command {
        final_items.push(syn::parse_quote! {
            #handle_command_impl
        });
    }

    Ok(quote! {
        impl #trait_path for #self_ty {
            #(#final_items)*
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
    let resolved_prefix = match args.prefix.as_deref() {
        Some(p) => p.to_string(),
        None => derive_prefix_from_type(self_ty),
    };
    let prefix_opt = if resolved_prefix.is_empty() {
        None
    } else {
        Some(resolved_prefix.as_str())
    };

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

            let export_fn_name = match prefix_opt {
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
    let metadata_export_tokens = metadata_exports(self_ty, prefix_opt);

    let create_fn_name = match prefix_opt {
        Some(p) => format_ident!("plugin_{}_create", p),
        None => format_ident!("plugin_create"),
    };
    let destroy_fn_name = match prefix_opt {
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
