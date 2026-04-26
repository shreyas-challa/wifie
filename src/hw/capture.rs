//! Shared types for telemetry + handshake captures.
//!
//! Frames flow from a blocking pcap thread into the async runtime via an
//! mpsc channel. Both telemetry aggregation (M1) and handshake EAPOL
//! filtering (M2) consume the same frame shape.

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// Ethernet payload — when the radio isn't in monitor mode (rare for
    /// us, but worth modelling so the parser doesn't blindly assume
    /// radiotap headers exist).
    Ethernet,
    /// 802.11 with radiotap headers (the normal monitor-mode case).
    IeeeWithRadiotap,
    /// Plain 802.11, no radiotap.
    Ieee80211,
    /// Anything else; we still pass the bytes through but skip parsers
    /// that depend on a known link layer.
    Other,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CapturedFrame {
    /// Unix epoch ms when pcap_next_ex returned the frame. Used by the
    /// handshake pipeline (M2) to stamp the artifact pcap.
    pub timestamp_ms: i64,
    /// Read by handshake capture to decide whether radiotap headers are
    /// present before parsing.
    pub link_type: LinkType,
    pub raw: Vec<u8>,
}

/// Hands a capture worker its lifecycle: the receiver delivers frames
/// while the worker is alive, and dropping the [`CaptureSession`] (or
/// hitting the stop signal) tears it down.
#[allow(dead_code)]
pub struct CaptureSession {
    /// Read by callers that want to log which interface owns the session.
    pub interface: String,
    pub link_type: LinkType,
    pub frames: mpsc::Receiver<CapturedFrame>,
    pub stop: oneshot::Sender<()>,
}

/// Single broadcast tick the dashboard plots. Aggregated from raw frames
/// in 100 ms windows.
#[derive(Debug, Clone, Serialize)]
pub struct TelemetryTick {
    pub seq: u64,
    pub timestamp_ms: i64,
    pub packets_per_second: u32,
    pub noise_floor_dbm: Option<i32>,
    pub rssi_dbm: Option<i32>,
}
