# WiFie

Local-host wireless penetration-testing console for Wi-Fi 6 / 6E / 7 audit work
in a controlled academic lab. Hardware-agnostic, no proprietary appliance, no
airmon-ng wrappers — Rust drives `nl80211` and `pcap` directly, the SPA
frontend renders telemetry on `<canvas>` and lives entirely in the browser.

## Stack
- **Backend** — Rust + `axum` HTTP, `socketioxide` (Socket.IO over WebSocket),
  `netlink_wi` for nl80211, `pcap` for raw frames. Active offensive paths
  (deauth, SAE downgrade, KRACK replay) are intentionally stubbed in this
  scaffold.
- **Frontend** — React 18 + Vite + Tailwind v4 (no SSR), `motion` for the
  floating dock + reveal animations, Tabler/Lucide icons, Canvas (no SVG) for
  packet plots. Both light and dark mode ship together; toggle is animated via
  the View Transitions API. Visual language follows `DESIGN_SYSTEM.md`.

## Run

### Backend (Rust)

```bash
cargo run --release
```

The server listens on `0.0.0.0:3000`. Socket.IO is at `/events`; REST routes
live under `/api/*`.

### Frontend (React + Vite)

```bash
cd frontend
npm install
npm run dev   # http://localhost:5173
```

`npm run build` produces a static `dist/` you can serve from the Rust binary
once you wire `tower-http`'s `ServeDir`.

## Hardware notes (Fedora + Alfa adapter)

The dev setup is a Fedora workstation with an Alfa USB radio plugged in.

```bash
# Build deps for the pcap crate and netlink work
sudo dnf install -y libpcap-devel pkgconf-pkg-config

# Optional: tools the lab cracking pipeline shells out to
sudo dnf install -y aircrack-ng hashcat iw
```

Talking to `nl80211` and putting an interface into Monitor Mode requires
elevated capabilities. Two clean options:

```bash
# Option A — run the binary as root for the session
sudo target/release/wifie-server

# Option B — grant the binary the specific caps once, then run as your user
sudo setcap cap_net_admin,cap_net_raw=eip target/release/wifie-server
target/release/wifie-server
```

NetworkManager will fight you for the radio. Either:

```bash
nmcli dev set wlan1 managed no   # release one adapter (replace wlan1)
# or, fully:
sudo systemctl stop NetworkManager
```

Most Alfa adapters (`rtl88xxau`, `mt76x2u`, `ath9k_htc`) support monitor mode
and frame injection out of the box on a recent Fedora kernel — you should see
the radio in `iw dev` and be able to flip it to monitor with `iw dev <iface>
set type monitor`. The dashboard's "Enable Monitor" button performs the same
operation through `netlink_wi`.

## Layout

```
.
├─ Cargo.toml                 # Rust workspace
├─ src/main.rs                # axum + socketioxide bootstrap, REST + WS
├─ DESIGN_SYSTEM.md           # the visual contract — read before touching UI
└─ frontend/
   ├─ index.html
   ├─ vite.config.js          # @tailwindcss/vite + react
   └─ src/
      ├─ App.jsx              # page composition, socket + state
      ├─ main.jsx
      ├─ styles.css           # oklch zinc tokens (both modes), Tailwind v4
      ├─ lib/
      │  ├─ utils.js          # cn()
      │  └─ theme.jsx         # ThemeProvider + useTheme
      └─ components/
         ├─ MinimalCard.jsx
         ├─ RippleButton.jsx
         ├─ FloatingDock.jsx
         ├─ AnimatedThemeToggler.jsx
         ├─ EncryptedText.jsx
         ├─ RevealOnScroll.jsx
         ├─ PacketCanvas.jsx
         ├─ Field.jsx
         ├─ InterfacePanel.jsx
         ├─ HandshakePanel.jsx
         └─ VulnLabPanel.jsx
```

## Safety

This project exists for authorized lab testing on networks the operator owns
or has explicit permission to attack. The active offensive modules ship as
stubs — wire them only inside that authorization boundary.
