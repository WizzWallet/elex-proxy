use std::collections::HashMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use tokio::sync::{oneshot, RwLock};

pub type MokaCache = Cache<u64, R>;

#[derive(Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub method: String,
    pub params: Vec<Value>,
    pub id: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct JsonRpcResponse {
    pub result: Option<Value>,
    pub error: Option<Value>,
    pub id: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct R {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<bool>,
}

impl R {
    pub fn ok(payload: Value) -> Self {
        Self {
            success: true,
            response: Some(payload),
            code: None,
            message: None,
            health: None,
            cache: None,
        }
    }
    pub fn error(code: i32, message: String) -> Self {
        Self {
            success: false,
            response: None,
            code: Some(Value::Number(Number::from(code))),
            message: Some(Value::String(message)),
            health: None,
            cache: None,
        }
    }
    pub fn health(health: bool) -> Self {
        Self {
            success: true,
            response: None,
            code: None,
            message: None,
            health: Some(health),
            cache: None,
        }
    }
}

pub type Callbacks = Arc<RwLock<HashMap<u32, oneshot::Sender<JsonRpcResponse>>>>;

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let value = R::error(-1, self.0.to_string());
        Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::from(serde_json::to_string(&value).unwrap()))
            .unwrap()
    }
}

impl IntoResponse for R {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
