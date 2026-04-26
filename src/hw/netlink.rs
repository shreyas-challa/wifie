//! Real nl80211 backend, powered by `netlink_wi::AsyncNlSocket`.
//!
//! Translates the crate's types into the neutral `WirelessInterface` /
//! `InterfaceMode` / `ChannelWidth` shapes so HTTP routes never see vendor
//! types.

use async_trait::async_trait;
use netlink_wi::{
    interface::{
        ChannelWidth as NlChannelWidth, InterfaceType as NlInterfaceType,
        WirelessInterface as NlWirelessInterface,
    },
    AsyncNlSocket, ChannelConfig, NlError,
};
use tracing::{debug, warn};

use super::{
    ChannelWidth, HwError, HwResult, InterfaceMode, WirelessBackend, WirelessInterface,
};

pub struct NetlinkBackend {
    socket: AsyncNlSocket,
}

impl NetlinkBackend {
    pub async fn connect() -> HwResult<Self> {
        let socket = AsyncNlSocket::connect().await.map_err(translate_err)?;
        Ok(Self { socket })
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
        debug!(if_index, ?if_type, "nl80211: set_interface");
        if let Err(err) = self.socket.set_interface(if_index, if_type).await {
            // Many drivers refuse mode changes while the netdev is UP.
            // Surface a hint instead of a raw errno.
            warn!(error=?err, "set_interface failed; the link may need to be brought down");
            return Err(translate_err(err));
        }
        Ok(())
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
