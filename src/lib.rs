#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;
use syn::Token;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;

static ENV_FILE_PATH: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    #[cfg(debug_assertions)]
    let filename = ".env.dev";
    #[cfg(not(debug_assertions))]
    let filename = ".env.prod";
    find_env_file(filename)
});

static CONFIG: LazyLock<HashMap<String, String>> = LazyLock::new(|| ENV_FILE_PATH.as_ref().and_then(|p| load_file(p).ok()).unwrap_or_default());

fn find_env_file(filename: &str) -> Option<PathBuf> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut dir = PathBuf::from(manifest_dir);
    loop {
        let candidate = dir.join(filename);
        if candidate.exists() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn load_file(path: &PathBuf) -> std::io::Result<HashMap<String, String>> {
    let content = std::fs::read_to_string(path)?;

    let mut vars = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim().trim_matches('"').trim_matches('\'');
            vars.insert(key.to_owned(), value.to_owned());
        }
    }
    Ok(vars)
}

#[proc_macro]
pub fn dot(input: TokenStream) -> TokenStream {
    expand_env(input.into()).unwrap_or_else(|e| e.to_compile_error()).into()
}

fn expand_env(input_raw: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let args = <Punctuated<syn::LitStr, Token![,]>>::parse_terminated.parse(input_raw.into())?;

    let mut iter = args.iter();
    let var_name = iter.next().ok_or_else(|| syn::Error::new(args.span(), "dot! 需要1～2个参数"))?.value();
    let err_msg = iter.next();
    if iter.next().is_some() {
        return Err(syn::Error::new(args.span(), "dot! 只需要1～2个参数"));
    }

    // 文件路径是可选的：缺文件不报错，只在文件存在时才嵌入 include_bytes! 以追踪文件变更
    let track = ENV_FILE_PATH
        .as_ref()
        .and_then(|p| p.to_str())
        .map(|p| quote! { const _: &[u8] = include_bytes!(#p); });

    match CONFIG.get(&var_name).cloned().or_else(|| std::env::var(&var_name).ok()) {
        Some(val) => Ok(quote! {
            {
                #track
                #val
            }
        }),
        None => Err(syn::Error::new(
            var_name.span(),
            err_msg.map_or_else(|| format!("dot! 环境变量 `{}` 未定义", var_name), |lit| lit.value()),
        )),
    }
}
