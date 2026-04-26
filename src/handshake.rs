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
    path::{Path, PathBuf},
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
    /// Hashcat 22000-format hashline file produced by `hcxpcapngtool`
    /// from the captured pcap. Set only after a successful conversion.
    #[serde(default)]
    pub hashcat_22000_path: Option<String>,
    /// Why the 22000 conversion didn't run / didn't produce output:
    /// `tool_missing`, an exit-code message, or "no hashes extracted".
    #[serde(default)]
    pub conversion_error: Option<String>,
}

pub type CaptureRegistry = Arc<RwLock<HashMap<Uuid, CaptureTask>>>;

pub fn captures_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from).or_else(|| {
        std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local/share"))
    });
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("wifie/captures")
}

fn task_json_path(dir: &Path, id: Uuid) -> PathBuf {
    dir.join(format!("{id}.json"))
}

/// Write the task as a JSON sidecar next to its pcap. Atomic-ish:
/// write to `<id>.json.tmp`, then rename. Logs and swallows errors —
/// persistence failure must never crash the capture worker.
fn persist_task(task: &CaptureTask) {
    let dir = captures_dir();
    if let Err(err) = std::fs::create_dir_all(&dir) {
        warn!(task = %task.id, ?err, "persist: mkdir failed");
        return;
    }
    let final_path = task_json_path(&dir, task.id);
    let tmp_path = dir.join(format!("{}.json.tmp", task.id));
    let bytes = match serde_json::to_vec_pretty(task) {
        Ok(b) => b,
        Err(err) => {
            warn!(task = %task.id, ?err, "persist: serialize failed");
            return;
        }
    };
    if let Err(err) = std::fs::write(&tmp_path, &bytes) {
        warn!(task = %task.id, ?err, "persist: tmp write failed");
        return;
    }
    if let Err(err) = std::fs::rename(&tmp_path, &final_path) {
        warn!(task = %task.id, ?err, "persist: rename failed");
    }
}

/// Reload all `<id>.json` sidecars from `captures_dir()` into a fresh
/// registry. Tasks left in `Pending` or `Running` from a prior process
/// are forced to `Failed` with a reason — they cannot resume across a
/// restart since their pcap session is gone.
pub fn load_persisted() -> CaptureRegistry {
    let mut map: HashMap<Uuid, CaptureTask> = HashMap::new();
    let dir = captures_dir();
    let entries = match std::fs::read_dir(&dir) {
        Ok(it) => it,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                warn!(?err, dir = %dir.display(), "load_persisted: read_dir failed");
            }
            return Arc::new(RwLock::new(map));
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(err) => {
                warn!(?err, path = %path.display(), "load_persisted: read failed");
                continue;
            }
        };
        let mut task: CaptureTask = match serde_json::from_slice(&bytes) {
            Ok(t) => t,
            Err(err) => {
                warn!(?err, path = %path.display(), "load_persisted: parse failed");
                continue;
            }
        };
        if matches!(task.status, TaskStatus::Pending | TaskStatus::Running) {
            task.status = TaskStatus::Failed;
            task.error = Some("interrupted: server restarted before capture finished".into());
            task.updated_at = Utc::now();
            persist_task(&task);
        }
        map.insert(task.id, task);
    }
    info!(count = map.len(), "load_persisted: hydrated capture tasks");
    Arc::new(RwLock::new(map))
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
        hashcat_22000_path: None,
        conversion_error: None,
    };
    registry.write().await.insert(id, task.clone());
    persist_task(&task);

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

        if frames_seen > 0 {
            convert_to_22000(&registry_for_task, id, &artifact).await;
        }
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
    let snapshot = futures::executor::block_on(async move {
        let mut guard = registry.write().await;
        guard.get_mut(&id).map(|t| {
            t.eapol_frames_seen += 1;
            t.updated_at = Utc::now();
            t.clone()
        })
    });
    if let Some(t) = snapshot {
        persist_task(&t);
    }
}

fn block_on_set_error(registry: &CaptureRegistry, id: Uuid, err: String) {
    let registry = registry.clone();
    let snapshot = futures::executor::block_on(async move {
        let mut guard = registry.write().await;
        guard.get_mut(&id).map(|t| {
            t.error = Some(err);
            t.status = TaskStatus::Failed;
            t.updated_at = Utc::now();
            t.clone()
        })
    });
    if let Some(t) = snapshot {
        persist_task(&t);
    }
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
    let snapshot = {
        let mut guard = registry.write().await;
        guard.get_mut(&id).map(|t| {
            f(t);
            t.clone()
        })
    };
    if let Some(t) = snapshot {
        persist_task(&t);
    }
}

/// Run `hcxpcapngtool` on the captured pcap to produce a hashcat
/// 22000-mode hashline file. Stored next to the pcap as
/// `<id>.22000`. Failure modes:
///   * tool not on PATH      → `conversion_error = "tool_missing: ..."`
///   * tool ran, no hashes   → `conversion_error = "no hashes extracted"`
///   * tool exited non-zero  → `conversion_error = "<exit + stderr>"`
async fn convert_to_22000(registry: &CaptureRegistry, id: Uuid, pcap: &Path) {
    let out_path = pcap.with_extension("22000");

    let result = tokio::process::Command::new("hcxpcapngtool")
        .arg("-o")
        .arg(&out_path)
        .arg(pcap)
        .output()
        .await;

    let outcome: Result<String, String> = match result {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err("tool_missing: hcxpcapngtool not on PATH (sudo dnf install hcxtools)".to_string())
        }
        Err(e) => Err(format!("failed to spawn hcxpcapngtool: {e}")),
        Ok(out) if !out.status.success() => Err(format!(
            "hcxpcapngtool exited with {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Ok(_) => match std::fs::metadata(&out_path) {
            Ok(m) if m.len() > 0 => Ok(out_path.to_string_lossy().into_owned()),
            _ => Err("no hashes extracted (capture may be incomplete)".to_string()),
        },
    };

    match outcome {
        Ok(path) => {
            info!(task = %id, %path, "22000 conversion: ok");
            update_task(registry, id, |t| {
                t.hashcat_22000_path = Some(path);
                t.conversion_error = None;
                t.updated_at = Utc::now();
            })
            .await;
        }
        Err(err) => {
            warn!(task = %id, "22000 conversion: {err}");
            update_task(registry, id, |t| {
                t.conversion_error = Some(err);
                t.updated_at = Utc::now();
            })
            .await;
        }
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
