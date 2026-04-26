import { useEffect, useMemo, useRef, useState } from "react";
import { io } from "socket.io-client";

const API_BASE = "http://localhost:3000";
const FREQUENCY_PRESETS = [2412, 2437, 2462, 5180, 5200, 5745, 5955, 6115, 6375];

export default function App() {
  const [socketConnected, setSocketConnected] = useState(false);
  const [packetTicks, setPacketTicks] = useState([]);
  const [interfaces, setInterfaces] = useState([]);
  const [selectedInterface, setSelectedInterface] = useState("wlan0");
  const [selectedFreq, setSelectedFreq] = useState(2412);
  const [targetBssid, setTargetBssid] = useState("AA:BB:CC:DD:EE:FF");

  const canvasRef = useRef(null);

  const socket = useMemo(() => io(`${API_BASE}/events`, { transports: ["websocket"] }), []);

  useEffect(() => {
    fetch(`${API_BASE}/api/interfaces`)
      .then((res) => res.json())
      .then((data) => {
        setInterfaces(data);
        if (data[0]?.name) setSelectedInterface(data[0].name);
      })
      .catch(console.error);
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
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    const { width, height } = canvas;
    ctx.clearRect(0, 0, width, height);

    ctx.fillStyle = "#09090b";
    ctx.fillRect(0, 0, width, height);

    ctx.strokeStyle = "#1f2937";
    ctx.lineWidth = 1;
    for (let y = 0; y < 5; y += 1) {
      const gy = (height / 5) * y;
      ctx.beginPath();
      ctx.moveTo(0, gy);
      ctx.lineTo(width, gy);
      ctx.stroke();
    }

    if (packetTicks.length < 2) return;

    const maxPps = Math.max(...packetTicks.map((p) => p.packets_per_second), 1000);
    ctx.strokeStyle = "#22d3ee";
    ctx.lineWidth = 2;
    ctx.beginPath();

    packetTicks.forEach((tick, idx) => {
      const x = (idx / (packetTicks.length - 1)) * width;
      const y = height - (tick.packets_per_second / maxPps) * (height - 8) - 4;
      if (idx === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });

    ctx.stroke();
  }, [packetTicks]);

  async function post(path, payload) {
    const res = await fetch(`${API_BASE}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload)
    });
    return res.json();
  }

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100 p-6">
      <div className="mx-auto grid max-w-7xl gap-6 lg:grid-cols-3">
        <section className="rounded-2xl border border-zinc-800 bg-zinc-900 p-5 lg:col-span-2">
          <header className="mb-4 flex items-center justify-between">
            <h1 className="text-xl font-semibold text-accent-400">WiFie Live Telemetry</h1>
            <span className={`rounded-full px-3 py-1 text-xs ${socketConnected ? "bg-emerald-600/30 text-emerald-300" : "bg-rose-600/30 text-rose-300"}`}>
              {socketConnected ? "WS Connected" : "WS Disconnected"}
            </span>
          </header>
          <canvas ref={canvasRef} width={1000} height={320} className="h-80 w-full rounded-lg border border-zinc-800" />
          <p className="mt-3 text-sm text-zinc-400">Canvas-rendered packets/second graph (optimized for high-frequency streams).</p>
        </section>

        <section className="rounded-2xl border border-zinc-800 bg-zinc-900 p-5">
          <h2 className="mb-4 text-lg font-semibold">Advanced Interface Management</h2>
          <div className="space-y-3 text-sm">
            <label className="block">
              Interface
              <select value={selectedInterface} onChange={(e) => setSelectedInterface(e.target.value)} className="mt-1 w-full rounded bg-zinc-800 px-3 py-2">
                {interfaces.map((i) => <option key={i.name}>{i.name}</option>)}
              </select>
            </label>

            <label className="block">
              Frequency (MHz)
              <select value={selectedFreq} onChange={(e) => setSelectedFreq(Number(e.target.value))} className="mt-1 w-full rounded bg-zinc-800 px-3 py-2">
                {FREQUENCY_PRESETS.map((f) => <option key={f} value={f}>{f}</option>)}
              </select>
            </label>

            <div className="grid grid-cols-2 gap-2">
              <button className="rounded bg-accent-500 px-3 py-2 font-medium text-zinc-900" onClick={() => post("/api/interfaces/monitor-mode", { interface: selectedInterface, enable: true })}>Enable Monitor</button>
              <button className="rounded bg-zinc-700 px-3 py-2" onClick={() => post("/api/interfaces/channel", { interface: selectedInterface, frequency_mhz: selectedFreq })}>Set Channel</button>
            </div>
          </div>

          <h2 className="mb-3 mt-6 text-lg font-semibold">Handshake Capture</h2>
          <input value={targetBssid} onChange={(e) => setTargetBssid(e.target.value)} className="w-full rounded bg-zinc-800 px-3 py-2 text-sm" />
          <button
            className="mt-2 w-full rounded bg-indigo-500 px-3 py-2 text-sm font-medium"
            onClick={() =>
              post("/api/captures/handshake", {
                interface: selectedInterface,
                target_bssid: targetBssid,
                channel_mhz: selectedFreq,
                capture_type: "wpa2_handshake"
              })
            }
          >
            Start Capture (Stub)
          </button>

          <h2 className="mb-3 mt-6 text-lg font-semibold">Vulnerability Re-implementation (Lab)</h2>
          <div className="grid gap-2">
            <button className="rounded bg-amber-500 px-3 py-2 text-sm font-medium text-zinc-900" onClick={() => post("/api/vuln-tests/start", { interface: selectedInterface, target_bssid: targetBssid, test_case: "dragonblood_sae_timing_stub" })}>Dragonblood/SAE Stub</button>
            <button className="rounded bg-rose-500 px-3 py-2 text-sm font-medium" onClick={() => post("/api/vuln-tests/start", { interface: selectedInterface, target_bssid: targetBssid, test_case: "krack_4way_replay_stub" })}>KRACK Stub</button>
          </div>
        </section>
      </div>
    </main>
  );
}
