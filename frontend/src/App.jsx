import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { io } from "socket.io-client";
import {
  IconAntennaBars5,
  IconKey,
  IconShieldHalfFilled,
  IconActivity,
  IconBook
} from "@tabler/icons-react";

import { ThemeProvider } from "./lib/theme";
import { AnimatedThemeToggler } from "./components/AnimatedThemeToggler";
import { FloatingDock } from "./components/FloatingDock";
import { MinimalCard, MinimalCardHeader } from "./components/MinimalCard";
import { PacketCanvas } from "./components/PacketCanvas";
import { EncryptedText } from "./components/EncryptedText";
import { RevealOnScroll } from "./components/RevealOnScroll";
import { Tag } from "./components/Field";
import { InterfacePanel } from "./components/InterfacePanel";
import { HandshakePanel } from "./components/HandshakePanel";
import { VulnLabPanel } from "./components/VulnLabPanel";

// Same-origin: API and socket.io requests are proxied to the Rust backend
// by Vite in dev (vite.config.js) and by the binary itself in prod.
const API_BASE = "";

const SECTIONS = [
  { id: "telemetry", label: "Telemetry", icon: IconActivity },
  { id: "interfaces", label: "Interfaces", icon: IconAntennaBars5 },
  { id: "capture", label: "Capture", icon: IconKey },
  { id: "vuln-lab", label: "Vuln Lab", icon: IconShieldHalfFilled },
  { id: "notes", label: "Notes", icon: IconBook }
];

function scrollToSection(id) {
  const el = document.getElementById(id);
  if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
}

function Dashboard() {
  const [socketConnected, setSocketConnected] = useState(false);
  const [packetTicks, setPacketTicks] = useState([]);
  const [interfaces, setInterfaces] = useState([]);
  const [selectedInterface, setSelectedInterface] = useState("wlan0");
  const [selectedFreq, setSelectedFreq] = useState(2412);
  const [targetBssid, setTargetBssid] = useState("AA:BB:CC:DD:EE:FF");
  const [targetClient, setTargetClient] = useState("");
  const [captureType, setCaptureType] = useState("wpa2_handshake");
  const [recentCaptures, setRecentCaptures] = useState([]);
  const [labAuth, setLabAuth] = useState({ env_var: "", bssids: [] });
  const [busy, setBusy] = useState(false);
  const [activeSection, setActiveSection] = useState("telemetry");

  const socket = useMemo(
    () => io("/events", { transports: ["websocket"] }),
    []
  );

  useEffect(() => {
    fetch(`${API_BASE}/api/interfaces`)
      .then((r) => r.json())
      .then((data) => {
        setInterfaces(data);
        if (data[0]?.name) setSelectedInterface(data[0].name);
        if (data[0]?.channel_mhz) setSelectedFreq(data[0].channel_mhz);
      })
      .catch(() => {
        setInterfaces([]);
      });

    fetch(`${API_BASE}/api/captures/handshake`)
      .then((r) => r.json())
      .then((tasks) => Array.isArray(tasks) && setRecentCaptures(tasks.slice(0, 6)))
      .catch(() => {});

    fetch(`${API_BASE}/api/auth/lab-bssids`)
      .then((r) => r.json())
      .then((data) => setLabAuth(data || { env_var: "", bssids: [] }))
      .catch(() => {});
  }, []);

  const refreshCaptures = useCallback(async () => {
    const fresh = await fetch(`${API_BASE}/api/captures/handshake`).then((r) => r.json());
    if (Array.isArray(fresh)) setRecentCaptures(fresh.slice(0, 6));
  }, []);

  useEffect(() => {
    socket.on("connect", () => setSocketConnected(true));
    socket.on("disconnect", () => setSocketConnected(false));
    socket.on("packet:tick", (tick) => {
      setPacketTicks((prev) => [...prev.slice(-119), tick]);
    });
    return () => {
      socket.removeAllListeners();
      socket.disconnect();
    };
  }, [socket]);

  useEffect(() => {
    const targets = SECTIONS.map((s) => document.getElementById(s.id)).filter(Boolean);
    if (!targets.length) return;
    const observer = new IntersectionObserver(
      (entries) => {
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => b.intersectionRatio - a.intersectionRatio);
        if (visible[0]) setActiveSection(visible[0].target.id);
      },
      { rootMargin: "-30% 0px -55% 0px", threshold: [0, 0.25, 0.5, 0.75, 1] }
    );
    targets.forEach((t) => observer.observe(t));
    return () => observer.disconnect();
  }, []);

  const post = useCallback(async (path, payload) => {
    setBusy(true);
    try {
      const res = await fetch(`${API_BASE}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(payload)
      });
      return await res.json();
    } finally {
      setBusy(false);
    }
  }, []);

  const refreshInterfaces = useCallback(async () => {
    const fresh = await fetch(`${API_BASE}/api/interfaces`).then((r) => r.json());
    setInterfaces(fresh);
  }, []);

  const handleToggleMonitor = async (enable) => {
    await post("/api/interfaces/monitor-mode", {
      interface: selectedInterface,
      enable
    });
    await refreshInterfaces();
  };

  const handleSetChannel = async () => {
    await post("/api/interfaces/channel", {
      interface: selectedInterface,
      frequency_mhz: selectedFreq
    });
    await refreshInterfaces();
  };

  const handleStartCapture = async () => {
    const out = await post("/api/captures/handshake", {
      interface: selectedInterface,
      target_bssid: targetBssid,
      target_client: targetClient || null,
      channel_mhz: selectedFreq,
      capture_type: captureType
    });
    if (out?.task) {
      setRecentCaptures((prev) => [out.task, ...prev].slice(0, 6));
      // Capture worker runs ~60s + a 22000 conversion; poll a couple
      // of times so the artifact download links appear without a
      // manual refresh.
      const interval = setInterval(refreshCaptures, 5000);
      setTimeout(() => clearInterval(interval), 90_000);
    }
  };

  const handleRunVuln = async (testCase) => {
    await post("/api/vuln-tests/start", {
      interface: selectedInterface,
      target_bssid: targetBssid,
      test_case: testCase
    });
  };

  const lastTick = packetTicks[packetTicks.length - 1];
  const dockItems = SECTIONS.map((s) => ({
    ...s,
    active: s.id === activeSection,
    onSelect: () => scrollToSection(s.id)
  }));

  return (
    <div className="relative flex min-h-screen w-full flex-col items-center bg-background text-foreground">
      <header className="sticky top-0 z-40 w-full border-b border-border/60 bg-background/80 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="mx-auto flex h-14 w-full max-w-[1200px] items-center justify-between px-6">
          <div className="flex items-center gap-2">
            <span className="inline-flex h-6 w-6 items-center justify-center rounded-md bg-foreground text-background">
              <span className="text-[10px] font-bold tracking-tight">W</span>
            </span>
            <span className="text-sm font-semibold tracking-tight">wifie</span>
            <span className="ml-2 hidden text-xs text-muted-foreground sm:inline">
              Wireless audit console — local lab build
            </span>
          </div>
          <div className="flex items-center gap-3">
            <Tag tone={socketConnected ? "monitor" : "danger"}>
              <span
                className={`mr-1 inline-block h-1.5 w-1.5 rounded-full ${
                  socketConnected ? "bg-background" : "bg-destructive"
                }`}
              />
              {socketConnected ? "WS live" : "WS offline"}
            </Tag>
            <AnimatedThemeToggler />
          </div>
        </div>
      </header>

      <main className="w-full max-w-[1200px] flex-1 px-6 pb-32 pt-12 sm:pt-16">
        <RevealOnScroll>
          <section className="mb-10">
            <p className="text-xs font-medium uppercase tracking-[0.18em] text-muted-foreground">
              Console
            </p>
            <h1 className="mt-3 text-4xl font-bold tracking-tight sm:text-5xl">
              <EncryptedText text="WIFIE" className="text-foreground" />
              <span className="ml-3 align-middle text-base font-normal text-muted-foreground">
                / Wi-Fi 6 · 6E · 7 audit
              </span>
            </h1>
            <p className="mt-3 max-w-xl text-sm text-muted-foreground">
              A local-host, hardware-agnostic dashboard for academic wireless
              penetration testing — Rust nl80211 control plane, Canvas-rendered
              telemetry, no proprietary boxes in the loop.
            </p>
          </section>
        </RevealOnScroll>

        <RevealOnScroll>
          <MinimalCard id="telemetry">
            <MinimalCardHeader
              icon={IconActivity}
              eyebrow="Live"
              title="Packet Telemetry"
              description="Canvas-driven packets-per-second graph, fed by the socketioxide /events namespace."
            />

            <PacketCanvas ticks={packetTicks} />

            <dl className="mt-5 grid grid-cols-2 gap-3 sm:grid-cols-4">
              <Stat
                label="pps (live)"
                value={lastTick?.packets_per_second ?? 0}
              />
              <Stat
                label="noise floor"
                value={lastTick ? `${lastTick.noise_floor_dbm} dBm` : "—"}
              />
              <Stat label="samples" value={packetTicks.length} />
              <Stat label="seq" value={lastTick?.seq ?? 0} />
            </dl>
          </MinimalCard>
        </RevealOnScroll>

        <div className="mt-8 grid gap-8 lg:grid-cols-2">
          <RevealOnScroll delay={75}>
            <InterfacePanel
              interfaces={interfaces}
              selectedInterface={selectedInterface}
              selectedFreq={selectedFreq}
              onSelectInterface={setSelectedInterface}
              onSelectFreq={setSelectedFreq}
              onToggleMonitor={handleToggleMonitor}
              onSetChannel={handleSetChannel}
              busy={busy}
            />
          </RevealOnScroll>

          <RevealOnScroll delay={150}>
            <HandshakePanel
              targetBssid={targetBssid}
              onTargetBssidChange={setTargetBssid}
              targetClient={targetClient}
              onTargetClientChange={setTargetClient}
              captureType={captureType}
              onCaptureTypeChange={setCaptureType}
              onStartCapture={handleStartCapture}
              recentCaptures={recentCaptures}
              busy={busy}
            />
          </RevealOnScroll>
        </div>

        <RevealOnScroll delay={75} className="mt-8 block">
          <VulnLabPanel onRunTest={handleRunVuln} busy={busy} />
        </RevealOnScroll>

        <RevealOnScroll delay={75} className="mt-8 block">
          <MinimalCard id="notes">
            <MinimalCardHeader
              icon={IconBook}
              eyebrow="Operator"
              title="Lab notes"
              description="Quick reference for what is real vs. stubbed in this build."
            />
            <ul className="space-y-2 text-sm text-muted-foreground">
              <li>
                <span className="font-medium text-foreground">Passive paths</span> —
                interface enumeration, monitor toggle, channel set, telemetry stream
                — are wired through axum + nl80211 abstractions on the Rust side.
              </li>
              <li>
                <span className="font-medium text-foreground">Active offence</span>
                — deauth injection, SAE timing probes, KRACK replay — is intentionally
                stubbed. Authorize, then implement inside the corresponding service
                modules.
              </li>
              <li>
                <span className="font-medium text-foreground">Hardware</span> — bring
                your own monitor-mode capable adapter (Alfa, ath9k_htc, mt76, etc.)
                and run the backend with CAP_NET_ADMIN + CAP_NET_RAW.
              </li>
            </ul>

            <div className="mt-5 rounded-lg border border-border bg-background/60 p-3 dark:bg-neutral-900/40">
              <div className="flex items-center justify-between gap-2">
                <p className="text-xs font-medium uppercase tracking-[0.16em] text-muted-foreground">
                  Lab authorization
                </p>
                <span className="font-mono text-[10px] text-muted-foreground">
                  {labAuth.env_var || "WIFIE_LAB_AUTHORIZED_BSSIDS"}
                </span>
              </div>
              {labAuth.bssids?.length ? (
                <ul className="mt-2 flex flex-wrap gap-1.5">
                  {labAuth.bssids.map((b) => (
                    <li
                      key={b}
                      className="rounded-md border border-border bg-background px-2 py-0.5 font-mono text-xs text-foreground dark:bg-neutral-950"
                    >
                      {b}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="mt-2 text-xs text-muted-foreground">
                  No BSSIDs authorized — offensive operations are blocked.
                  Set <span className="font-mono">{labAuth.env_var || "WIFIE_LAB_AUTHORIZED_BSSIDS"}</span> to
                  a comma-separated allowlist before starting the backend.
                </p>
              )}
            </div>
          </MinimalCard>
        </RevealOnScroll>
      </main>

      <div className="pointer-events-none fixed inset-x-0 bottom-3 z-50 flex justify-center">
        <div className="pointer-events-auto">
          <FloatingDock items={dockItems} />
        </div>
      </div>
    </div>
  );
}

function Stat({ label, value }) {
  return (
    <div className="rounded-xl border border-border bg-background/60 px-3 py-2.5 dark:bg-neutral-900/40">
      <p className="text-[10px] font-medium uppercase tracking-[0.16em] text-muted-foreground">
        {label}
      </p>
      <p className="mt-1 font-mono text-lg font-semibold text-foreground">{value}</p>
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider defaultTheme="system">
      <Dashboard />
    </ThemeProvider>
  );
}
