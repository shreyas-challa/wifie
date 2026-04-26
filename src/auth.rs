//! Lab authorization gate.
//!
//! Anything that can attack a real network — deauth injection, EAPOL
//! capture targeted at a specific BSSID, the vuln-test runners — must
//! check `is_authorized()` before doing the thing. The allowlist comes
//! from `WIFIE_LAB_AUTHORIZED_BSSIDS`, a comma- or whitespace-separated
//! list of MACs in any common format. An unset env var means **deny
//! everything**: the project's safety contract is opt-in, not opt-out.

use std::collections::HashSet;
use std::sync::OnceLock;

const ENV_VAR: &str = "WIFIE_LAB_AUTHORIZED_BSSIDS";

static ALLOWLIST: OnceLock<HashSet<String>> = OnceLock::new();

fn allowlist() -> &'static HashSet<String> {
    ALLOWLIST.get_or_init(|| match std::env::var(ENV_VAR) {
        Ok(raw) => raw
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .map(normalize)
            .collect(),
        Err(_) => HashSet::new(),
    })
}

/// Returns the canonical (uppercase, colon-delimited) form of a MAC.
/// Strips spaces and accepts dash- or colon-delimited inputs. Inputs
/// that don't look MAC-shaped are returned uppercased so the matcher
/// still produces a consistent miss.
pub fn normalize(input: &str) -> String {
    input
        .trim()
        .replace('-', ":")
        .to_ascii_uppercase()
}

pub fn is_authorized(bssid: &str) -> bool {
    let list = allowlist();
    if list.is_empty() {
        return false;
    }
    list.contains(&normalize(bssid))
}

/// What the dashboard / docs surface to operators so they can see what
/// they're allowed to touch without grepping their environment.
pub fn authorized_list() -> Vec<String> {
    let mut v: Vec<String> = allowlist().iter().cloned().collect();
    v.sort();
    v
}

pub fn env_var_name() -> &'static str {
    ENV_VAR
}
