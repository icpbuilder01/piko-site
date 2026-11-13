//! piko-site: serves one static site, embedded in the module at build time.
//!
//! - No update method: nothing can be uploaded, changed or deleted by calls.
//!   Changing the site means installing a new module (controllers only).
//! - Every ingress update call is refused in `canister_inspect_message`,
//!   which costs this canister 0 cycles. Pages are served by the
//!   `http_request` query, which is free for the canister.
//! - Responses are certified (response verification v2), like the stock
//!   asset canister, so icp0.io gateways and browsers can check them.

use std::cell::RefCell;

use ic_asset_certification::{
    Asset, AssetConfig, AssetEncoding, AssetFallbackConfig, AssetRedirectKind, AssetRouter,
};
use ic_http_certification::{HttpRequest, HttpResponse, StatusCode};

include!(concat!(env!("OUT_DIR"), "/assets.rs"));

thread_local! {
    static ROUTER: RefCell<AssetRouter<'static>> = RefCell::new(AssetRouter::default());
}

fn content_type(path: &str) -> Option<&'static str> {
    if path == ".well-known/ii-alternative-origins" {
        // Same as the stock asset canister served it.
        return Some("application/octet-stream");
    }
    let ext = path.rsplit('.').next()?.to_ascii_lowercase();
    Some(match ext.as_str() {
        "html" => "text/html",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "json" => "application/json",
        "webmanifest" => "application/manifest+json",
        "txt" => "text/plain",
        "xml" => "application/xml",
        "wasm" => "application/wasm",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "mp4" => "video/mp4",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    })
}

fn cache_control(path: &str) -> &'static str {
    // Vite puts content-hashed files under assets/: they never change.
    if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "public, no-cache"
    }
}

fn pct(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// The `ic_env` cookie the stock asset canister sets on HTML pages, read by
/// `getCanisterEnv()` in @icp-sdk/core: the root key plus every PUBLIC_*
/// canister environment variable (icp-cli sets PUBLIC_CANISTER_ID:<name>).
fn ic_env_cookie() -> String {
    let root_key: String = ic_cdk::api::root_key().iter().map(|b| format!("{b:02x}")).collect();
    let mut pairs = vec![format!("ic_root_key={root_key}")];
    let mut names: Vec<String> = (0..ic_cdk::api::env_var_count())
        .map(ic_cdk::api::env_var_name)
        .filter(|n| n.starts_with("PUBLIC_"))
        .collect();
    names.sort();
    for n in names {
        let v = ic_cdk::api::env_var_value(&n);
        pairs.push(format!("{n}={v}"));
    }
    format!("ic_env={}; Secure; SameSite=None; Partitioned", pct(&pairs.join("&")))
}

fn certify() {
    let cookie = ic_env_cookie();
    let mut assets: Vec<Asset<'static, 'static>> = Vec::new();
    let mut configs: Vec<AssetConfig> = Vec::new();
    let mut redirects: Vec<AssetConfig> = Vec::new();

    let mut push = |path: &'static str, raw: &'static [u8], gz: Option<&'static [u8]>| {
        assets.push(Asset::new(path, raw));
        let mut encodings = Vec::new();
        if let Some(gz) = gz {
            assets.push(Asset::new(format!("{path}.gz"), gz));
            encodings.push(AssetEncoding::Gzip.default_config());
        }
        let (fallback_for, aliased_by) = match path {
            "index.html" => (vec![], vec!["/".to_string()]),
            "404.html" => (
                vec![AssetFallbackConfig {
                    scope: "/".to_string(),
                    status_code: Some(StatusCode::NOT_FOUND),
                }],
                vec![],
            ),
            _ => match path.strip_suffix("/index.html") {
                // dir/index.html answers /dir/; /dir redirects there (307),
                // like the stock asset canister.
                Some(dir) => {
                    redirects.push(AssetConfig::Redirect {
                        from: format!("/{dir}"),
                        to: format!("/{dir}/"),
                        kind: AssetRedirectKind::Temporary,
                        headers: vec![("content-type".to_string(), "text/plain".to_string())],
                    });
                    (vec![], vec![format!("/{dir}/")])
                }
                None => (vec![], vec![]),
            },
        };
        let mut headers = vec![
            ("cache-control".to_string(), cache_control(path).to_string()),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
        ];
        if path.ends_with(".html") {
            headers.push(("set-cookie".to_string(), cookie.clone()));
        }
        configs.push(AssetConfig::File {
            path: path.to_string(),
            content_type: content_type(path).map(str::to_string),
            headers,
            fallback_for,
            aliased_by,
            encodings,
        });
    };

    for (path, raw, gz) in FILES.iter() {
        push(path, raw, *gz);
    }

    configs.extend(redirects);

    ROUTER.with_borrow_mut(|r| {
        *r = AssetRouter::default();
        r.certify_assets(assets, configs).expect("certify assets");
        ic_cdk::api::certified_data_set(r.root_hash());
    });
}

#[ic_cdk::init]
fn init() {
    certify();
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    certify();
}

/// Refuse every ingress update call before it is charged. Controllers do not
/// need any update method: changes go through install_code.
#[ic_cdk::inspect_message]
fn inspect_message() {}

#[ic_cdk::query]
fn http_request(req: HttpRequest<'static>) -> HttpResponse<'static> {
    let cert = ic_cdk::api::data_certificate().unwrap_or_default();
    ROUTER.with_borrow(|r| match r.serve_asset(&cert, &req) {
        Ok(resp) => resp,
        Err(_) => HttpResponse::builder()
            .with_status_code(StatusCode::NOT_FOUND)
            .with_body(b"Not found".to_vec())
            .build(),
    })
}

ic_cdk::export_candid!();
