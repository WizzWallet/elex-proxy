use anyhow::{anyhow, Result};
use axum::Extension;
use axum::extract::Path;
use futures::future::ok;
use regex::Regex;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::info;

use crate::{AppError, Callbacks, handle_request, JsonRpcRequest, R};

async fn handle_urn(
    Extension(callbacks): Extension<Callbacks>,
    Extension(ws_tx): Extension<mpsc::UnboundedSender<JsonRpcRequest>>,
    Path(urn): Path<String>,
) -> std::result::Result<R, AppError> {
    let result = decode_urn(&urn).unwrap();
    info!("URN info: {:?}", result);
    match result.urn_type {
        UrnType::AtomicalId => {
            Ok(R::ok(Value::Null))
        }
        UrnType::Realm => {
            let r = handle_request(callbacks, ws_tx, "blockchain.atomicals.get_by_realm".into(), vec![Value::String(result.identifier)]).await;
            if let Some(v) = r.response {
                let atomical_id = v.as_object().unwrap().get("result").unwrap().as_object().unwrap().get("atomical_id").unwrap().as_str();
                // if let Some(v) = atomical_id{
                //
                // }else {
                //     Ok(R::error(-1,format!("realm not found: {}", &result.identifier)))
                // }
                Ok(R::ok(Value::Null))
            }else {
                Ok(r)
            }
        }
        UrnType::Container => {
            Ok(R::ok(Value::Null))
        }
        UrnType::Dat => {
            Ok(R::ok(Value::Null))
        }
        UrnType::Arc => {
            Ok(R::ok(Value::Null))
        }
    }
}

#[derive(Debug)]
pub enum UrnType {
    AtomicalId = 0,
    Realm = 1,
    Container = 2,
    Dat = 3,
    Arc = 4,
}

#[derive(Debug)]
pub struct UrnInfo {
    pub urn_type: UrnType,
    pub identifier: String,
    pub path_type: Option<String>,
    pub path: Option<String>,
}

pub fn decode_urn(urn: &str) -> Result<UrnInfo> {
    if let Some(captures) = Regex::new(r"atom:btc:dat:([0-9a-f]{64}i\d+)(([/$])?.*)")?.captures(urn) {
        return process_match(UrnType::Dat, captures);
    }
    if let Some(captures) = Regex::new(r"atom:btc:id:([0-9a-f]{64}i\d+)(([/$])?.*)")?.captures(urn) {
        return process_match(UrnType::AtomicalId, captures);
    }
    if let Some(captures) = Regex::new(r"atom:btc:realm:(.+)(([/$])?.*)")?.captures(urn) {
        return process_match(UrnType::Realm, captures);
    }
    if let Some(captures) = Regex::new(r"atom:btc:container:(.+)(([/$])?.*)")?.captures(urn) {
        return process_match(UrnType::Container, captures);
    }
    if let Some(captures) = Regex::new(r"atom:btc:arc:(.+)(([/$])?.*)")?.captures(urn) {
        return process_match(UrnType::Arc, captures);
    }
    Err(anyhow!("Invalid URN"))
}

fn process_match(urn_type: UrnType, captures: regex::Captures) -> Result<UrnInfo> {
    Ok(UrnInfo {
        urn_type,
        identifier: captures[1].to_string(),
        path_type: Some(captures.get(3).map_or("", |m| m.as_str()).to_string()),
        path: captures.get(2).map(|m| m.as_str().to_string()),
    })
}
