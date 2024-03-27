use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct UsageInfo {
    note: String,
    #[serde(rename = "POST")]
    post: String,
    #[serde(rename = "GET")]
    get: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct Info {
    note: String,
    #[serde(rename = "usageInfo")]
    usage_info: UsageInfo,
    #[serde(rename = "healthCheck")]
    health_check: String,
    github: String,
    license: String,
    build: Build,
}

#[derive(Serialize, Deserialize, Clone)]
struct Build {
    version: String,
    #[serde(rename = "buildAt")]
    timestamp: String,
    #[serde(rename = "target")]
    target: String,
    #[serde(rename = "rustc")]
    rustc: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Response {
    success: bool,
    info: Info,
}

pub static PROXY_RESPONSE: LazyLock<Response> = LazyLock::new(|| {
    Response {
    success: true,
    info: Info {
        note: "Atomicals ElectrumX Digital Object Proxy Online".to_string(),
        usage_info: UsageInfo {
            note: "The service offers both POST and GET requests for proxying requests to ElectrumX. To handle larger broadcast transaction payloads use the POST method instead of GET.".to_string(),
            post: "POST /proxy/:method with string encoded array in the field \"params\" in the request body.".to_string(),
            get: "GET /proxy/:method?params=[\"value1\"] with string encoded array in the query argument \"params\" in the URL.".to_string(),
        },
        health_check: "GET /proxy/health".to_string(),
        github: "https://github.com/WizzWallet/elex-proxy".to_string(),
        license: "MIT".to_string(),
        build: Build {
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: env!("VERGEN_BUILD_TIMESTAMP").to_string(),
            target: env!("VERGEN_CARGO_TARGET_TRIPLE").to_string(),
            rustc: env!("VERGEN_RUSTC_SEMVER").to_string(),
        },
    },
}
});
