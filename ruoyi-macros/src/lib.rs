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
                let _ident: Ident = input.parse()?; // 消耗 "all"
                let _eq_token: Token![=] = input.parse()?; // 消耗 "="
                let content; // 准备一个变量来接收方括号内的内容
                bracketed!(content in input); // `bracketed!` 宏会解析 `[...]` 并将内部流放入 `content`
                let perms = Punctuated::<PermissionExpr, Token![,]>::parse_terminated(&content)?;
                Ok(PermissionArgs::All(perms.into_iter().collect()))
            } else if ident == "any" {
                // `any` 的逻辑与 `all` 完全相同。
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
            // 如果开头既不是字符串也不是标识符，语法肯定错了。
            Err(lookahead.error())
        }
    }
}


fn find_ident_in_pat(pat: &Pat) -> Option<&Ident> {
    match pat {
        Pat::Ident(pat_ident) => Some(&pat_ident.ident),
        Pat::Type(pat_type) => find_ident_in_pat(&pat_type.pat),
        Pat::TupleStruct(pat_tuple_struct) => {
            pat_tuple_struct.elems.iter().find_map(find_ident_in_pat)
        }
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

    let vis = &func.vis; // 函数的可见性 (pub)
    let sig = &func.sig; // 函数的完整签名 (async fn list(...))
    let original_block = &func.block; // 原始的函数体 `{ ... }`

    let check_logic = match args {
        // 情况一：单个权限
        PermissionArgs::Single(perm) => {
            // quote! 宏让我们能像写普通 Rust 代码一样编写将要生成的代码。
            // `#perm` 会被替换成 `PermissionExpr` 转换后的代码。
            quote! {
                let required_permission = #perm;
                if !user_perms.iter().any(|p| p == required_permission) {
                    return Err(::common::error::AppError::PermissionDenied);
                }
            }
        }
        // 情况二：`all` 逻辑
        PermissionArgs::All(perms) => {
            quote! {
                let required_permissions: &[&str] = &[#(#perms),*];
                // `.all()` 检查是否所有必需权限都被满足。
                if !required_permissions.iter().all(|req_p| user_perms.iter().any(|user_p| user_p == *req_p)) {
                    return Err(::common::error::AppError::PermissionDenied);
                }
            }
        }
        // 情况三：`any` 逻辑
        PermissionArgs::Any(perms) => {
            quote! {
                let required_permissions: &[&str] = &[#(#perms),*];
                // `.any()` 检查是否至少有一个必需权限被满足。
                if !required_permissions.iter().any(|req_p| user_perms.iter().any(|user_p| user_p == *req_p)) {
                    return Err(::common::error::AppError::PermissionDenied);
                }
            }
        }
    };

    let expanded = quote! {
        #vis #sig {
            let state_ref = &#state_ident;
            let claims_ref = &#claims_ident;
            let is_admin = claims_ref.user_id == 1;

            if !is_admin {
                let user_perms = match crate::user::service::get_user_permissions(&state_ref.db_pool, claims_ref.user_id).await {
                    Ok(perms) => perms,
                    Err(e) => return Err(e.into()),
                };
                #check_logic
            }

            #original_block
        }
    };

    TokenStream::from(expanded)
}