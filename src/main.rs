use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use radiotap::Radiotap;
use serde::Deserialize;
use socketioxide::{
    extract::{Data, SocketRef},
    SocketIo,
};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use tracing::{error, info, warn};
use uuid::Uuid;

mod auth;
mod handshake;
mod hw;
mod vuln;

use handshake::{CaptureRegistry, CaptureType, CaptureTask};
use hw::{
    capture::{CapturedFrame, LinkType, TelemetryTick},
    ChannelWidth, HwError, InterfaceMode, SharedBackend,
};

const TELEMETRY_WINDOW_MS: u64 = 100;

#[derive(Clone)]
struct AppState {
    backend: SharedBackend,
    captures: CaptureRegistry,
    telemetry: Arc<Mutex<Option<TelemetrySession>>>,
    io: SocketIo,
    seq: Arc<AtomicU64>,
}

struct TelemetrySession {
    interface: String,
    handle: JoinHandle<()>,
    /// Held to keep the capture alive; dropping it stops the worker.
    _stop: tokio::sync::oneshot::Sender<()>,
}

impl TelemetrySession {
    async fn shutdown(self) {
        // Drop _stop by destructuring; the worker pcap thread will exit
        // on its next 200 ms timeout, then this join returns.
        let TelemetrySession { handle, .. } = self;
        handle.abort();
        let _ = handle.await;
    }
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
    #[serde(default)]
    width: Option<ChannelWidth>,
}

#[derive(Debug, Deserialize)]
struct HandshakeCaptureRequest {
    interface: String,
    target_bssid: String,
    target_client: Option<String>,
    #[serde(default)]
    channel_mhz: Option<u32>,
    capture_type: CaptureType,
    /// If true, fire `count` deauth frames at the target right after the
    /// capture worker is up. Defaults to false because deauth is destructive.
    #[serde(default)]
    deauth: bool,
    #[serde(default = "default_deauth_count")]
    deauth_count: u8,
}

fn default_deauth_count() -> u8 {
    3
}

#[derive(Debug, Deserialize)]
struct DeauthRequest {
    interface: String,
    bssid: String,
    #[serde(default)]
    client: Option<String>,
    #[serde(default = "default_deauth_count")]
    count: u8,
}

#[derive(Debug, Deserialize)]
struct VulnerabilityTestRequest {
    interface: String,
    target_bssid: String,
    test_case: vuln::TestCase,
}

#[derive(Debug, Deserialize)]
struct TelemetryStartRequest {
    interface: String,
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
            warn!(error = %err, "real wireless backend unavailable; falling back to mock backend");
            Arc::new(hw::mock::MockBackend::seeded()) as SharedBackend
        }
    };

    let (io_layer, io) = SocketIo::builder().build_layer();
    io.ns("/events", on_socket_connect);

    let state = AppState {
        backend,
        captures: handshake::load_persisted(),
        telemetry: Arc::new(Mutex::new(None)),
        io: io.clone(),
        seq: Arc::new(AtomicU64::new(0)),
    };

    let mut app = Router::new()
        .route("/health", get(health))
        .route("/api/interfaces", get(list_interfaces))
        .route("/api/interfaces/monitor-mode", post(toggle_monitor_mode))
        .route("/api/interfaces/channel", post(set_channel))
        .route("/api/telemetry/start", post(start_telemetry))
        .route("/api/telemetry/stop", post(stop_telemetry))
        .route("/api/captures/handshake", get(list_captures).post(start_handshake_capture))
        .route("/api/captures/handshake/:id", get(get_capture))
        .route("/api/captures/deauth", post(send_deauth))
        .route("/api/auth/lab-bssids", get(list_authorized_bssids))
        .route("/api/vuln-tests/start", post(start_vuln_test_stub))
        .with_state(state);

    // Production mode: serve the built SPA from the same port. Vite is
    // not in the loop. WIFIE_SERVE_DIR points at frontend/dist (or any
    // directory holding index.html + assets/).
    if let Ok(dir) = std::env::var("WIFIE_SERVE_DIR") {
        let path = std::path::PathBuf::from(&dir);
        if !path.join("index.html").exists() {
            warn!(
                serve_dir = %dir,
                "WIFIE_SERVE_DIR is set but index.html not found; SPA will not be served"
            );
        } else {
            info!(serve_dir = %dir, "serving SPA bundle (production mode)");
            let serve = ServeDir::new(path).append_index_html_on_directories(true);
            app = app.fallback_service(serve);
        }
    }

    let app = app
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(io_layer);

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

async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let telemetry_iface = state
        .telemetry
        .lock()
        .await
        .as_ref()
        .map(|s| s.interface.clone());
    Json(serde_json::json!({
        "status": "ok",
        "service": "wifie-server",
        "backend": state.backend.name(),
        "telemetry_interface": telemetry_iface,
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

    // If we're flipping the active telemetry source, stop it first so
    // the link-cycle inside set_mode doesn't fail with "device busy in
    // pcap" on stricter drivers.
    {
        let mut guard = state.telemetry.lock().await;
        if guard
            .as_ref()
            .map(|s| s.interface == req.interface)
            .unwrap_or(false)
        {
            if let Some(session) = guard.take() {
                session.shutdown().await;
            }
        }
    }

    if let Err(err) = state.backend.set_mode(&req.interface, mode).await {
        return err_to_json(err);
    }

    // Auto-bind telemetry on monitor enable; stop it on monitor disable.
    if req.enable {
        if let Err(err) = bind_telemetry(&state, &req.interface).await {
            warn!(error = %err, "auto-bind telemetry after monitor enable failed");
        }
    }

    Json(serde_json::json!({
        "ok": true,
        "interface": req.interface,
        "mode": mode,
    }))
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

async fn start_telemetry(
    State(state): State<AppState>,
    Json(req): Json<TelemetryStartRequest>,
) -> impl IntoResponse {
    match bind_telemetry(&state, &req.interface).await {
        Ok(()) => Json(serde_json::json!({ "ok": true, "interface": req.interface })),
        Err(err) => err_to_json(err),
    }
}

async fn stop_telemetry(State(state): State<AppState>) -> impl IntoResponse {
    let mut guard = state.telemetry.lock().await;
    let was = guard.take().map(|s| s.interface.clone());
    if let Some(session) = guard.take() {
        session.shutdown().await;
    }
    drop(guard);
    Json(serde_json::json!({ "ok": true, "stopped": was }))
}

async fn bind_telemetry(state: &AppState, interface: &str) -> HwResult<()> {
    let session = state.backend.start_capture(interface, None).await?;
    let frames = session.frames;
    let link_type = session.link_type;
    let interface_owned = interface.to_string();
    let io = state.io.clone();
    let seq = state.seq.clone();

    let handle = tokio::spawn(async move {
        forward_telemetry(io, seq, link_type, frames).await;
    });

    let mut guard = state.telemetry.lock().await;
    if let Some(prev) = guard.take() {
        prev.shutdown().await;
    }
    *guard = Some(TelemetrySession {
        interface: interface_owned,
        handle,
        _stop: session.stop,
    });
    Ok(())
}

async fn forward_telemetry(
    io: SocketIo,
    seq: Arc<AtomicU64>,
    link_type: LinkType,
    mut frames: tokio::sync::mpsc::Receiver<CapturedFrame>,
) {
    let mut window_count: u32 = 0;
    let mut last_noise: Option<i32> = None;
    let mut last_rssi: Option<i32> = None;
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(TELEMETRY_WINDOW_MS));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    loop {
        tokio::select! {
            maybe_frame = frames.recv() => {
                match maybe_frame {
                    Some(frame) => {
                        window_count += 1;
                        if matches!(link_type, LinkType::IeeeWithRadiotap) {
                            if let Ok(rt) = Radiotap::from_bytes(&frame.raw) {
                                if let Some(n) = rt.antenna_noise { last_noise = Some(n.value as i32); }
                                if let Some(s) = rt.antenna_signal { last_rssi = Some(s.value as i32); }
                            }
                        }
                    }
                    None => break, // capture stopped
                }
            }
            _ = tick.tick() => {
                let pps = window_count.saturating_mul((1000 / TELEMETRY_WINDOW_MS) as u32);
                window_count = 0;
                let s = seq.fetch_add(1, Ordering::Relaxed) + 1;
                let payload = TelemetryTick {
                    seq: s,
                    timestamp_ms: Utc::now().timestamp_millis(),
                    packets_per_second: pps,
                    noise_floor_dbm: last_noise,
                    rssi_dbm: last_rssi,
                };
                if let Some(ns) = io.of("/events") {
                    ns.emit("packet:tick", &payload).ok();
                }
            }
        }
    }
}

async fn start_handshake_capture(
    State(state): State<AppState>,
    Json(req): Json<HandshakeCaptureRequest>,
) -> Json<serde_json::Value> {
    if !auth::is_authorized(&req.target_bssid) {
        return auth_denied(&req.target_bssid);
    }
    if req.target_client.is_none() {
        warn!("handshake capture requested without target_client (passive only)");
    }

    // Optional pre-capture channel pin. We don't fail the whole request
    // if this errors — the operator can still try to capture; they just
    // won't be locked to the AP's channel.
    if let Some(freq) = req.channel_mhz {
        let width = ChannelWidth::default_for_freq(freq);
        if let Err(err) = state.backend.set_channel(&req.interface, freq, width).await {
            warn!(error = %err, "pre-capture set_channel failed; continuing");
        }
    }

    let task = match handshake::spawn(
        state.backend.clone(),
        state.captures.clone(),
        req.interface.clone(),
        req.target_bssid.clone(),
        req.capture_type,
    )
    .await
    {
        Ok(t) => t,
        Err(err) => {
            return Json(serde_json::json!({
                "ok": false,
                "error": "spawn_failed",
                "message": err,
            }));
        }
    };

    if req.deauth {
        let interface = req.interface.clone();
        let bssid = req.target_bssid.clone();
        let client = req.target_client.clone();
        let count = req.deauth_count;
        // Fire-and-forget; the result is logged. The capture task will
        // pick up the resulting EAPOL frames if the client reconnects.
        tokio::spawn(async move {
            // Brief pause so the pcap worker is reading by the time the
            // deauths actually go out.
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            match handshake::deauth(&interface, &bssid, client.as_deref(), count).await {
                Ok(out) => info!(target = %bssid, %count, "deauth sent: {}", out.trim()),
                Err(err) => warn!(target = %bssid, "deauth failed: {err}"),
            }
        });
    }

    Json(serde_json::json!({ "ok": true, "task": task }))
}

async fn list_captures(State(state): State<AppState>) -> Json<serde_json::Value> {
    let mut tasks: Vec<CaptureTask> = state.captures.read().await.values().cloned().collect();
    tasks.sort_by_key(|t| std::cmp::Reverse(t.created_at));
    Json(serde_json::json!(tasks))
}

async fn get_capture(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Json<serde_json::Value> {
    match state.captures.read().await.get(&id).cloned() {
        Some(t) => Json(serde_json::json!(t)),
        None => Json(serde_json::json!({
            "ok": false,
            "error": "not_found",
            "message": format!("no capture task with id {id}"),
        })),
    }
}

async fn send_deauth(Json(req): Json<DeauthRequest>) -> Json<serde_json::Value> {
    if !auth::is_authorized(&req.bssid) {
        return auth_denied(&req.bssid);
    }
    match handshake::deauth(&req.interface, &req.bssid, req.client.as_deref(), req.count).await {
        Ok(stdout) => Json(serde_json::json!({ "ok": true, "stdout": stdout })),
        Err(err) => Json(serde_json::json!({
            "ok": false,
            "error": "deauth_failed",
            "message": err,
        })),
    }
}

async fn list_authorized_bssids() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "env_var": auth::env_var_name(),
        "bssids": auth::authorized_list(),
    }))
}

fn auth_denied(target: &str) -> Json<serde_json::Value> {
    let env = auth::env_var_name();
    warn!(target, "rejected: BSSID not in {env}");
    Json(serde_json::json!({
        "ok": false,
        "error": "not_authorized",
        "message": format!(
            "target BSSID '{target}' is not in {env}. \
             Set the env var to a comma-separated allowlist before driving offensive operations."
        ),
        "env_var": env,
    }))
}

async fn start_vuln_test_stub(Json(req): Json<VulnerabilityTestRequest>) -> Json<serde_json::Value> {
    if !auth::is_authorized(&req.target_bssid) {
        return auth_denied(&req.target_bssid);
    }
    match vuln::run(req.test_case, &req.interface, &req.target_bssid).await {
        Ok(result) => Json(serde_json::json!({ "ok": true, "result": result })),
        Err(err) => {
            let kind = if err.starts_with("tool_missing") {
                "tool_missing"
            } else {
                "vuln_test_failed"
            };
            Json(serde_json::json!({
                "ok": false,
                "error": kind,
                "message": err,
            }))
        }
    }
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

type HwResult<T> = Result<T, HwError>;
