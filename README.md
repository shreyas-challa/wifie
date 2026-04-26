# WiFie (MVP Scaffold)

Lean full-stack scaffold for a local-host wireless security testing dashboard.

## Stack
- **Backend**: Rust (`axum` + `socketioxide` + `pcap` + `netlink_wi`)
- **Frontend**: React + Vite + Tailwind + Canvas graphing

## Run
### Backend
```bash
cargo run
```

### Frontend
```bash
cd frontend
npm install
npm run dev
```

## Notes
- The offensive security logic (deauth/frame injection and exploit chains) is intentionally left as **stubbed lab hooks**.
- API and UI wiring are in place so your team can plug in authorized experiment implementations.
- Could not locate a design artifact in this repo; the UI currently uses a lean dark console-inspired layout.
