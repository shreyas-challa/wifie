#!/usr/bin/env bash
# wifie — single-command bring-up.
#
# Boots the Rust backend and the React dev server side by side and tears
# them both down on Ctrl+C. Designed so a first-time contributor can clone
# the repo and run `./run.sh` without reading the README.

set -Eeuo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

# ---- defaults ----------------------------------------------------------------
MODE="${WIFIE_HW:-}"           # mock | real ; empty = auto (real on Linux)
BUILD_ONLY=false
RUN_BACKEND=true
RUN_FRONTEND=true
RELEASE=false

# ---- ui helpers --------------------------------------------------------------
c_dim()  { printf '\033[2m%s\033[0m\n' "$*"; }
c_ok()   { printf '\033[32m%s\033[0m\n' "$*"; }
c_warn() { printf '\033[33m%s\033[0m\n' "$*"; }
c_err()  { printf '\033[31m%s\033[0m\n' "$*" >&2; }

usage() {
  cat <<USAGE
Usage: ./run.sh [options]

Boots the Rust backend (port 3000) and the React dev server (port 5173)
in one shell. Open http://localhost:5173 once both are ready.

Options
  --mock              Use the in-memory mock backend (no caps required)
  --real              Force the real nl80211 backend (default on Linux)
  --release           Build the backend in release mode
  --build-only        Build both halves and exit
  --backend-only      Only run the Rust backend
  --frontend-only     Only run the frontend dev server
  -h, --help          Show this help

Environment
  WIFIE_HW=mock|real         Same as --mock / --real
  WIFIE_BACKEND_URL=URL      Override backend URL the frontend proxies to
  RUST_LOG=info              Standard tracing filter (passed through)
USAGE
}

# ---- arg parse ---------------------------------------------------------------
while [[ $# -gt 0 ]]; do
  case "$1" in
    --mock)            MODE="mock" ;;
    --real)            MODE="real" ;;
    --release)         RELEASE=true ;;
    --build-only)      BUILD_ONLY=true ;;
    --backend-only)    RUN_FRONTEND=false ;;
    --frontend-only)   RUN_BACKEND=false ;;
    -h|--help)         usage; exit 0 ;;
    *)                 c_err "Unknown option: $1"; usage; exit 1 ;;
  esac
  shift
done

# ---- toolchain detection -----------------------------------------------------
if command -v cargo >/dev/null 2>&1; then
  CARGO="$(command -v cargo)"
elif [[ -x "$HOME/.cargo/bin/cargo" ]]; then
  CARGO="$HOME/.cargo/bin/cargo"
elif $RUN_BACKEND; then
  c_err "cargo not found. Install rustup: https://rustup.rs"
  c_err "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

if $RUN_FRONTEND && ! command -v npm >/dev/null 2>&1; then
  c_err "npm not found. Install Node.js >= 20 (https://nodejs.org)."
  exit 1
fi

# ---- libpcap detection (advisory) -------------------------------------------
if $RUN_BACKEND && ! ldconfig -p 2>/dev/null | grep -q 'libpcap\.so'; then
  c_warn "libpcap not detected. On Fedora: sudo dnf install libpcap-devel"
  c_warn "(Build will fail at the pcap crate link step otherwise.)"
fi

# ---- caps advisory -----------------------------------------------------------
warn_caps_if_needed() {
  local effective="$MODE"
  [[ -z "$effective" ]] && effective="real"
  [[ "$effective" != "real" ]] && return 0
  [[ $EUID -eq 0 ]] && return 0

  local profile=debug
  $RELEASE && profile=release
  local bin="$ROOT/target/$profile/wifie-server"
  [[ -f "$bin" ]] || return 0

  if ! getcap "$bin" 2>/dev/null | grep -q cap_net_admin; then
    cat <<EOF

$(c_warn "  Note: real backend will start but Monitor Mode and channel changes")
$(c_warn "  need CAP_NET_ADMIN. Grant once (no sudo on every run):")

      sudo setcap cap_net_admin,cap_net_raw=eip $bin

  …or run with sudo, or pass --mock to skip nl80211 entirely.

EOF
  fi
}

# ---- shutdown handling -------------------------------------------------------
PIDS=()
shutdown() {
  trap - INT TERM EXIT
  echo
  c_dim "shutting down…"
  for pid in "${PIDS[@]:-}"; do
    if [[ -n "${pid:-}" ]] && kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
    fi
  done
  wait 2>/dev/null || true
}
trap shutdown INT TERM EXIT

# ---- build -------------------------------------------------------------------
if $RUN_FRONTEND && [[ ! -d frontend/node_modules ]]; then
  c_dim "==> installing frontend deps (first run only)"
  (cd frontend && npm install --silent)
fi

if $RUN_BACKEND; then
  c_dim "==> building backend (first run takes a couple of minutes)"
  if $RELEASE; then
    "$CARGO" build --release
  else
    "$CARGO" build
  fi
fi

if $BUILD_ONLY; then
  if $RUN_FRONTEND; then
    c_dim "==> building frontend bundle"
    (cd frontend && npm run build)
  fi
  c_ok "build complete."
  exit 0
fi

warn_caps_if_needed

# ---- run ---------------------------------------------------------------------
if $RUN_BACKEND; then
  : "${RUST_LOG:=info,wifie_server=debug}"
  export RUST_LOG
  if [[ -n "$MODE" ]]; then
    export WIFIE_HW="$MODE"
  fi
  c_ok   "==> backend starting on http://localhost:3000${MODE:+  (WIFIE_HW=$MODE)}"
  if $RELEASE; then
    "$CARGO" run --quiet --release &
  else
    "$CARGO" run --quiet &
  fi
  PIDS+=("$!")
fi

if $RUN_FRONTEND; then
  c_ok "==> frontend starting on http://localhost:5173"
  ( cd frontend && npm run dev --silent ) &
  PIDS+=("$!")
fi

cat <<EOF

$(c_ok "ready.")
  $(c_dim "frontend  →  http://localhost:5173")
  $(c_dim "backend   →  http://localhost:3000  (health: /health)")
  $(c_dim "Ctrl+C    →  stop both")

EOF

# Exit as soon as either child dies, so the user sees crashes immediately
# and the trap cleans up the survivor.
wait -n
status=$?
c_warn "a subprocess exited (status $status); shutting the rest down."
exit "$status"
