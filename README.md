# WiFie

Local-host wireless penetration-testing console for Wi-Fi 6 / 6E / 7 audit work
in a controlled academic lab. Hardware-agnostic, no proprietary appliance, no
airmon-ng wrappers — Rust drives `nl80211` and `pcap` directly, the SPA
frontend renders telemetry on `<canvas>` and lives entirely in the browser.

## Quick start

```bash
git clone https://github.com/shreyas-challa/wifie.git
cd wifie
./run.sh --mock        # demo mode, no caps, no real radio touched
```

Open <http://localhost:5173>. Both halves of the stack share that one port —
the React dev server proxies `/api`, `/health`, and `/socket.io` (WebSocket
upgrade included) to the Rust backend. So if you're SSH'd into another box,
forward only `5173`:

```bash
ssh -L 5173:localhost:5173 user@your-linux-host
```

When you're ready to touch real hardware:

```bash
./run.sh               # real nl80211 backend (default on Linux)
```

`./run.sh --help` for all flags (`--release`, `--build-only`,
`--backend-only`, `--frontend-only`).

## Stack

- **Backend** — Rust + `axum` HTTP, `socketioxide` (Socket.IO over WebSocket),
  `netlink_wi` for nl80211, `pcap` for raw frames. Active offensive paths
  (deauth, SAE downgrade, KRACK replay) are intentionally stubbed in this
  scaffold.
- **Frontend** — React 18 + Vite + Tailwind v4 (no SSR), `motion` for the
  floating dock + reveal animations, Tabler/Lucide icons, Canvas (no SVG) for
  packet plots. Both light and dark mode ship together; toggle is animated via
  the View Transitions API. Visual language follows `DESIGN_SYSTEM.md`.

## Hardware abstraction

All radio control flows through `WirelessBackend` (`src/hw/mod.rs`) — a small
async trait with three operations: `list_interfaces`, `set_mode`,
`set_channel`. Two impls ship today:

| Backend     | When                                               | Notes                                            |
|-------------|----------------------------------------------------|--------------------------------------------------|
| `mock`      | `--mock` / `WIFIE_HW=mock`, or any non-Linux host  | Seeds two adapters; writes persist in memory.    |
| `nl80211`   | Default on Linux                                   | Wraps `netlink_wi::AsyncNlSocket` against nl80211. |

If the real backend can't connect at startup (no nl80211, missing caps), the
server logs a warning and falls back to mock — the dashboard never crashes
and the operator can see why via `/health` + the warning in stderr.

Add a new backend by implementing the trait (see `src/hw/mock.rs` for the
shortest possible example) and threading it through `select_from_env`.

## Real hardware setup (Fedora + Alfa-style USB radios)

```bash
# Build deps
sudo dnf install -y libpcap-devel pkgconf-pkg-config

# Optional: tools the lab cracking pipeline shells out to
sudo dnf install -y aircrack-ng hashcat iw
```

Talking to `nl80211` and putting an interface into Monitor Mode requires
elevated capabilities. Two clean options:

```bash
# A — run the binary as root for the session
sudo ./run.sh

# B — grant the binary the specific caps once, then run as your user
./run.sh --build-only
sudo setcap cap_net_admin,cap_net_raw=eip target/debug/wifie-server
./run.sh
```

NetworkManager will fight you for the radio. Release one adapter:

```bash
nmcli dev set wlp0s20f0u1i3 managed no   # adjust to your USB Wi-Fi name
```

Most Alfa adapters (`rtl88xxau`, `mt76x2u`, `ath9k_htc`) support monitor mode
and frame injection out of the box on a recent Fedora kernel.

## Hacking on it

Want the two halves in separate terminals (e.g. for clearer log output)?

```bash
./run.sh --backend-only       # terminal 1
./run.sh --frontend-only      # terminal 2
```

Useful endpoints:

```bash
curl localhost:3000/health           # which backend is live?
curl localhost:3000/api/interfaces   # what radios are visible?
```

Override the backend URL the frontend proxies to (split-host setups,
container networking, etc.):

```bash
WIFIE_BACKEND_URL=http://10.0.0.5:3000 ./run.sh --frontend-only
```

## Layout

```
.
├─ run.sh                     # single-command bring-up
├─ Cargo.toml
├─ DESIGN_SYSTEM.md           # the visual contract — read before touching UI
├─ src/
│  ├─ main.rs                 # axum + socketioxide bootstrap, REST + WS
│  └─ hw/
│     ├─ mod.rs               # WirelessBackend trait, types, env selector
│     ├─ mock.rs              # in-memory backend
│     └─ netlink.rs           # real nl80211 backend (Linux only)
└─ frontend/
   ├─ index.html
   ├─ vite.config.js          # proxy: /api, /health, /socket.io → backend
   └─ src/
      ├─ App.jsx              # page composition, socket + state
      ├─ styles.css           # oklch zinc tokens (both modes), Tailwind v4
      ├─ lib/                 # cn(), ThemeProvider
      └─ components/          # MinimalCard, RippleButton, FloatingDock, …
```

## Safety

This project exists for authorized lab testing on networks the operator owns
or has explicit permission to attack. Every offensive code path is gated
behind the `WIFIE_LAB_AUTHORIZED_BSSIDS` allowlist (`src/auth.rs`); an unset
env var means deny-everything. The dashboard's Notes card surfaces the
current allowlist so operators see what they're authorized to touch without
grepping their environment.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the ground rules, dev loop, and
the current "good first issue" list. CI on every PR runs `cargo build`,
`cargo clippy -- -D warnings`, and the frontend `npm run build`.
