# Contributing to WiFie

Thanks for taking the time to read this. WiFie is a small,
research-oriented project — the goal is a Rust-native, hardware-agnostic
console for Wi-Fi 6 / 6E / 7 audit work in a controlled academic lab.
Anything that helps that goal is in scope; anything that turns this
into a generalist offensive toolkit is not.

## Ground rules

1. **Lab use only.** Every offensive code path is gated by the
   `WIFIE_LAB_AUTHORIZED_BSSIDS` allowlist. New offensive code must
   call `auth::is_authorized()` before doing the thing — no exceptions.
2. **Golden Stack.** The architecture is fixed (Rust + axum +
   `netlink_wi` + `pcap` + `socketioxide` on the backend; Vite + plain
   React + Canvas on the frontend). Don't introduce Next.js, SSR, SVG
   plotting, or shell wrappers around `airmon-ng` — those choices are
   load-bearing. See `CLAUDE.md` and `DESIGN_SYSTEM.md` for the
   rationale.
3. **Subprocesses are for offline heavy work only.** `hashcat`,
   `aircrack-ng`, `hcxpcapngtool`, `dragondrain-ng`, the Vanhoef
   krack-* scripts. Read paths and mode/channel writes go through the
   `WirelessBackend` trait.
4. **No proprietary hardware deps.** If a feature only works with one
   vendor's adapter, gate it and document it.

## Local dev

```bash
git clone https://github.com/shreyas-challa/wifie
cd wifie
./run.sh --mock         # demo path, no caps, no real radio
./run.sh                # real nl80211 backend (Linux)
./run.sh --build-only   # compile both halves and exit
```

System packages (Fedora):

```bash
sudo dnf install libpcap-devel pkgconf-pkg-config
# optional: hashcat, aircrack-ng, hcxtools, iw
```

CI runs `cargo build`, `cargo clippy -- -D warnings`, and `npm run
build` in `frontend/`. Reproduce that locally before opening a PR.

## Commits & PRs

* Subject prefixes in active use: `feat(...)`, `fix(...)`,
  `refactor(...)`, `docs:`, `build(deps):`, `chore:`.
* Each commit should compile cleanly. Don't squash unrelated work.
* Keep PRs scoped — one milestone-sized change at a time.
* If a change touches the safety contract (auth gate, deauth,
  injection), call that out in the PR description.

## Good first issues

Pulled from `CLAUDE.md` § Next milestones. These are concrete,
self-contained, and don't require deep familiarity with the rest of
the codebase.

* **Demo pcap for the mock backend.** Check in a small `.pcap` and
  have `MockBackend` replay it through the same `CapturedFrame`
  channel the real backend uses, so the dashboard shows live-ish
  telemetry without a radio plugged in.
* **Clipboard copy on artifact paths.** When a capture completes,
  show the absolute path next to the download chip with a one-click
  copy button.
* **`/api/captures/handshake` filtering.** Add `?status=complete`
  and `?since=<rfc3339>` query params so the UI can drop in-flight
  pollers as captures pile up.
* **Hardware compatibility doc.** A short page in the repo listing
  adapters confirmed to work for monitor mode + injection, with the
  driver / firmware combo each was tested on.
* **`hcxpcapngtool` version probe in `/health`.** Surface whether
  the conversion tool is on PATH at boot so the UI can warn before a
  capture runs, not after.

## Code style

* Rust: `cargo fmt` before committing. Errors flow through
  `HwError` (`src/hw/mod.rs`) — don't leak vendor types past the
  trait boundary.
* React: monochrome zinc visual language per `DESIGN_SYSTEM.md`.
  No emoji in UI strings, no purple/violet gradients, Tabler / Lucide
  icons only.
* Comments: load-bearing only. Explain *why* the code is unusual,
  not *what* it does.

## Reporting security issues

If you find a way to bypass the authorization gate, escape the
captures directory in artifact downloads, or otherwise turn a lab
tool into a real-world weapon, please open a private security
advisory on GitHub instead of a public issue.
