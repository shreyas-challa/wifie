//! Hardware abstraction for wireless adapters.
//!
//! `WirelessBackend` is the single seam between the HTTP/WS layer and any
//! actual radio. The mock backend powers demos and CI; the netlink backend
//! drives real Wi-Fi adapters via nl80211. New backends (USRP, virtual sniffer
//! pcap replays, remote agents) plug in by implementing the same trait.

pub mod mock;

#[cfg(target_os = "linux")]
pub mod link;
#[cfg(target_os = "linux")]
pub mod netlink;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceMode {
    Managed,
    Monitor,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChannelWidth {
    Width20,
    Width40,
    Width80,
    Width160,
    Width320,
}

impl ChannelWidth {
    /// Pick a sane default width for a given centre frequency.
    /// 6 GHz / Wi-Fi 7 favors 160 MHz, 5 GHz uses 80, 2.4 GHz stays 20 to
    /// avoid wedging the whole band.
    pub fn default_for_freq(freq_mhz: u32) -> Self {
        match freq_mhz {
            f if f >= 5955 => ChannelWidth::Width160,
            f if f >= 5170 => ChannelWidth::Width80,
            _ => ChannelWidth::Width20,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WirelessInterface {
    pub name: String,
    pub phy: String,
    pub if_index: u32,
    pub mac: String,
    pub mode: InterfaceMode,
    pub supported_bands_ghz: Vec<f32>,
    pub frequency_mhz: Option<u32>,
    pub channel_width: Option<ChannelWidth>,
    pub ssid: Option<String>,
}

#[derive(Debug, Error)]
pub enum HwError {
    #[error("interface not found: {0}")]
    InterfaceNotFound(String),
    #[error("operation not permitted (need CAP_NET_ADMIN / CAP_NET_RAW; try sudo or setcap)")]
    PermissionDenied,
    // Constructed on non-Linux targets when the real backend is requested.
    #[allow(dead_code)]
    #[error("backend unavailable: {0}")]
    Unavailable(String),
    #[error("backend error: {0}")]
    Other(String),
}

pub type HwResult<T> = Result<T, HwError>;

#[async_trait]
pub trait WirelessBackend: Send + Sync {
    /// Human label for telemetry / logs (e.g. "mock", "nl80211").
    fn name(&self) -> &'static str;

    async fn list_interfaces(&self) -> HwResult<Vec<WirelessInterface>>;

    async fn set_mode(&self, interface: &str, mode: InterfaceMode) -> HwResult<()>;

    async fn set_channel(
        &self,
        interface: &str,
        frequency_mhz: u32,
        width: ChannelWidth,
    ) -> HwResult<()>;
}

/// Shared handle the HTTP layer holds; lets us swap backends without changing
/// route signatures.
pub type SharedBackend = Arc<dyn WirelessBackend>;

/// Selects a backend from the `WIFIE_HW` env var.
///
/// - `mock` always returns the in-memory mock backend.
/// - `real` (or unset, on Linux) tries to connect to nl80211; on failure the
///   caller decides whether to fall back. We never silently lie about the
///   backend in use — the chosen name is logged.
pub async fn select_from_env() -> HwResult<SharedBackend> {
    let kind = std::env::var("WIFIE_HW").unwrap_or_else(|_| default_backend_kind().to_string());
    match kind.to_ascii_lowercase().as_str() {
        "mock" => Ok(Arc::new(mock::MockBackend::seeded())),
        "real" | "nl80211" | "netlink" => connect_real_backend().await,
        other => Err(HwError::Other(format!(
            "unknown WIFIE_HW value: '{other}' (expected mock|real)"
        ))),
    }
}

fn default_backend_kind() -> &'static str {
    if cfg!(target_os = "linux") {
        "real"
    } else {
        "mock"
    }
}

#[cfg(target_os = "linux")]
async fn connect_real_backend() -> HwResult<SharedBackend> {
    let backend = netlink::NetlinkBackend::connect().await?;
    Ok(Arc::new(backend))
}

#[cfg(not(target_os = "linux"))]
async fn connect_real_backend() -> HwResult<SharedBackend> {
    Err(HwError::Unavailable(
        "real nl80211 backend is only available on Linux".into(),
    ))
}
