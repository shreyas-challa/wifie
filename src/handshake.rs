//! Handshake capture pipeline.
//!
//! Subscribes to a `WirelessBackend::start_capture` session with an
//! EAPOL filter, dumps each captured frame to a `.pcap` artifact under
//! `~/.local/share/wifie/captures/<task-id>.pcap`, and updates a shared
//! task record so the dashboard can poll progress.
//!
//! Lifecycle is bounded: we stop after `MAX_EAPOL_FRAMES` (full 4-way
//! handshake plus a few extras) or after `CAPTURE_TIMEOUT_SECS` of
//! quiet, whichever comes first.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use pcap::{Capture, Linktype, PacketHeader};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::hw::{capture::LinkType, SharedBackend};

/// EAPOL = ether type 0x888e. Works on radiotap-tagged 802.11 captures
/// because libpcap synthesises the ether type from the LLC/SNAP header.
const EAPOL_FILTER: &str = "ether proto 0x888e";
const MAX_EAPOL_FRAMES: u32 = 8;
const CAPTURE_TIMEOUT_SECS: u64 = 60;
const QUIET_TIMEOUT_SECS: u64 = 25;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureType {
    Wpa2Handshake,
    Wpa3Pmkid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureTask {
    pub id: Uuid,
    pub interface: String,
    pub target_bssid: String,
    pub capture_type: CaptureType,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub eapol_frames_seen: u32,
    pub artifact_path: Option<String>,
    pub error: Option<String>,
}

pub type CaptureRegistry = Arc<RwLock<HashMap<Uuid, CaptureTask>>>;

pub fn captures_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from).or_else(|| {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local/share"))
    });
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("wifie/captures")
}

/// Spawn a capture task. Returns the initial CaptureTask synchronously
/// (Pending status); the worker updates the registry as it makes
/// progress.
pub async fn spawn(
    backend: SharedBackend,
    registry: CaptureRegistry,
    interface: String,
    target_bssid: String,
    capture_type: CaptureType,
) -> Result<CaptureTask, String> {
    let id = Uuid::new_v4();
    let dir = captures_dir();
    if let Err(err) = std::fs::create_dir_all(&dir) {
        return Err(format!("failed to create {}: {err}", dir.display()));
    }
    let artifact = dir.join(format!("{id}.pcap"));

    let now = Utc::now();
    let task = CaptureTask {
        id,
        interface: interface.clone(),
        target_bssid: target_bssid.clone(),
        capture_type: capture_type.clone(),
        status: TaskStatus::Pending,
        created_at: now,
        updated_at: now,
        eapol_frames_seen: 0,
        artifact_path: Some(artifact.to_string_lossy().into_owned()),
        error: None,
    };
    registry.write().await.insert(id, task.clone());

    let registry_for_task = registry.clone();
    tokio::spawn(async move {
        let session = match backend
            .start_capture(&interface, Some(EAPOL_FILTER))
            .await
        {
            Ok(s) => s,
            Err(err) => {
                fail_task(&registry_for_task, id, format!("start_capture: {err}")).await;
                return;
            }
        };

        // pcap savefile is sync; spawn it on a blocking thread and
        // shuttle frames in via blocking_recv so we don't hold a
        // !Send pointer across an await.
        update_task(&registry_for_task, id, |t| {
            t.status = TaskStatus::Running;
            t.updated_at = Utc::now();
        })
        .await;
        info!(task = %id, %interface, %target_bssid, "handshake capture: running");

        let mut frames = session.frames;
        let link_type = session.link_type;
        let stop = session.stop;

        let registry_for_writer = registry_for_task.clone();
        let writer_path = artifact.clone();
        let (frame_tx, frame_rx) = std::sync::mpsc::sync_channel::<(i64, Vec<u8>)>(64);

        let writer = std::thread::Builder::new()
            .name(format!("wifie-pcap-dump-{id}"))
            .spawn(move || {
                run_writer(id, &writer_path, link_type, frame_rx, registry_for_writer)
            });

        let writer_handle = match writer {
            Ok(h) => h,
            Err(err) => {
                fail_task(&registry_for_task, id, format!("spawn dump thread: {err}")).await;
                return;
            }
        };

        let deadline = tokio::time::Instant::now() + Duration::from_secs(CAPTURE_TIMEOUT_SECS);
        let mut last_frame_at = tokio::time::Instant::now();

        loop {
            let timeout = Duration::from_secs(QUIET_TIMEOUT_SECS);
            tokio::select! {
                frame = frames.recv() => {
                    match frame {
                        Some(f) => {
                            last_frame_at = tokio::time::Instant::now();
                            if frame_tx.send((f.timestamp_ms, f.raw)).is_err() {
                                break; // writer exited
                            }
                        }
                        None => break,
                    }
                }
                _ = tokio::time::sleep_until(deadline) => {
                    debug!(task = %id, "handshake capture: hit absolute timeout");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    if tokio::time::Instant::now().duration_since(last_frame_at) > timeout {
                        debug!(task = %id, "handshake capture: hit quiet timeout");
                        break;
                    }
                    let task_now = registry_for_task.read().await.get(&id).cloned();
                    if let Some(t) = task_now {
                        if t.eapol_frames_seen >= MAX_EAPOL_FRAMES {
                            debug!(task = %id, "handshake capture: enough EAPOL frames seen");
                            break;
                        }
                    }
                }
            }
        }

        // Drop the frame_tx so the writer sees a closed channel and
        // flushes the pcap; also drop the stop sender so the pcap thread
        // exits cleanly.
        drop(frame_tx);
        drop(stop);
        let _ = writer_handle.join();

        let frames_seen = registry_for_task
            .read()
            .await
            .get(&id)
            .map(|t| t.eapol_frames_seen)
            .unwrap_or(0);
        let final_status = if frames_seen > 0 {
            TaskStatus::Complete
        } else {
            TaskStatus::Failed
        };
        update_task(&registry_for_task, id, move |t| {
            t.status = final_status;
            if t.error.is_none() && t.eapol_frames_seen == 0 {
                t.error = Some(
                    "no EAPOL frames captured (try targeted deauth, or move closer to the AP)"
                        .into(),
                );
            }
            t.updated_at = Utc::now();
        })
        .await;
        info!(task = %id, "handshake capture: finished");
    });

    Ok(task)
}

fn run_writer(
    id: Uuid,
    path: &std::path::Path,
    link_type: LinkType,
    rx: std::sync::mpsc::Receiver<(i64, Vec<u8>)>,
    registry: CaptureRegistry,
) {
    let dlt = match link_type {
        LinkType::IeeeWithRadiotap => Linktype::IEEE802_11_RADIOTAP,
        LinkType::Ieee80211 => Linktype::IEEE802_11,
        LinkType::Ethernet => Linktype::ETHERNET,
        LinkType::Other => Linktype(147), // LINKTYPE_USER0
    };
    let dead = match Capture::dead(dlt) {
        Ok(d) => d,
        Err(err) => {
            error!(task = %id, ?err, "Capture::dead failed");
            block_on_set_error(&registry, id, err.to_string());
            return;
        }
    };
    let mut savefile = match dead.savefile(path) {
        Ok(s) => s,
        Err(err) => {
            error!(task = %id, ?err, "savefile open failed");
            block_on_set_error(&registry, id, err.to_string());
            return;
        }
    };

    while let Ok((ts_ms, raw)) = rx.recv() {
        let header = PacketHeader {
            ts: libc::timeval {
                tv_sec: (ts_ms / 1000) as libc::time_t,
                tv_usec: ((ts_ms % 1000) * 1000) as libc::suseconds_t,
            },
            caplen: raw.len() as u32,
            len: raw.len() as u32,
        };
        let pkt = pcap::Packet::new(&header, &raw);
        savefile.write(&pkt);
        block_on_increment(&registry, id);
    }
    let _ = savefile.flush();
}

fn block_on_increment(registry: &CaptureRegistry, id: Uuid) {
    let registry = registry.clone();
    // Cheap blocking write — registry lock is short-lived.
    futures::executor::block_on(async move {
        if let Some(t) = registry.write().await.get_mut(&id) {
            t.eapol_frames_seen += 1;
            t.updated_at = Utc::now();
        }
    });
}

fn block_on_set_error(registry: &CaptureRegistry, id: Uuid, err: String) {
    let registry = registry.clone();
    futures::executor::block_on(async move {
        if let Some(t) = registry.write().await.get_mut(&id) {
            t.error = Some(err);
            t.status = TaskStatus::Failed;
            t.updated_at = Utc::now();
        }
    });
}

async fn fail_task(registry: &CaptureRegistry, id: Uuid, err: String) {
    warn!(task = %id, "{err}");
    update_task(registry, id, |t| {
        t.status = TaskStatus::Failed;
        t.error = Some(err);
        t.updated_at = Utc::now();
    })
    .await;
}

async fn update_task<F: FnOnce(&mut CaptureTask)>(registry: &CaptureRegistry, id: Uuid, f: F) {
    let mut guard = registry.write().await;
    if let Some(t) = guard.get_mut(&id) {
        f(t);
    }
}

/// Send N deauthentication frames to a target client (or broadcast).
/// Uses `aireplay-ng -0 N -a <bssid> [-c <client>] <iface>`. We shell
/// out here because netlink_wi 0.8 doesn't expose NL80211_CMD_FRAME for
/// management-frame TX, and per the founding spec subprocess wrappers
/// are explicitly allowed for active offensive tools.
pub async fn deauth(
    interface: &str,
    bssid: &str,
    client: Option<&str>,
    count: u8,
) -> Result<String, String> {
    let mut args: Vec<String> = vec![
        "-0".into(),
        count.to_string(),
        "-a".into(),
        bssid.to_string(),
    ];
    if let Some(c) = client {
        args.push("-c".into());
        args.push(c.to_string());
    }
    args.push(interface.to_string());

    let out = tokio::process::Command::new("aireplay-ng")
        .args(&args)
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "aireplay-ng not installed (sudo dnf install aircrack-ng)".to_string()
            } else {
                format!("failed to spawn aireplay-ng: {e}")
            }
        })?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "aireplay-ng exited with {}: {}",
            out.status,
            stderr.trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
