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
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Clone, Default)]
struct AppState {
    interfaces: Arc<RwLock<Vec<WirelessInterface>>>,
    captures: Arc<RwLock<HashMap<Uuid, CaptureTask>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum InterfaceMode {
    Managed,
    Monitor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WirelessInterface {
    name: String,
    phy: String,
    mac: String,
    mode: InterfaceMode,
    supported_bands_ghz: Vec<f32>,
    channel_mhz: Option<u16>,
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
    frequency_mhz: u16,
}

#[derive(Debug, Deserialize)]
struct HandshakeCaptureRequest {
    interface: String,
    target_bssid: String,
    target_client: Option<String>,
    channel_mhz: u16,
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

    // Placeholder bootstrap demonstrating where netlink_wi and pcap initialize.
    // TODO: wire actual netlink_wi::NlSocket and interface enumeration.
    info!("starting WiFie backend (Rust + nl80211 + pcap)");

    let state = AppState {
        interfaces: Arc::new(RwLock::new(seed_interfaces())),
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

    socket.on("client:ping", |socket: SocketRef, Data::<serde_json::Value>(payload)| {
        socket
            .emit(
                "server:pong",
                &serde_json::json!({
                    "echo": payload,
                    "ts": Utc::now().to_rfc3339(),
                }),
            )
            .ok();
    });
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
            ).ok();
            }
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    });
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "wifie-server" }))
}

async fn list_interfaces(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.interfaces.read().await.clone())
}

async fn toggle_monitor_mode(
    State(state): State<AppState>,
    Json(req): Json<MonitorModeRequest>,
) -> impl IntoResponse {
    let mut interfaces = state.interfaces.write().await;
    if let Some(iface) = interfaces.iter_mut().find(|i| i.name == req.interface) {
        // TODO: replace with netlink_wi call to switch interface mode.
        iface.mode = if req.enable {
            InterfaceMode::Monitor
        } else {
            InterfaceMode::Managed
        };
        return Json(serde_json::json!({ "ok": true, "interface": iface }));
    }

    Json(serde_json::json!({ "ok": false, "error": "interface_not_found" }))
}

async fn set_channel(
    State(state): State<AppState>,
    Json(req): Json<ChannelRequest>,
) -> impl IntoResponse {
    let mut interfaces = state.interfaces.write().await;
    if let Some(iface) = interfaces.iter_mut().find(|i| i.name == req.interface) {
        // TODO: replace with netlink_wi channel/frequency API for nl80211.
        iface.channel_mhz = Some(req.frequency_mhz);
        return Json(serde_json::json!({ "ok": true, "interface": iface }));
    }

    Json(serde_json::json!({ "ok": false, "error": "interface_not_found" }))
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

    // NOTE: This intentionally does not implement frame injection/deauthentication.
    // It is a safe placeholder for authorized lab integration with pcap/netlink_wi.
    if req.target_client.is_none() {
        warn!("capture requested without target_client, running in passive mode");
    }
    let _channel_hint = req.channel_mhz;

    state.captures.write().await.insert(id, task.clone());
    Json(serde_json::json!({ "ok": true, "task": task }))
}

async fn start_vuln_test_stub(Json(req): Json<VulnerabilityTestRequest>) -> impl IntoResponse {
    // Safety-first stub: this endpoint only records intent for controlled, ethical testing.
    Json(serde_json::json!({
        "ok": true,
        "message": "stub_only_no_active_attack_logic",
        "interface": req.interface,
        "target_bssid": req.target_bssid,
        "test_case": req.test_case
    }))
}

fn seed_interfaces() -> Vec<WirelessInterface> {
    vec![WirelessInterface {
        name: "wlan0".to_string(),
        phy: "phy0".to_string(),
        mac: "02:11:22:33:44:55".to_string(),
        mode: InterfaceMode::Managed,
        supported_bands_ghz: vec![2.4, 5.0, 6.0],
        channel_mhz: Some(2412),
    }]
}
