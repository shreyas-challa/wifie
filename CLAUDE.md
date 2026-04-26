# WiFie — Project Context for Claude

This file is the cold-start brief for any future Claude session in this repo.
Read this first; defer to it before re-deriving anything from scratch.

The companion auto-memory at
`/home/noigel/.claude/projects/-home-noigel-wifi-wifie/memory/` is also
loaded automatically — `user_environment.md` there has the user's hardware
context (Fedora + Alfa USB adapter, WPA2-only validation so far).

---

## What this project is

A local-host, hardware-agnostic, open-source **wireless penetration-testing
console** for Wi-Fi 6 / 6E / 7 audit work in a controlled academic lab. The
explicit goal is to **replace the WiFi Pineapple and unstable legacy
wrappers** like `airmon-ng` with a Rust-native control plane and a single
React SPA dashboard. Owned by `shreyas-challa`. Public repo:
<https://github.com/shreyas-challa/wifie>.

The user is a student/researcher building this as both a real lab tool and
an OSS contribution — community-friendly defaults matter (single-command
bring-up, no proprietary deps, clear safety boundaries). Treat all
"offensive" code paths as opt-in lab features that must never run against
networks the operator doesn't control.

## The "Golden Stack" (from the founding prompt — non-negotiable)

**Backend (Rust):**
- `axum` HTTP, `tokio` runtime, `tracing` for logs.
- `netlink_wi` (0.8, async feature) for nl80211 — interface enumeration,
  monitor mode, channel/freq selection. **No bash wrappers, no airmon-ng.**
- `pcap` (2.x) for raw radiotap-tagged frame capture from the kernel.
- `socketioxide` (0.15) for ultra-low-latency WebSocket telemetry.
- Hybrid abstraction: passive paths (sniff, scan, enumerate) are native
  Rust; active heavy tools (password cracking) wrap C-based binaries
  (`hashcat`, `aircrack-ng`) via `tokio::process` subprocesses.

**Frontend (React SPA):**
- Plain React 18 + Vite 5 — **no Next.js, no SSR**. SEO is irrelevant for
  a localhost tool.
- Tailwind v4 via `@tailwindcss/vite` (no PostCSS).
- HTML5 `<canvas>` for packet plotting — **never SVG**, the DOM overhead
  blows up at high frequency.
- Visual language is dictated by `DESIGN_SYSTEM.md` (monochrome zinc,
  layered shadows, FloatingDock nav, oklch tokens for both modes,
  Tabler/Lucide icons, no emoji, no purple gradients). Read that file
  before touching any UI.

## Required MVP modules (from the founding prompt)

1. **Advanced Interface Management** — detect adapters, toggle Monitor
   Mode via nl80211, select frequencies across 2.4 / 5 / 6 GHz.
2. **Automated Handshake Capture** — trigger targeted deauths, monitor
   reconnection, parse + save WPA2 4-way handshakes / WPA3 PMKID locally.
3. **Vulnerability Re-implementation** — controlled downgrade attacks
   (Dragonblood / SAE timing) and lab-only evil-twin 4-way handshake
   manipulation (KRACK) to test modern client resilience.

---

## Where the project stands today

| Module                              | Status        | Notes |
|-------------------------------------|---------------|-------|
| Backend HTTP + WS bootstrap         | done          | `src/main.rs` |
| `WirelessBackend` trait + selector  | done          | `src/hw/mod.rs`; `WIFIE_HW=mock\|real` |
| Mock backend                        | done          | `src/hw/mock.rs` |
| nl80211 backend (real adapters)     | done          | `src/hw/netlink.rs` — verified on user's Alfa |
| Link-down wrapper around `set_mode` | done          | `src/hw/link.rs`; rtnetlink, in-process so caps survive |
| Module 1: Interface Management UI   | done          | Real adapters, monitor toggle, channel set |
| Frontend redesign per DESIGN_SYSTEM | done          | All components in `frontend/src/components/` |
| Single-command `./run.sh`           | done          | EXIT-trap cleanup, `--mock` / `--release` flags |
| Same-origin Vite proxy              | done          | `/api`, `/health`, `/socket.io` (ws) → backend |
| Module 2: Handshake Capture         | done          | `src/handshake.rs` — EAPOL pcap + deauth via aireplay-ng + auth gate |
| Module 3: Vuln tests (Dragonblood/KRACK) | scaffolded | `src/vuln.rs` wraps dragondrain-ng / krack-ft-test.py; gated |
| Real packet telemetry               | done          | M1: pcap thread + radiotap parse → /events |
| Production binary (Rust serves SPA) | done          | M3: `WIFIE_SERVE_DIR` + `./run.sh --prod` |
| Wiphy-aware supported_bands_ghz     | done          | M4: NetlinkBackend caches list_physical_devices |
| Lab authorization gate              | done          | `WIFIE_LAB_AUTHORIZED_BSSIDS`, `src/auth.rs` |
| Capture task disk persistence       | done          | N1: JSON sidecar per task; reload on startup |

### Repo layout

```
.
├─ run.sh                     single-command bring-up
├─ Cargo.toml                 axum + socketioxide + netlink_wi (linux) + pcap
├─ DESIGN_SYSTEM.md           visual contract — read before touching UI
├─ CLAUDE.md                  this file
├─ src/
│  ├─ main.rs                 axum routes, socketioxide, telemetry spawner
│  └─ hw/
│     ├─ mod.rs               WirelessBackend trait + types + select_from_env
│     ├─ mock.rs              in-memory backend (demos, CI, no caps)
│     └─ netlink.rs           Linux: AsyncNlSocket adapter
└─ frontend/
   ├─ index.html              theme bootstrap script (no FOUC)
   ├─ vite.config.js          @tailwindcss/vite + proxy
   └─ src/
      ├─ App.jsx              composition + state + socket
      ├─ styles.css           oklch zinc tokens, both modes
      ├─ lib/                 cn(), ThemeProvider
      └─ components/          MinimalCard, RippleButton, FloatingDock,
                              AnimatedThemeToggler, EncryptedText,
                              RevealOnScroll, PacketCanvas, Field,
                              InterfacePanel, HandshakePanel, VulnLabPanel
```

---

## Local environment (the user's machine)

- Fedora Linux. Use `dnf`, not `apt`. Wi-Fi build deps:
  `libpcap-devel`, `pkgconf-pkg-config`. Optional cracking tooling:
  `aircrack-ng`, `hashcat`, `iw`.
- Alfa USB Wi-Fi adapter is plugged in, currently appears as
  `wlp0s20f0u1i3` on `phy4`. The laptop's internal card is `wlp1s0`.
- **The Alfa is validated for WPA2 capture/crack only.** MitM-style
  flows (KRACK, evil twin, SAE downgrade) have **not** been tested
  against this adapter. Treat injection/AP-mode capability as unproven
  for this specific hardware when implementing offensive modules — start
  small with a controlled lab AP.
- Rust toolchain lives at `~/.cargo/bin/cargo` (installed via rustup,
  not via dnf — system PATH does not include it). Use the absolute path
  if `cargo` isn't on PATH.
- `netlink_wi` 0.8 source is vendored in
  `~/.cargo/registry/src/index.crates.io-*/netlink_wi-0.8.0/src/` —
  read it directly if you need to confirm an API signature.
- `node` / `npm` available system-wide.
- The user develops over SSH from a Windows laptop. The Vite proxy means
  they only need to forward port `5173`.

---

## Conventions for this repo (don't drift)

1. **Commits.** Author and committer must be:
   `shreyas-challa <shreyas.challa3@gmail.com>`. **No Co-Authored-By
   line.** Use the inline override every time:
   ```
   git -c user.name="shreyas-challa" -c user.email="shreyas.challa3@gmail.com" commit -m ...
   ```
   The user's global git config is **not** to be modified.
2. **Cadence.** Commit at every meaningful milestone, not in one giant
   final dump. Each commit should compile cleanly. Subject prefixes in
   use: `feat(...)`, `fix(...)`, `refactor(...)`, `docs:`, `build(deps):`,
   `chore:`.
3. **Pushing to `main`** is allowed when the user explicitly authorizes
   it. Don't auto-push.
4. **UI work.** `DESIGN_SYSTEM.md` is law. Specifically: no purple/violet/
   pink gradients, no glowing borders, no emoji in any UI string, no
   sharp corners on interactive elements. Pull animated components from
   the listed registries before hand-rolling.
5. **Hardware control.** Goes through `WirelessBackend` (`src/hw/mod.rs`).
   Never reach for `airmon-ng` or shell-out wrappers for read paths or
   for mode/channel writes — those are explicitly disallowed by the
   founding spec. Subprocess wrappers are only acceptable for
   computationally heavy *offline* tools (hashcat, aircrack).
6. **Errors.** Translate vendor errors (e.g. `NlError`) into the neutral
   `HwError` enum in `src/hw/mod.rs`. The frontend never sees vendor
   types. EPERM/EACCES collapse to `HwError::PermissionDenied` with a
   setcap hint.
7. **Authorization gates.** Before any deauth / exploit / injection code
   is wired, an authorization check must guard it (lab-allowlisted
   BSSIDs at minimum). This is the project's safety contract.

---

## How to work on it (dev loop)

```bash
cd ~/wifi/wifie
./run.sh --mock         # demo path, no caps, no real radio touched
./run.sh                # real nl80211 backend (Linux default)
./run.sh --build-only   # compile both halves, exit
./run.sh --help         # all flags
```

The single-command path is the user-facing contract. Any future feature
should keep `./run.sh` working without flags.

For low-level control while iterating:

```bash
~/.cargo/bin/cargo build
~/.cargo/bin/cargo run                    # backend only
WIFIE_HW=mock ~/.cargo/bin/cargo run      # backend, mock mode
( cd frontend && npm run dev )            # frontend only
```

Health probe:

```bash
curl localhost:3000/health                # backend live? which backend?
curl localhost:3000/api/interfaces        # what radios are visible?
```

---

## Next milestones — work through these in order

The original M1–M5 from the founding plan are all landed. The next
batch is about polish, hardening, and bridging from "the runners
exist" to "the runners produce verified results against the user's
own lab AP."

### N1 — Disk persistence for capture tasks ✓ done
JSON sidecar per task lives next to its pcap under
`~/.local/share/wifie/captures/<id>.json`. Persisted on every status
change. `handshake::load_persisted()` rehydrates the registry on
startup and demotes any in-flight (Pending/Running) tasks to Failed.

### N2 — Convert captured handshakes to hashcat 22000 format
Add a small post-capture step that runs `hcxpcapngtool -o <id>.22000
<id>.pcap` (subprocess; ship a "Download .22000" link in the UI).
Tool path discovery + tool_missing error using the same pattern as
`src/vuln.rs`.

### N3 — UI: artifact downloads + lab BSSID display
HandshakePanel should surface the artifact path with a download link
(serve from a new `/api/captures/handshake/:id/artifact` route),
and the dashboard footer / Notes card should show
`/api/auth/lab-bssids` so operators see exactly what's authorized
without grepping their environment.

### N4 — Verified offensive run on the user's Alfa
The infrastructure is real but the operator-side verification isn't.
Walk through one full WPA2 handshake capture against a controlled lab
AP, confirm the resulting `.pcap` cracks under hashcat with a known
PSK. This is the milestone that turns "the pipeline runs" into "the
pipeline works." **Note**: the Alfa is validated for WPA2 cap/crack;
KRACK and SAE downgrade depend on injection capabilities that haven't
been exercised — do those second, with a small expected-failure budget.

### N5 — CI + contributor polish
- `rust-toolchain.toml` pinning stable.
- GitHub Actions: cargo build + clippy + npm run build on PRs.
- `CONTRIBUTING.md` with "good first issue" list pulled from this
  section.
- A small demo `.pcap` checked in for the mock backend to replay.

---

## Founding prompt (verbatim, for future-you)

> architect and write the foundational boilerplate for a custom,
> open-source Wireless Penetration Testing Web Interface. This is a
> local-host, hardware-agnostic dashboard designed specifically to
> audit Wi-Fi 6, 6E, and 7 networks in a controlled academic lab
> environment. It is designed to replace outdated, proprietary hardware
> like the WiFi Pineapple and unstable legacy wrappers.
>
> The architecture must strictly follow this "Golden Stack" specification:
>
> 1. Backend Architecture (Rust)
> The backend must be written entirely in Rust to ensure zero
> garbage-collection pauses, memory safety, and the extreme speed
> required to process Wi-Fi 7 (802.11be) multi-link operation (MLO)
> traffic and 320 MHz channels.
> * Hardware Control: Use the `netlink_wi` crate to interact natively
>   with the Linux `nl80211` interface. Do not use external bash
>   scripts like `airmon-ng`. The backend must natively list interfaces,
>   unbind them, switch them to Monitor Mode, and change channels.
> * Packet Sniffing: Use the `pcap` crate for raw radio frame capture
>   directly from the kernel.
> * Real-Time Streaming: Implement an ultra-low latency WebSocket
>   server using the `socketioxide` crate to stream live network events
>   to the frontend.
> * Abstraction Strategy (Hybrid): Passive tasks (sniffing, scanning)
>   must be native Rust. Active, computationally heavy offline tasks
>   (password cracking) must wrap established C-based tools (like
>   `hashcat` or `aircrack-ng`) via secure subprocess execution.
>
> 2. Frontend Architecture (React)
> The frontend must be a lightweight Single Page Application (SPA)
> served locally. Do not use Next.js or Server-Side Rendering (SSR),
> as SEO is irrelevant for a localhost tool and adds unnecessary
> WebSocket overhead.
> * Framework: Plain React built with Vite.
> * Styling: Tailwind CSS for a modern, responsive, terminal-free UI.
> * Visualization: You must use HTML5 Canvas for real-time packet
>   graphing. Do not use SVG, as the DOM overhead will crash the
>   browser when plotting thousands of high-frequency network packets.
>
> 3. Required MVP Modules
> * Advanced Interface Management: A UI component and Rust route to
>   detect adapters, toggle Monitor Mode via `nl80211`, and select
>   frequencies across 2.4, 5, and 6 GHz bands.
> * Automated Handshake Capture: Logic to trigger targeted
>   Deauthentication frames, monitor the reconnection, and parse/save
>   the resulting WPA2 4-way handshake or WPA3 PMKID locally.
> * Vulnerability Re-implementation (Lab Testing): Stubs and UI
>   triggers for controlled downgrade attacks (Dragonblood / SAE
>   timing attacks) and our lab's evil twin 4-way handshake
>   manipulation (KRACK) to test modern client resilience.

(Subsequent prompts in the founding session also asked for the
`Cargo.toml`, `package.json`, `main.rs`, and `App.jsx` to be the first
artifacts — those exist and have been iterated on since.)

---

## Cold-start checklist for a new session

1. Read this file. Read `DESIGN_SYSTEM.md` if any UI work is on the
   table.
2. `git log --oneline -10` to see what landed since.
3. `./run.sh --mock` to confirm the stack still boots.
4. Pick the next un-done milestone above (M1 unless the user redirects).
5. Plan. Confirm scope with the user before writing code if the
   milestone is non-trivial.
6. Commit per the conventions in §Conventions. Don't push without
   explicit authorization.
