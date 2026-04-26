//! Vulnerability test runners (Module 3).
//!
//! Wraps published research tooling rather than reimplementing it. The
//! original Dragonblood and KRACK papers each shipped reference exploit
//! scripts (Vanhoef et al.); rebuilding those from scratch in Rust is a
//! research project, not a sprint. So we run the published tools as
//! subprocesses, behind the same `WIFIE_LAB_AUTHORIZED_BSSIDS` gate
//! that guards the handshake pipeline.
//!
//! If a tool isn't on PATH, the runner returns a `tool_missing` error
//! that names the tool and how to install it — never a silent skip.
//!
//! NOTE for this repo's hardware: the user's Alfa adapter has been
//! validated for WPA2 capture/crack only. KRACK and SAE-downgrade flows
//! both depend on injection capabilities and AP-mode that haven't been
//! exercised yet. Treat any test that runs successfully here as a
//! starting point, not a green check — verify the result against the
//! attack's expected ground truth before believing it.

use std::ffi::OsStr;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestCase {
    /// SAE timing / downgrade probe. Wraps `dragondrain-ng` (or
    /// `dragondrain` if the newer fork isn't installed).
    #[serde(rename = "dragonblood_sae_timing")]
    DragonbloodSaeTiming,
    /// KRACK 4-way replay against an evil-twin AP. Wraps the
    /// `krack-test-client.py` runner from Vanhoef's krackattacks-scripts
    /// repo — operator must clone it and put `krack-ft-test.py` on PATH.
    #[serde(rename = "krack_4way_replay")]
    Krack4WayReplay,
}

#[derive(Debug, Serialize)]
pub struct TestResult {
    pub tool: String,
    pub args: Vec<String>,
    pub status: String,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
}

pub async fn run(case: TestCase, interface: &str, target_bssid: &str) -> Result<TestResult, String> {
    let (tool, args, install_hint) = build_invocation(case, interface, target_bssid);

    info!(?case, %target_bssid, %interface, tool, "vuln-test: invoking");
    let out = Command::new(&tool).args(&args).output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("tool_missing: {tool} not on PATH. {install_hint}")
        } else {
            format!("failed to spawn {tool}: {e}")
        }
    })?;

    let result = TestResult {
        tool: tool.clone(),
        args: args.iter().map(|a| a.to_string_lossy().into_owned()).collect(),
        status: format!("{}", out.status),
        stdout_excerpt: head(&String::from_utf8_lossy(&out.stdout), 4000),
        stderr_excerpt: head(&String::from_utf8_lossy(&out.stderr), 4000),
    };

    if !out.status.success() {
        warn!(?case, status = ?out.status, "vuln-test: non-zero exit");
    }
    Ok(result)
}

fn build_invocation(case: TestCase, interface: &str, target_bssid: &str) -> (String, Vec<std::ffi::OsString>, String) {
    match case {
        TestCase::DragonbloodSaeTiming => (
            // Prefer the maintained fork; users with the original
            // `dragondrain` can symlink it to `dragondrain-ng`.
            "dragondrain-ng".to_string(),
            vec![
                OsStr::new("--interface").into(),
                OsStr::new(interface).into(),
                OsStr::new("--target").into(),
                OsStr::new(target_bssid).into(),
            ],
            "Install from https://github.com/vanhoefm/dragondrain-and-time".to_string(),
        ),
        TestCase::Krack4WayReplay => (
            "krack-ft-test.py".to_string(),
            vec![OsStr::new(interface).into()],
            "Clone https://github.com/vanhoefm/krackattacks-scripts and put krack-ft-test.py on PATH"
                .to_string(),
        ),
    }
}

fn head(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…[truncated, {} bytes total]", &s[..max], s.len())
    }
}
