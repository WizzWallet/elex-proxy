use crate::{handle_request, random_callback, AppError, Callbacks, JsonRpcRequest, R};
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Redirect, Response};
use axum::{Extension, Json};
use bitcoin::opcodes::all::{OP_ENDIF, OP_IF};
use bitcoin::{Script, Transaction};
use headers::HeaderMap;
use mime_guess::Mime;
use regex::Regex;
use serde_json::{Number, Value};
use std::io::Cursor;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::structs::MokaCache;

const ATOMICALS_PROTOCOL_ENVELOPE_ID: [u8; 4] = [97, 116, 111, 109];
// // dat
// const ATOMICALS_PROTOCOL_DAT: [u8; 3] = [100, 97, 116];

pub async fn handle_urn(
    Extension(callbacks): Extension<Vec<(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks)>>,
    Extension(cache): Extension<MokaCache>,
    headers: HeaderMap,
    Path(urn): Path<String>,
    Query(query): Query<Value>,
) -> Result<impl IntoResponse, AppError> {
    info!("URN: {}", &urn);
    let result = decode_urn(&urn).unwrap();
    debug!("URN info: {:?}", result);
    if UrnType::Dat == result.urn_type {
        let item = random_callback(&callbacks);
        let sender = item.0.clone();
        let calls = item.1.clone();
        let txid = result.identifier.split('i').collect::<Vec<&str>>()[0];
        let r = handle_request(
            cache,
            sender,
            calls,
            headers,
            "blockchain.transaction.get".into(),
            vec![Value::String(txid.to_string())],
        )
        .await;
        return if r.success {
            let value = r.response.unwrap();
            let rawhex = value.as_str().unwrap();
            let transaction = transaction_from_hex(rawhex).unwrap();
            let option = transaction.input.first().unwrap();
            let mut bytes = vec![];
            for witness in option.witness.iter().enumerate() {
                if witness.0 == 1 {
                    let script = Script::from_bytes(witness.1);
                    let mut start = false;
                    let mut index = 0;
                    for v in script.instructions() {
                        let instruction = v.unwrap();
                        if let Some(op) = instruction.opcode() {
                            if op == OP_IF {
                                start = true;
                                continue;
                            }
                        }
                        if let Some(op) = instruction.opcode() {
                            if op == OP_ENDIF {
                                break;
                            }
                        }
                        if start {
                            if index == 0 {
                                let x1 = instruction.push_bytes().unwrap().as_bytes();
                                if x1 != ATOMICALS_PROTOCOL_ENVELOPE_ID {
                                    return to_urn_r(R::error(
                                        -1,
                                        "ATOMICALS_PROTOCOL_ENVELOPE_ID not matched".to_string(),
                                    ));
                                }
                            } else if index == 1 {
                                // ignore ATOMICALS_PROTOCOL_OP_TYPE
                                // let x2 = instruction.push_bytes().unwrap().as_bytes();
                                // if x2 != ATOMICALS_PROTOCOL_DAT {
                                //     return to_urn_r(R::error(
                                //         -1,
                                //         "ATOMICALS_PROTOCOL_DAT tag not matched".to_string(),
                                //     ));
                                // }
                            } else {
                                let x3 = instruction.push_bytes().unwrap().as_bytes();
                                bytes.extend_from_slice(x3);
                            }
                            index += 1;
                        }
                    }
                    break;
                }
            }
            let cursor = Cursor::new(bytes);
            let v: ciborium::Value = ciborium::de::from_reader(cursor).unwrap();
            if let Some(p) = result.path {
                let f = &p[1..];
                if !f.is_empty() {
                    for (k, v) in v.as_map().unwrap().iter() {
                        let x = k.as_text().unwrap();
                        if x == f {
                            let mime_type = mime_guess::from_path(f).first_or_octet_stream();
                            if let Some(v) = find_cbor_first_bytes(v) {
                                return to_urn_response(
                                    mime_type,
                                    Body::from(v.as_bytes().unwrap().to_vec()),
                                );
                            }
                        }
                    }
                    return to_urn_r(R::error(-1, format!("No field found: {}", f)));
                }
            }
            to_urn_json(cbor_to_json(v))
        } else {
            to_urn_r(r)
        };
    }
    let atomical_id;
    if let UrnType::AtomicalId = result.urn_type {
        atomical_id = result.identifier;
    } else {
        let method = match result.urn_type {
            UrnType::Realm => "blockchain.atomicals.get_by_realm",
            UrnType::Container => "blockchain.atomicals.get_by_container",
            UrnType::Arc => "blockchain.atomicals.get_by_ticker",
            _ => unreachable!(),
        };
        let item = random_callback(&callbacks);
        let sender = item.0.clone();
        let calls = item.1.clone();
        let r = handle_request(
            cache.clone(),
            sender,
            calls,
            headers.clone(),
            method.into(),
            vec![Value::String(result.identifier)],
        )
        .await;
        if r.success {
            let res = r.response.unwrap();
            let aid = res
                .as_object()
                .and_then(|x| x.get("result"))
                .and_then(|x| x.as_object())
                .and_then(|x| x.get("atomical_id"))
                .and_then(|x| x.as_str());
            if let Some(aid) = aid {
                atomical_id = aid.to_string();
            } else {
                return to_urn_r(R::error(-1, "Not atomical found".to_string()));
            }
        } else {
            return to_urn_r(r);
        }
    }
    let item = random_callback(&callbacks);
    let sender = item.0.clone();
    let calls = item.1.clone();
    let r = handle_request(
        cache,
        sender,
        calls,
        headers,
        "blockchain.atomicals.get_state".into(),
        vec![Value::String(atomical_id), Value::Bool(false)],
    )
    .await;
    if r.success {
        let res = r.response.unwrap();
        let res = res
            .as_object()
            .and_then(|x| x.get("result"))
            .and_then(|x| x.as_object());
        if res.is_none() {
            return to_urn_r(R::error(-1, "No result found".to_string()));
        }
        if let Some(t) = result.path_type {
            if t == "$" {
                let path = result
                    .path
                    .map(|x| x[1..].to_owned())
                    .unwrap_or("".to_string());
                let location = res
                    .unwrap()
                    .get("mint_info")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("reveal_location")
                    .unwrap()
                    .as_str()
                    .unwrap();
                return to_urn_redirect(&format!("/urn/atom:btc:dat:{}{}{}", location, &t, path));
            }
        }
        let state = res
            .and_then(|x| x.get("state"))
            .and_then(|x| x.as_object())
            .and_then(|x| x.get("latest"));
        if let Some(state) = state {
            if let Some(p) = result.path {
                let f = &p[1..];
                if !f.is_empty() {
                    if let Some(v) = state.get(f) {
                        if let Some(b) = v.get("$b").and_then(|x| x.as_object()) {
                            let hex = b.get("$b").and_then(|x| x.as_str());
                            if let Some(hex) = hex {
                                let bytes = hex::decode(hex).unwrap();
                                let mime_type = match b.get("$ct").and_then(|x| x.as_str()) {
                                    Some(x) => Mime::from_str(x).unwrap_or(
                                        mime_guess::from_path(f).first_or_octet_stream(),
                                    ),
                                    None => mime_guess::from_path(f).first_or_octet_stream(),
                                };
                                return to_urn_response(mime_type, Body::from(bytes));
                            }
                        }
                    }
                    return to_urn_r(R::error(-1, format!("No field found: {}", f)));
                }
            }
            let use_image = query.as_object().unwrap().contains_key("image");
            if use_image {
                if let Some(urn) = state.get("image").and_then(|x| x.as_str()) {
                    if urn.starts_with("atom:btc:") {
                        let new_uri = format!("/urn/{}", urn);
                        return to_urn_redirect(&new_uri);
                    }
                }
            }
            to_urn_json(state.to_owned())
        } else {
            to_urn_r(R::error(-1, "No state found".to_string()))
        }
    } else {
        to_urn_r(r)
    }
}

fn find_cbor_first_bytes(cbor: &ciborium::Value) -> Option<&ciborium::Value> {
    if cbor.is_bytes() {
        return Some(cbor);
    }
    if cbor.is_map() {
        for (_, v) in cbor.as_map().unwrap().iter() {
            let x = find_cbor_first_bytes(v);
            if x.is_some() {
                return x;
            }
        }
    }
    if cbor.is_array() {
        for i in cbor.as_array().unwrap() {
            let x = find_cbor_first_bytes(i);
            if x.is_some() {
                return x;
            }
        }
    }
    if cbor.is_tag() {
        return find_cbor_first_bytes(cbor.as_tag().unwrap().1);
    }
    None
}

fn cbor_to_json(cbor: ciborium::Value) -> Value {
    if let ciborium::Value::Integer(x) = cbor {
        let result: i64 = x.try_into().unwrap();
        Value::Number(Number::from(result))
    } else if let ciborium::Value::Bytes(x) = cbor {
        Value::String(hex::encode(x))
    } else if let ciborium::Value::Text(x) = cbor {
        Value::String(x)
    } else if let ciborium::Value::Array(x) = cbor {
        let mut v = vec![];
        for i in x {
            v.push(cbor_to_json(i));
        }
        Value::Array(v)
    } else if let ciborium::Value::Map(x) = cbor {
        let mut v = serde_json::Map::new();
        for (k, i) in x {
            v.insert(k.as_text().unwrap().to_string(), cbor_to_json(i));
        }
        Value::Object(v)
    } else if let ciborium::Value::Tag(_, x) = cbor {
        cbor_to_json(*x)
    } else if let ciborium::Value::Float(x) = cbor {
        Value::Number(Number::from_f64(x).unwrap())
    } else if let ciborium::Value::Null = cbor {
        Value::Null
    } else if let ciborium::Value::Bool(x) = cbor {
        Value::Bool(x)
    } else {
        unreachable!()
    }
}

fn to_urn_response(context_type: Mime, body: Body) -> anyhow::Result<Response, AppError> {
    Ok(Response::builder()
        .header(CONTENT_TYPE, context_type.to_string())
        .body(body)
        .unwrap())
}

fn to_urn_json(json: Value) -> anyhow::Result<Response, AppError> {
    Ok(Json(json).into_response())
}

fn to_urn_r(r: R) -> anyhow::Result<Response, AppError> {
    Ok(r.into_response())
}

fn to_urn_redirect(urn: &str) -> anyhow::Result<Response, AppError> {
    Ok(Redirect::permanent(urn).into_response())
}

fn transaction_from_hex(hex: &str) -> anyhow::Result<Transaction> {
    let bytes = hex::decode(hex)?;
    let transaction = bitcoin::consensus::deserialize(&bytes)?;
    Ok(transaction)
}

#[derive(Debug, PartialEq)]
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

pub fn decode_urn(urn: &str) -> anyhow::Result<UrnInfo> {
    if let Some(captures) = Regex::new(r"atom:btc:dat:([0-9a-f]{64}i\d+)(([/$])?.*)")?.captures(urn)
    {
        return process_match(UrnType::Dat, captures);
    } else if let Some(captures) =
        Regex::new(r"atom:btc:id:([0-9a-f]{64}i\d+)(([/$])?.*)")?.captures(urn)
    {
        return process_match(UrnType::AtomicalId, captures);
    } else if let Some(captures) =
        Regex::new(r"atom:btc:realm:([a-z0-9-]+)(([/$])?.*)")?.captures(urn)
    {
        return process_match(UrnType::Realm, captures);
    } else if let Some(captures) =
        Regex::new(r"atom:btc:container:([a-z0-9-]+)(([/$])?.*)")?.captures(urn)
    {
        return process_match(UrnType::Container, captures);
    } else if let Some(captures) = Regex::new(r"atom:btc:arc:([a-z0-9]+)(([/$])?.*)")?.captures(urn)
    {
        return process_match(UrnType::Arc, captures);
    }
    Err(anyhow::anyhow!("Invalid URN"))
}

fn process_match(urn_type: UrnType, captures: regex::Captures) -> anyhow::Result<UrnInfo> {
    Ok(UrnInfo {
        urn_type,
        identifier: captures[1].to_string(),
        path_type: captures.get(3).map(|m| m.as_str().to_string()),
        path: match captures.get(2) {
            Some(m) => {
                let x = m.as_str();
                if x.is_empty() {
                    None
                } else {
                    Some(x.to_string())
                }
            }
            _ => None,
        },
    })
}
