#![feature(lazy_cell)]

use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, LazyLock};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::extract::{Path, Query};
use axum::extract::Extension;
use axum::extract::Json;
use axum::http;
use axum::http::{header, HeaderMap};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Response;
use axum::Router;
use axum::routing::get;
use bytes::Bytes;
use dotenv::dotenv;
use futures::{SinkExt, StreamExt};
use http_body_util::Full;
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::ip::maybe_ip_from_headers;

mod ip;
mod urn;

#[derive(Serialize)]
struct JsonRpcRequest {
    method: String,
    params: Vec<Value>,
    id: u32,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<Value>,
    id: u32,
}

#[derive(Serialize)]
struct R {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<bool>,
}

impl R {
    fn ok(payload: Value) -> Self {
        Self {
            success: true,
            response: Some(payload),
            code: None,
            message: None,
            health: None,
        }
    }
    fn error(code: i32, message: String) -> Self {
        Self {
            success: false,
            response: None,
            code: Some(Value::Number(Number::from(code))),
            message: Some(Value::String(message)),
            health: None,
        }
    }
    fn health(health: bool) -> Self {
        Self {
            success: true,
            response: None,
            code: None,
            message: None,
            health: Some(health),
        }
    }
}

static IP_LIMIT_PER_MILLS: LazyLock<u64> = LazyLock::new(|| {
    let per_second:u64 = env::var("IP_LIMIT_PER_SECOND")
        .unwrap_or("0".to_string())
        .parse()
        .unwrap();
    if per_second > 0 {
        per_second * 1000
    } else {
        env::var("IP_LIMIT_PER_MILLS")
            .unwrap_or("10".to_string())
            .parse()
            .unwrap()
    }
});

static IP_LIMIT_BURST_SIZE: LazyLock<u32> = LazyLock::new(|| {
    env::var("IP_LIMIT_BURST_SIZE")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap()
});

static CONCURRENCY_LIMIT: LazyLock<usize> = LazyLock::new(|| {
    env::var("CONCURRENCY_LIMIT")
        .unwrap_or("500".to_string())
        .parse()
        .unwrap()
});

static ELECTRUMX_WSS: LazyLock<String> = LazyLock::new(|| {
    env::var("ELECTRUMX_WSS").unwrap_or("wss://electrumx.atomicals.xyz:50012".to_string())
});

static ELECTRUMX_WS_INSTANCE: LazyLock<u32> = LazyLock::new(|| {
    env::var("ELECTRUMX_WS_INSTANCE")
        .unwrap_or("1".to_string())
        .parse()
        .unwrap()
});

static PROXY_HOST: LazyLock<String> =
    LazyLock::new(|| env::var("PROXY_HOST").unwrap_or("0.0.0.0:12321".into()));

static RESPONSE_TIMEOUT: LazyLock<u64> = LazyLock::new(|| {
    env::var("RESPONSE_TIMEOUT")
        .unwrap_or("10".to_string())
        .parse()
        .unwrap()
});

// The use of `AtomicU32` is to ensure not exceeding the integer range of other systems.
static ID_COUNTER: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

fn get_next_id() -> u32 {
    // Reset the counter when it reaches the maximum value.
    if ID_COUNTER.load(Ordering::SeqCst) == u32::MAX {
        ID_COUNTER.store(0, Ordering::SeqCst);
    }
    ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

type Callbacks = Arc<RwLock<HashMap<u32, oneshot::Sender<JsonRpcResponse>>>>;

struct AppError(anyhow::Error);

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

async fn handle_get(
    Extension(callbacks): Extension<Vec<(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks)>>,
    headers: HeaderMap,
    Path(method): Path<String>,
    Query(query): Query<Value>,
) -> Result<R, AppError> {
    let item = random_callback(&callbacks);
    let sender = item.0.clone();
    let calls = item.1.clone();
    let r = match query.get("params") {
        None => handle_request(sender, calls, headers, method, vec![]).await,
        Some(v) => {
            let x = v
                .as_str()
                .map(|s| if s.is_empty() { "[]" } else { s })
                .unwrap();
            let params = serde_json::from_str(x).unwrap();
            handle_request(sender, calls, headers, method, params).await
        }
    };
    Ok(r)
}

fn random_callback(
    callbacks: &[(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks)],
) -> &(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks) {
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..callbacks.len());
    &callbacks[index]
}

async fn handle_post(
    Extension(callbacks): Extension<Vec<(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks)>>,
    headers: HeaderMap,
    Path(method): Path<String>,
    body: Option<Json<Value>>,
) -> Result<R, AppError> {
    let item = random_callback(&callbacks);
    let sender = item.0.clone();
    let calls = item.1.clone();
    let r = match body {
        None => handle_request(sender, calls, headers, method, vec![]).await,
        Some(v) => match v.0.get("params") {
            None => handle_request(sender, calls, headers, method, vec![]).await,
            Some(v) => {
                let x = v.as_array().unwrap();
                handle_request(sender, calls, headers, method, x.clone()).await
            }
        },
    };
    Ok(r)
}

async fn handle_request(
    ws_tx: mpsc::UnboundedSender<JsonRpcRequest>,
    callbacks: Callbacks,
    headers: HeaderMap,
    method: String,
    params: Vec<Value>,
) -> R {
    let id = get_next_id();
    let addr = maybe_ip_from_headers(&headers);
    info!(
        "{} => {}, {}({:?})",
        &addr, &id, &method, &params
    );
    let (response_tx, response_rx) = oneshot::channel();
    {
        callbacks.write().await.insert(id, response_tx);
    }
    let request = JsonRpcRequest { id, method, params };
    ws_tx.send(request).unwrap();
    match tokio::time::timeout(Duration::from_secs(*RESPONSE_TIMEOUT), response_rx).await {
        Ok(Ok(rep)) => {
            if let Some(result) = rep.result {
                R::ok(result)
            } else if let Some(err) = rep.error {
                let err = err.as_object().unwrap();
                R {
                    success: false,
                    code: err.get("code").cloned(),
                    message: err.get("message").cloned(),
                    response: None,
                    health: None,
                }
            } else {
                R::error(-1, "No response".into())
            }
        }
        Ok(Err(_)) | Err(_) => {
            warn!(
                "{} <= {}, No response received within {} seconds",
                &addr, &id, *RESPONSE_TIMEOUT
            );
            {
                callbacks.write().await.remove(&id);
            }
            R::error(-1, "Response timeout".into())
        }
    }
}

async fn handle_health(
    Extension(callbacks): Extension<Vec<(mpsc::UnboundedSender<JsonRpcRequest>, Callbacks)>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let id = get_next_id();
    let item = random_callback(&callbacks);
    let addr = maybe_ip_from_headers(&headers);
    info!("{} => {}, Detecting server health", &addr, &id);

    let (response_tx, response_rx) = oneshot::channel();
    {
        item.1.write().await.insert(id, response_tx);
    }
    let request = JsonRpcRequest {
        id,
        method: "blockchain.atomicals.get_global".into(),
        params: vec![],
    };
    item.0.send(request).unwrap();
    match tokio::time::timeout(Duration::from_secs(5), response_rx).await {
        Ok(Ok(rep)) => R::health(rep.result.is_some()),
        Ok(Err(_)) | Err(_) => {
            warn!(
                "{} <= {}, Check server health timeout, no response received within 5 seconds",
                &addr, &id
            );
            {
                item.1.write().await.remove(&id);
            }
            R::health(false)
        }
    }
}

async fn handle_proxy() -> impl IntoResponse {
    Json(json!({
        "success": true,
        "info": {
            "note": "Atomicals ElectrumX Digital Object Proxy Online",
            "usageInfo": {
                "note": "The service offers both POST and GET requests for proxying requests to ElectrumX. To handle larger broadcast transaction payloads use the POST method instead of GET.",
                "POST": "POST /proxy/:method with string encoded array in the field \"params\" in the request body. ",
                "GET": "GET /proxy/:method?params=[\"value1\"] with string encoded array in the query argument \"params\" in the URL."
            },
            "healthCheck": "GET /proxy/health",
            "github": "https://github.com/AstroxNetwork/elex-proxy",
            "license": "MIT"
        }
    }))
}

fn handle_panic(err: Box<dyn Any + Send + 'static>) -> http::Response<Full<Bytes>> {
    let details = if let Some(s) = err.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = err.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "Unknown error".to_string()
    };

    let body = R::error(-1, details);
    let body = serde_json::to_string(&body).unwrap();

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::from(body))
        .unwrap()
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_millisecond(*IP_LIMIT_PER_MILLS)
            .burst_size(*IP_LIMIT_BURST_SIZE)
            .key_extractor(SmartIpKeyExtractor)
            .use_headers()
            .finish()
            .unwrap(),
    );
    let mut calls = vec![];
    for i in 0..*ELECTRUMX_WS_INSTANCE {
        let (ws_tx, callbacks, ws_rx_stream) = new_callbacks();
        calls.push((ws_tx, callbacks.clone()));
        try_new_client(i, callbacks, ws_rx_stream);
    }
    let app = Router::new()
        .fallback(|uri: http::Uri| async move {
            let body = R::error(-1, format!("No route: {}", &uri));
            let body = serde_json::to_string(&body).unwrap();
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::from(body))
                .unwrap()
        })
        .route("/", get(|| async { "Hello, Atomicals!" }))
        .route("/proxy", get(handle_proxy).post(handle_proxy))
        .route("/proxy/health", get(handle_health).post(handle_health))
        .route("/proxy/:method", get(handle_get).post(handle_post))
        .layer(GovernorLayer {
            config: Box::leak(governor_conf),
        })
        .layer(ConcurrencyLimitLayer::new(*CONCURRENCY_LIMIT))
        .layer(CatchPanicLayer::custom(handle_panic))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .layer(Extension(calls.clone()));
    let listener = tokio::net::TcpListener::bind((*PROXY_HOST).clone())
        .await
        .unwrap();
    info!("Listening on {}", *PROXY_HOST);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn new_callbacks() -> (
    mpsc::UnboundedSender<JsonRpcRequest>,
    Callbacks,
    Arc<Mutex<UnboundedReceiverStream<JsonRpcRequest>>>,
) {
    let (ws_tx, ws_rx) = mpsc::unbounded_channel::<JsonRpcRequest>();
    let callbacks: Callbacks = Arc::new(RwLock::new(HashMap::new()));
    let ws_rx_stream = Arc::new(Mutex::new(UnboundedReceiverStream::new(ws_rx)));
    (ws_tx, callbacks, ws_rx_stream)
}

fn try_new_client(
    ins: u32,
    callbacks: Callbacks,
    ws_rx_stream: Arc<Mutex<UnboundedReceiverStream<JsonRpcRequest>>>,
) {
    tokio::spawn(async move {
        let list = ELECTRUMX_WSS.split(',').collect::<Vec<&str>>();
        info!("WS-{} ElectrumX WSS: {:?}", ins, &list);
        let mut index = 0;
        loop {
            let wss = list.get(index).unwrap();
            info!("WS-{} Try to connect to ElectrumX: {}", ins, &wss);
            match connect_async(*wss).await {
                Ok((ws, _)) => {
                    info!("WS-{} Connected to ElectrumX: {}", ins, &wss);
                    let (mut write, mut read) = ws.split();
                    let ws_rx_stream = Arc::clone(&ws_rx_stream);
                    let send_handle = tokio::spawn(async move {
                        let mut guard = ws_rx_stream.lock().await;
                        while let Some(message) = guard.next().await {
                            let request_text = serde_json::to_string(&message).unwrap();
                            if let Err(e) = write.send(Message::Text(request_text)).await {
                                error!("WS-{} Failed to send message to ElectrumX: {:?}", ins, e);
                                break;
                            }
                        }
                    });
                    while let Some(Ok(msg)) = read.next().await {
                        if msg.is_text() {
                            if let Ok(text) = msg.to_text() {
                                if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(text) {
                                    if let Some(callback) = callbacks.write().await.remove(&resp.id)
                                    {
                                        info!("WS-{} <= {}, Request matched", ins, &resp.id);
                                        let _ = callback.send(resp);
                                    } else {
                                        warn!("WS-{} <= {}, No matching request found", ins, &resp.id);
                                    }
                                } else {
                                    error!("WS-{} Failed to parse ws response: {}", ins, text);
                                }
                            }
                        } else if msg.is_close() {
                            warn!("WS-{} Connection closed: {}", ins, &wss);
                            // Close the send handle to stop the send task.
                            if !send_handle.is_finished() {
                                send_handle.abort();
                            }
                            break;
                        }
                    }
                }
                Err(e) => {
                    error!("WS-{} Failed to connect to ElectrumX: {:?}", ins, e);
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            }
            if index >= list.len() - 1 {
                index = 0;
            } else {
                index += 1;
            }
        }
    });
}
