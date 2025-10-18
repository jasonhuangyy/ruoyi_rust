use proc_macro::TokenStream;

use quote::{quote, ToTokens};

use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    FnArg, GenericArgument, Ident, ItemFn, LitStr, Pat, PatType, Path, PathArguments, Token, Type, TypePath,
};

enum PermissionExpr {
    Lit(LitStr),

    Path(Path),
}

impl ToTokens for PermissionExpr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            PermissionExpr::Path(path) => quote! { #path.as_ref() }.to_tokens(tokens),

            PermissionExpr::Lit(lit) => lit.to_tokens(tokens),
        }
    }
}

impl Parse for PermissionExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            input.parse().map(PermissionExpr::Lit)
        } else if lookahead.peek(Ident) {
            input.parse().map(PermissionExpr::Path)
        } else {
            Err(lookahead.error())
        }
    }
}

enum PermissionArgs {
    Single(PermissionExpr),

    All(Vec<PermissionExpr>),

    Any(Vec<PermissionExpr>),
}

impl Parse for PermissionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            Ok(PermissionArgs::Single(PermissionExpr::Lit(input.parse()?)))
        } else if lookahead.peek(Ident) {
            let ident: Ident = input.fork().parse()?;

            if ident == "all" {
                let _ident: Ident = input.parse()?;
                let _eq_token: Token![=] = input.parse()?;
                let content;
                bracketed!(content in input);

                let perms = Punctuated::<PermissionExpr, Token![,]>::parse_terminated(&content)?;
                Ok(PermissionArgs::All(perms.into_iter().collect()))
            } else if ident == "any" {
                let _ident: Ident = input.parse()?;
                let _eq_token: Token![=] = input.parse()?;
                let content;
                bracketed!(content in input);
                let perms = Punctuated::<PermissionExpr, Token![,]>::parse_terminated(&content)?;
                Ok(PermissionArgs::Any(perms.into_iter().collect()))
            } else {
                Ok(PermissionArgs::Single(input.parse()?))
            }
        } else {
            Err(lookahead.error())
        }
    }
}

fn find_ident_in_pat(pat: &Pat) -> Option<&Ident> {
    match pat {
        Pat::Ident(pat_ident) => Some(&pat_ident.ident),

        Pat::Type(pat_type) => find_ident_in_pat(&pat_type.pat),

        Pat::TupleStruct(pat_tuple_struct) => pat_tuple_struct.elems.iter().find_map(find_ident_in_pat),

        Pat::Reference(pat_ref) => find_ident_in_pat(&pat_ref.pat),
        _ => None,
    }
}

#[proc_macro_attribute]
pub fn require_permission(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as PermissionArgs);

    let func = parse_macro_input!(item as ItemFn);

    let mut state_ident = None;
    let mut claims_ident = None;

    for arg in &func.sig.inputs {
        if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
            if let Type::Path(TypePath { path, .. }) = &**ty {
                if let Some(segment) = path.segments.last() {
                    let ident_str = segment.ident.to_string();

                    if ident_str == "State" {
                        state_ident = find_ident_in_pat(pat).cloned();
                    } else if ident_str == "Extension" {
                        if let PathArguments::AngleBracketed(args) = &segment.arguments {
                            if let Some(GenericArgument::Type(Type::Path(inner_type_path))) = args.args.first() {
                                if let Some(inner_segment) = inner_type_path.path.segments.last() {
                                    if inner_segment.ident == "ClaimsData" {
                                        claims_ident = find_ident_in_pat(pat).cloned();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let state_ident = state_ident.expect("`#[require_permission]` 宏要求 handler 函数必须有一个 `State<...>` 类型的参数，并且能够从中解析出变量名。");
    let claims_ident = claims_ident.expect("`#[require_permission]` 宏要求 handler 函数必须有一个 `Extension<ClaimsData>` 类型的参数，并且能够从中解析出变量名。");

    let vis = &func.vis;
    let sig = &func.sig;
    let original_block = &func.block;

    let check_logic = match args {
        PermissionArgs::Single(perm) => {
            quote! {
                let required_permission = #perm;
                let has_permission = user_perms.iter().any(|p| p == required_permission);
                if !has_permission {
                    // 权限检查失败时，记录详细的警告信息
                    tracing::warn!(
                        user_id = claims_ref.user_id,
                        user_name = claims_ref.user_name,
                        required = required_permission,
                        owned = ?&user_perms, // 使用 `?` 格式化 Vec<String>
                        "Permission denied: User does not have the required permission."
                    );
                    return Err(::common::error::AppError::PermissionDenied);
                }
            }
        }

        PermissionArgs::All(perms) => {
            quote! {
                let required_permissions: &[&str] = &[#(#perms),*];
                let has_all_permissions = required_permissions.iter().all(|req_p| user_perms.iter().any(|user_p| user_p == *req_p));
                if !has_all_permissions {
                    tracing::warn!(
                        user_id = claims_ref.user_id,
                        user_name = %claims_ref.user_name,
                        required_all = ?required_permissions,
                        owned = ?&user_perms,
                        "Permission denied: User is missing one or more required permissions (ALL)."
                    );
                    return Err(::common::error::AppError::PermissionDenied);
                }

                // if !required_permissions.iter().all(|req_p| user_perms.iter().any(|user_p| user_p == *req_p)) {
                //     return Err(::common::error::AppError::PermissionDenied);
                // }
            }
        }

        PermissionArgs::Any(perms) => {
            quote! {
                let required_permissions: &[&str] = &[#(#perms),*];
                let has_any_permission = required_permissions.iter().any(|req_p| user_perms.iter().any(|user_p| user_p == *req_p));
                if !has_any_permission {
                    tracing::warn!(
                        user_id = claims_ref.user_id,
                        user_name = %claims_ref.user_name,
                        required_any = ?required_permissions,
                        owned = ?&user_perms,
                        "Permission denied: User has none of the required permissions (ANY)."
                    );
                    return Err(::common::error::AppError::PermissionDenied);
                }
                // if !required_permissions.iter().any(|req_p| user_perms.iter().any(|user_p| user_p == *req_p)) {
                //     return Err(::common::error::AppError::PermissionDenied);
                // }
            }
        }
    };

    let expanded = quote! {
        #vis #sig {
            let state_ref = &#state_ident;
            let claims_ref = &#claims_ident;
            let is_admin = claims_ref.user_id == 1;
             // 在执行权限检查前，先记录一条 info 级别的日志
            tracing::info!(
                user_id = claims_ref.user_id,
                user_name = %claims_ref.user_name,
                is_admin = is_admin,
                "Executing permission check for handler `{}`.",
                // 从函数签名中获取函数名
                stringify!(#sig.ident)
            );
            if !is_admin {
                let user_perms = match common::auth::get_user_permissions(&state_ref.db_pool, claims_ref.user_id).await {
                    Ok(perms) => perms,
                    Err(e) => {
                        // 如果数据库查询失败，也记录错误
                        tracing::error!("Failed to get user permissions during check: {:?}", e);
                        return Err(e.into());
                    },
                };
                // 注入带有详细日志的检查逻辑
                #check_logic
            }
            // 如果是管理员或权限检查通过，则执行原始的函数体
            #original_block
        }
    };

    TokenStream::from(expanded)
}
