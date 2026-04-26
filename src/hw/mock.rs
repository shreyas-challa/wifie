//! In-memory backend for demos, CI, and developing without a radio attached.

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::{mpsc, oneshot, RwLock};

use super::{
    capture::{CaptureSession, CapturedFrame, LinkType},
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

    async fn start_capture(
        &self,
        interface: &str,
        _bpf_filter: Option<&str>,
    ) -> HwResult<CaptureSession> {
        // Confirm the interface exists; otherwise the consumer would just
        // see an idle channel and wonder.
        if !self
            .state
            .read()
            .await
            .iter()
            .any(|i| i.name == interface)
        {
            return Err(HwError::InterfaceNotFound(interface.to_string()));
        }

        let (frames_tx, frames_rx) = mpsc::channel(256);
        let (stop_tx, mut stop_rx) = oneshot::channel();

        // Synthetic stream: ~120 pps with 5 ms jitter so the dashboard's
        // canvas plot still moves in demo mode without touching a radio.
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(8));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    _ = tick.tick() => {
                        let frame = CapturedFrame {
                            timestamp_ms: Utc::now().timestamp_millis(),
                            link_type: LinkType::IeeeWithRadiotap,
                            // 64-byte synthetic frame; consumers only count
                            // it for the rate plot.
                            raw: vec![0u8; 64],
                        };
                        if frames_tx.send(frame).await.is_err() { break; }
                    }
                }
            }
        });

        Ok(CaptureSession {
            interface: interface.to_string(),
            link_type: LinkType::IeeeWithRadiotap,
            frames: frames_rx,
            stop: stop_tx,
        })
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
