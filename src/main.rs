use std::any::Any;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use axum::body::Body;
use axum::error_handling::HandleErrorLayer;
use axum::extract::{Path, Query};
use axum::extract::Extension;
use axum::extract::Json;
use axum::http;
use axum::http::header;
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
use serde::{Deserialize, Serialize};
use serde_json::{json, Number, Value};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tower::{BoxError, ServiceBuilder};
use tower::limit::ConcurrencyLimitLayer;
use tower_governor::errors::display_error;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};

#[derive(Serialize, Deserialize)]
struct JsonRpcRequest {
    method: String,
    params: Vec<Value>,
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<Value>,
    id: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct R {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<Value>,
}

impl R {
    fn ok(payload: Value) -> Self {
        Self {
            success: true,
            response: Some(payload),
            code: None,
            message: None,
        }
    }
    fn error(code: i32, message: String) -> Self {
        Self {
            success: false,
            response: None,
            code: Some(Value::Number(Number::from(code))),
            message: Some(Value::String(message)),
        }
    }
}

static ID_COUNTER: Lazy<AtomicU64> = Lazy::new(|| AtomicU64::new(0));

fn get_next_id() -> u64 {
    ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}

type Callbacks = Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>;

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let value = R {
            success: false,
            code: None,
            message: Some(Value::String(self.0.to_string())),
            response: None,
        };
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
    Extension(callbacks): Extension<Callbacks>,
    Extension(ws_tx): Extension<mpsc::UnboundedSender<Message>>,
    Path(method): Path<String>,
    Query(query): Query<Value>,
) -> Result<R, AppError> {
    let x = query.get("params").unwrap().as_str().unwrap();
    let params = serde_json::from_str(x).unwrap();
    let r = handle_request(method, params, callbacks, ws_tx).await;
    Ok(r)
}

async fn handle_post(
    Extension(callbacks): Extension<Callbacks>,
    Extension(ws_tx): Extension<mpsc::UnboundedSender<Message>>,
    Path(method): Path<String>,
    Json(body): Json<Value>,
) -> Result<R, AppError> {
    let x = body.get("params").unwrap().as_array().unwrap();
    let r = handle_request(method, x.clone(), callbacks, ws_tx).await;
    Ok(r)
}

async fn handle_request(
    method: String,
    params: Vec<Value>,
    callbacks: Callbacks,
    ws_tx: mpsc::UnboundedSender<Message>,
) -> R {
    let id = get_next_id();
    info!("=> id: {}, method: {}, params: {:?}", &id, &method, &params);

    let (response_tx, response_rx) = oneshot::channel();
    {
        callbacks.write().await.insert(id, response_tx);
    }
    let request = JsonRpcRequest {
        id,
        method,
        params,
    };
    let request_text = serde_json::to_string(&request).unwrap();
    ws_tx.send(Message::Text(request_text)).unwrap();
    match tokio::time::timeout(Duration::from_secs(10), response_rx).await {
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
                }
            } else {
                R::error(-1, "No response".into())
            }
        }
        Ok(Err(_)) | Err(_) => {
            {
                callbacks.write().await.remove(&id);
            }
            R::error(-1, "Read timeout".into())
        }
    }
}

async fn handle_proxy() -> impl IntoResponse {
    Json(json!(
    {
        "success": true,
        "info": {
            "note": "Atomicals ElectrumX Digital Object Proxy Online",
            "usageInfo": {
                "note": "The service offers both POST and GET requests for proxying requests to ElectrumX. To handle larger broadcast transaction payloads use the POST method instead of GET.",
                "POST": "POST /proxy/:method with string encoded array in the field \"params\" in the request body. ",
                "GET": "GET /proxy/:method?params=[\"value1\"] with string encoded array in the query argument \"params\" in the URL."
            },
            "healthCheck": "GET /proxy/health",
            "github": "https://github.com/atomicals/electrumx-proxy",
            "license": "MIT"
        }
    }
            ))
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
    let (ws_tx, ws_rx) = mpsc::unbounded_channel::<Message>();
    let callbacks: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let ws_rx_stream = Arc::new(Mutex::new(UnboundedReceiverStream::new(ws_rx)));
    let governor_conf = Box::new(
        GovernorConfigBuilder::default()
            .per_second(env::var("IP_LIMIT_PER_SECOND").unwrap_or("1".to_string()).parse().unwrap())
            .burst_size(env::var("IP_LIMIT_BURST_SIZE").unwrap_or("10".to_string()).parse().unwrap())
            .finish()
            .unwrap(),
    );
    let app = Router::new()
        .route("/", get(|| async { "Hello, Atomicals!" }))
        .route("/proxy", get(handle_proxy).post(handle_proxy))
        .route("/proxy/:method", get(handle_get).post(handle_post))
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|e: BoxError| async move {
                    display_error(e)
                }))
                .layer(CatchPanicLayer::custom(handle_panic))
                .layer(TraceLayer::new_for_http())
                .layer(GovernorLayer {
                    config: Box::leak(governor_conf),
                })
        )
        .layer(ConcurrencyLimitLayer::new(
            env::var("CONCURRENCY_LIMIT").unwrap_or("500".to_string()).parse().unwrap(),
        ))
        .layer(CorsLayer::permissive())
        .layer(Extension(callbacks.clone()))
        .layer(Extension(ws_tx.clone()));

    tokio::spawn(async move {
        let wss_var = env::var("ELECTRUMX_WSS").unwrap_or("wss://electrumx.atomicals.xyz:50012".to_string());
        let list = wss_var.split(',').collect::<Vec<&str>>();
        info!("ElectrumX WSS: {:?}", &list);
        let mut index = 0;
        loop {
            let wss = list.get(index).unwrap();
            info!("Try to connect to electrumx: {}", &wss);
            match connect_async(*wss).await {
                Ok((ws, _)) => {
                    info!("Connected to electrumx");
                    let (mut write, mut read) = ws.split();
                    let ws_rx_stream = Arc::clone(&ws_rx_stream);
                    tokio::spawn(async move {
                        let mut guard = ws_rx_stream.lock().await;
                        while let Some(message) = guard.next().await {
                            let _ = write.send(message).await;
                        }
                    });
                    while let Some(Ok(msg)) = read.next().await {
                        if let Ok(text) = msg.to_text() {
                            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(text) {
                                info!(
                                    "<= id: {}, success: {}",
                                    &response.id,
                                    response.result.is_some()
                                );
                                if let Some(callback) = callbacks.write().await.remove(&response.id)
                                {
                                    let _ = callback.send(response);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to connect to electrumx: {:?}", e);
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
    let app_api = env::var("PROXY_HOST").unwrap_or("0.0.0.0:12321".to_string());
    let listener = tokio::net::TcpListener::bind(&app_api)
        .await
        .unwrap();
    info!("Listening on {}", &app_api);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.unwrap();
}
