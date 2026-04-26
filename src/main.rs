use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

mod hw;

use hw::{ChannelWidth, HwError, InterfaceMode, SharedBackend};

#[derive(Clone)]
struct AppState {
    backend: SharedBackend,
    captures: Arc<RwLock<HashMap<Uuid, CaptureTask>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CaptureType {
    Wpa2Handshake,
    Wpa3Pmkid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Pending,
    Running,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaptureTask {
    id: Uuid,
    interface: String,
    target_bssid: String,
    capture_type: CaptureType,
    status: TaskStatus,
    created_at: DateTime<Utc>,
    artifact_path: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MonitorModeRequest {
    interface: String,
    enable: bool,
}

#[derive(Debug, Deserialize)]
struct ChannelRequest {
    interface: String,
    frequency_mhz: u32,
    /// Optional channel width override; defaults to a sane value for the band.
    #[serde(default)]
    width: Option<ChannelWidth>,
}

#[derive(Debug, Deserialize)]
struct HandshakeCaptureRequest {
    interface: String,
    target_bssid: String,
    target_client: Option<String>,
    channel_mhz: u32,
    capture_type: CaptureType,
}

#[derive(Debug, Deserialize)]
struct VulnerabilityTestRequest {
    interface: String,
    target_bssid: String,
    test_case: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info,wifie_server=debug")
        .init();

    let backend = match hw::select_from_env().await {
        Ok(b) => {
            info!(backend = b.name(), "wireless backend ready");
            b
        }
        Err(err) => {
            warn!(
                error = %err,
                "real wireless backend unavailable; falling back to mock backend"
            );
            Arc::new(hw::mock::MockBackend::seeded()) as SharedBackend
        }
    };

    let state = AppState {
        backend,
        captures: Arc::new(RwLock::new(HashMap::new())),
    };

    let (io_layer, io) = SocketIo::builder().build_layer();
    io.ns("/events", on_socket_connect);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/interfaces", get(list_interfaces))
        .route("/api/interfaces/monitor-mode", post(toggle_monitor_mode))
        .route("/api/interfaces/channel", post(set_channel))
        .route("/api/captures/handshake", post(start_handshake_capture))
        .route("/api/vuln-tests/start", post(start_vuln_test_stub))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(io_layer);

    spawn_telemetry(io);

    let addr: SocketAddr = "0.0.0.0:3000".parse()?;
    info!(%addr, "listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn on_socket_connect(socket: SocketRef) {
    info!(sid = %socket.id, "socket connected");
    let ready = serde_json::json!({ "ok": true });
    socket.emit("server:ready", &ready).ok();

    socket.on(
        "client:ping",
        |socket: SocketRef, Data::<serde_json::Value>(payload)| {
            socket
                .emit(
                    "server:pong",
                    &serde_json::json!({
                        "echo": payload,
                        "ts": Utc::now().to_rfc3339(),
                    }),
                )
                .ok();
        },
    );
}

fn spawn_telemetry(io: SocketIo) {
    tokio::spawn(async move {
        let mut counter: u64 = 0;
        loop {
            counter += 1;
            if let Some(ns) = io.of("/events") {
                ns.emit(
                    "packet:tick",
                    &serde_json::json!({
                        "seq": counter,
                        "packets_per_second": 700 + ((counter % 100) as i32),
                        "noise_floor_dbm": -95 + ((counter % 5) as i32),
                        "timestamp": Utc::now().timestamp_millis(),
                    }),
                )
                .ok();
            }
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    });
}

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "ok",
        "service": "wifie-server",
        "backend": state.backend.name(),
    }))
}

async fn list_interfaces(State(state): State<AppState>) -> impl IntoResponse {
    match state.backend.list_interfaces().await {
        Ok(list) => Json(serde_json::json!(list)),
        Err(err) => {
            error!(?err, "list_interfaces failed");
            Json(serde_json::json!({ "error": err.to_string(), "interfaces": [] }))
        }
    }
}

async fn toggle_monitor_mode(
    State(state): State<AppState>,
    Json(req): Json<MonitorModeRequest>,
) -> impl IntoResponse {
    let mode = if req.enable {
        InterfaceMode::Monitor
    } else {
        InterfaceMode::Managed
    };
    match state.backend.set_mode(&req.interface, mode).await {
        Ok(()) => Json(serde_json::json!({ "ok": true, "interface": req.interface, "mode": mode })),
        Err(err) => err_to_json(err),
    }
}

async fn set_channel(
    State(state): State<AppState>,
    Json(req): Json<ChannelRequest>,
) -> impl IntoResponse {
    let width = req
        .width
        .unwrap_or_else(|| ChannelWidth::default_for_freq(req.frequency_mhz));
    match state
        .backend
        .set_channel(&req.interface, req.frequency_mhz, width)
        .await
    {
        Ok(()) => Json(serde_json::json!({
            "ok": true,
            "interface": req.interface,
            "frequency_mhz": req.frequency_mhz,
            "width": width,
        })),
        Err(err) => err_to_json(err),
    }
}

async fn start_handshake_capture(
    State(state): State<AppState>,
    Json(req): Json<HandshakeCaptureRequest>,
) -> impl IntoResponse {
    let id = Uuid::new_v4();
    let task = CaptureTask {
        id,
        interface: req.interface,
        target_bssid: req.target_bssid,
        capture_type: req.capture_type,
        status: TaskStatus::Pending,
        created_at: Utc::now(),
        artifact_path: None,
        error: None,
    };

    if req.target_client.is_none() {
        warn!("capture requested without target_client, running in passive mode");
    }
    let _channel_hint = req.channel_mhz;

    state.captures.write().await.insert(id, task.clone());
    Json(serde_json::json!({ "ok": true, "task": task }))
}

async fn start_vuln_test_stub(Json(req): Json<VulnerabilityTestRequest>) -> impl IntoResponse {
    Json(serde_json::json!({
        "ok": true,
        "message": "stub_only_no_active_attack_logic",
        "interface": req.interface,
        "target_bssid": req.target_bssid,
        "test_case": req.test_case
    }))
}

fn err_to_json(err: HwError) -> Json<serde_json::Value> {
    let kind = match &err {
        HwError::InterfaceNotFound(_) => "interface_not_found",
        HwError::PermissionDenied => "permission_denied",
        HwError::Unavailable(_) => "backend_unavailable",
        HwError::Other(_) => "backend_error",
    };
    Json(serde_json::json!({
        "ok": false,
        "error": kind,
        "message": err.to_string(),
    }))
}
