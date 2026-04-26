//! Real nl80211 backend, powered by `netlink_wi::AsyncNlSocket`.
//!
//! Translates the crate's types into the neutral `WirelessInterface` /
//! `InterfaceMode` / `ChannelWidth` shapes so HTTP routes never see vendor
//! types.

use async_trait::async_trait;
use chrono::Utc;
use netlink_wi::{
    interface::{
        ChannelWidth as NlChannelWidth, InterfaceType as NlInterfaceType,
        WirelessInterface as NlWirelessInterface,
    },
    AsyncNlSocket, ChannelConfig, NlError,
};
use pcap::{Capture, Linktype};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, warn};

use super::{
    capture::{CaptureSession, CapturedFrame, LinkType},
    link::LinkController,
    ChannelWidth, HwError, HwResult, InterfaceMode, WirelessBackend, WirelessInterface,
};

pub struct NetlinkBackend {
    socket: AsyncNlSocket,
    link: LinkController,
}

impl NetlinkBackend {
    pub async fn connect() -> HwResult<Self> {
        let socket = AsyncNlSocket::connect().await.map_err(translate_err)?;
        let link = LinkController::connect()?;
        Ok(Self { socket, link })
    }

    async fn find_if_index(&self, name: &str) -> HwResult<u32> {
        let interfaces = self
            .socket
            .list_interfaces()
            .await
            .map_err(translate_err)?;
        interfaces
            .into_iter()
            .find(|i| i.name == name)
            .map(|i| i.interface_index)
            .ok_or_else(|| HwError::InterfaceNotFound(name.to_string()))
    }
}

#[async_trait]
impl WirelessBackend for NetlinkBackend {
    fn name(&self) -> &'static str {
        "nl80211"
    }

    async fn list_interfaces(&self) -> HwResult<Vec<WirelessInterface>> {
        let raw = self
            .socket
            .list_interfaces()
            .await
            .map_err(translate_err)?;
        Ok(raw.into_iter().map(translate_interface).collect())
    }

    async fn set_mode(&self, interface: &str, mode: InterfaceMode) -> HwResult<()> {
        let if_index = self.find_if_index(interface).await?;
        let if_type = mode_to_nl(mode);

        // Most Wi-Fi drivers reject SET_INTERFACE while the netdev is UP
        // with EBUSY. Cycle the link around the call. If bringing it down
        // fails we still try the mode change — some drivers accept it
        // either way and the user gets a clearer error than a partial
        // failure halfway through.
        debug!(if_index, "rtnetlink: set link down before mode change");
        if let Err(err) = self.link.set_link_down(interface).await {
            warn!(error=%err, "rtnetlink set_link_down failed; attempting mode change anyway");
        }

        debug!(if_index, ?if_type, "nl80211: set_interface");
        let result = self.socket.set_interface(if_index, if_type).await;

        // Always try to bring the link back up so we don't leave the
        // operator with a dead interface, even if the mode change failed.
        debug!(if_index, "rtnetlink: set link back up");
        if let Err(err) = self.link.set_link_up(interface).await {
            warn!(error=%err, "rtnetlink set_link_up failed after mode change; interface may be left DOWN");
        }

        match result {
            Ok(()) => Ok(()),
            Err(err) => {
                warn!(error=?err, "set_interface failed");
                Err(translate_err(err))
            }
        }
    }

    async fn set_channel(
        &self,
        interface: &str,
        frequency_mhz: u32,
        width: ChannelWidth,
    ) -> HwResult<()> {
        let if_index = self.find_if_index(interface).await?;
        let cfg = ChannelConfig::new(if_index, frequency_mhz, width_to_nl(width));
        debug!(if_index, frequency_mhz, ?width, "nl80211: set_channel");
        self.socket.set_channel(cfg).await.map_err(translate_err)
    }

    async fn start_capture(
        &self,
        interface: &str,
        bpf_filter: Option<&str>,
    ) -> HwResult<CaptureSession> {
        // Open the pcap handle before the worker thread so any error
        // (interface missing, EPERM, etc.) surfaces synchronously to the
        // caller instead of being lost inside the spawn.
        let mut cap = Capture::from_device(interface)
            .map_err(|e| pcap_to_hw(interface, e))?
            .promisc(true)
            .immediate_mode(true)
            .snaplen(65535)
            .timeout(200)
            .open()
            .map_err(|e| pcap_to_hw(interface, e))?;

        if let Some(filter) = bpf_filter {
            cap.filter(filter, true)
                .map_err(|e| HwError::Other(format!("bad bpf filter `{filter}`: {e}")))?;
        }

        let datalink = cap.get_datalink();
        let link_type = link_type_from_pcap(datalink);

        let (frames_tx, frames_rx) = mpsc::channel::<CapturedFrame>(1024);
        let (stop_tx, stop_rx) = oneshot::channel();
        let if_name = interface.to_string();

        // pcap is blocking; isolate it on the blocking pool. Stop signal
        // is checked between packets via a try_recv since pcap_next_ex
        // can't be cancelled mid-call (we use a 200 ms read timeout to
        // bound the worst-case shutdown latency).
        std::thread::Builder::new()
            .name(format!("wifie-pcap-{if_name}"))
            .spawn(move || {
                let mut stop_rx = stop_rx;
                loop {
                    match stop_rx.try_recv() {
                        Ok(()) | Err(oneshot::error::TryRecvError::Closed) => break,
                        Err(oneshot::error::TryRecvError::Empty) => {}
                    }
                    match cap.next_packet() {
                        Ok(packet) => {
                            let frame = CapturedFrame {
                                timestamp_ms: Utc::now().timestamp_millis(),
                                link_type,
                                raw: packet.data.to_vec(),
                            };
                            // blocking_send so we exert backpressure on
                            // the kernel rather than silently dropping
                            // frames when consumers fall behind.
                            if frames_tx.blocking_send(frame).is_err() {
                                break;
                            }
                        }
                        Err(pcap::Error::TimeoutExpired) => continue,
                        Err(pcap::Error::NoMorePackets) => break,
                        Err(e) => {
                            error!(?e, "pcap read error; stopping capture");
                            break;
                        }
                    }
                }
                debug!(if_name = %if_name, "pcap worker exiting");
            })
            .map_err(|e| HwError::Other(format!("failed to spawn pcap thread: {e}")))?;

        Ok(CaptureSession {
            interface: interface.to_string(),
            link_type,
            frames: frames_rx,
            stop: stop_tx,
        })
    }
}

fn link_type_from_pcap(dlt: Linktype) -> LinkType {
    // Linktype is a thin newtype around c_int; match on the well-known
    // numeric constants from <pcap/dlt.h>.
    match dlt.0 {
        1 => LinkType::Ethernet,
        105 => LinkType::Ieee80211,
        127 => LinkType::IeeeWithRadiotap,
        _ => LinkType::Other,
    }
}

fn pcap_to_hw(interface: &str, err: pcap::Error) -> HwError {
    let msg = err.to_string();
    let lower = msg.to_ascii_lowercase();
    if lower.contains("permission") || lower.contains("operation not permitted") {
        HwError::PermissionDenied
    } else if lower.contains("no such device") || lower.contains("not found") {
        HwError::InterfaceNotFound(interface.to_string())
    } else {
        HwError::Other(msg)
    }
}

fn translate_err(err: NlError) -> HwError {
    let msg = err.msg;
    let lower = msg.to_ascii_lowercase();
    if lower.contains("eperm") || lower.contains("eacces") || lower.contains("permission") {
        HwError::PermissionDenied
    } else if lower.contains("enodev") || lower.contains("no such") {
        HwError::InterfaceNotFound(msg)
    } else {
        HwError::Other(msg)
    }
}

fn translate_interface(raw: NlWirelessInterface) -> WirelessInterface {
    let mode = raw
        .interface_type
        .map(nl_to_mode)
        .unwrap_or(InterfaceMode::Unspecified);
    let bands = raw
        .frequency
        .map(|f| vec![band_for_freq(f)])
        .unwrap_or_default();
    WirelessInterface {
        name: raw.name,
        phy: format!("phy{}", raw.wiphy_index),
        if_index: raw.interface_index,
        mac: raw.mac.to_string(),
        mode,
        supported_bands_ghz: bands,
        frequency_mhz: raw.frequency,
        channel_width: nl_width_to_width(raw.channel_width),
        ssid: raw.ssid,
    }
}

fn band_for_freq(freq_mhz: u32) -> f32 {
    match freq_mhz {
        f if f >= 5955 => 6.0,
        f if f >= 5170 => 5.0,
        _ => 2.4,
    }
}

fn mode_to_nl(mode: InterfaceMode) -> NlInterfaceType {
    match mode {
        InterfaceMode::Monitor => NlInterfaceType::Monitor,
        InterfaceMode::Managed => NlInterfaceType::Station,
        InterfaceMode::Unspecified => NlInterfaceType::Unspecified,
    }
}

fn nl_to_mode(t: NlInterfaceType) -> InterfaceMode {
    match t {
        NlInterfaceType::Monitor => InterfaceMode::Monitor,
        NlInterfaceType::Station => InterfaceMode::Managed,
        _ => InterfaceMode::Unspecified,
    }
}

fn width_to_nl(width: ChannelWidth) -> NlChannelWidth {
    match width {
        ChannelWidth::Width20 => NlChannelWidth::Width20,
        ChannelWidth::Width40 => NlChannelWidth::Width40,
        ChannelWidth::Width80 => NlChannelWidth::Width80,
        ChannelWidth::Width160 => NlChannelWidth::Width160,
        ChannelWidth::Width320 => NlChannelWidth::Width320,
    }
}

fn nl_width_to_width(width: NlChannelWidth) -> Option<ChannelWidth> {
    match width {
        NlChannelWidth::Width20 | NlChannelWidth::Width20NoHT => Some(ChannelWidth::Width20),
        NlChannelWidth::Width40 => Some(ChannelWidth::Width40),
        NlChannelWidth::Width80 | NlChannelWidth::Width80P80 => Some(ChannelWidth::Width80),
        NlChannelWidth::Width160 => Some(ChannelWidth::Width160),
        NlChannelWidth::Width320 => Some(ChannelWidth::Width320),
        _ => None,
    }
}
