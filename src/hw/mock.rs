//! In-memory backend for demos, CI, and developing without a radio attached.

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{
    ChannelWidth, HwError, HwResult, InterfaceMode, WirelessBackend, WirelessInterface,
};

pub struct MockBackend {
    state: RwLock<Vec<WirelessInterface>>,
}

impl MockBackend {
    pub fn seeded() -> Self {
        Self {
            state: RwLock::new(seed_fixtures()),
        }
    }
}

#[async_trait]
impl WirelessBackend for MockBackend {
    fn name(&self) -> &'static str {
        "mock"
    }

    async fn list_interfaces(&self) -> HwResult<Vec<WirelessInterface>> {
        Ok(self.state.read().await.clone())
    }

    async fn set_mode(&self, interface: &str, mode: InterfaceMode) -> HwResult<()> {
        let mut guard = self.state.write().await;
        let iface = guard
            .iter_mut()
            .find(|i| i.name == interface)
            .ok_or_else(|| HwError::InterfaceNotFound(interface.to_string()))?;
        iface.mode = mode;
        Ok(())
    }

    async fn set_channel(
        &self,
        interface: &str,
        frequency_mhz: u32,
        width: ChannelWidth,
    ) -> HwResult<()> {
        let mut guard = self.state.write().await;
        let iface = guard
            .iter_mut()
            .find(|i| i.name == interface)
            .ok_or_else(|| HwError::InterfaceNotFound(interface.to_string()))?;
        iface.frequency_mhz = Some(frequency_mhz);
        iface.channel_width = Some(width);
        Ok(())
    }
}

fn seed_fixtures() -> Vec<WirelessInterface> {
    vec![
        WirelessInterface {
            name: "wlan0".into(),
            phy: "phy0".into(),
            if_index: 3,
            mac: "02:11:22:33:44:55".into(),
            mode: InterfaceMode::Managed,
            supported_bands_ghz: vec![2.4, 5.0],
            frequency_mhz: Some(2412),
            channel_width: Some(ChannelWidth::Width20),
            ssid: Some("lab-mgmt".into()),
        },
        WirelessInterface {
            name: "wlan1".into(),
            phy: "phy1".into(),
            if_index: 5,
            mac: "02:AA:BB:CC:DD:EE".into(),
            mode: InterfaceMode::Monitor,
            supported_bands_ghz: vec![2.4, 5.0, 6.0],
            frequency_mhz: Some(5955),
            channel_width: Some(ChannelWidth::Width160),
            ssid: None,
        },
    ]
}
