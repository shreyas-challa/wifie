import { useEffect, useRef } from "react";
import { useTheme } from "../lib/theme";
import { cn } from "../lib/utils";

const LIGHT = {
  bg: "rgba(250, 250, 250, 1)",
  grid: "rgba(24, 24, 27, 0.06)",
  axis: "rgba(24, 24, 27, 0.18)",
  line: "rgba(24, 24, 27, 0.85)",
  fill: "rgba(24, 24, 27, 0.06)"
};

const DARK = {
  bg: "rgba(20, 20, 23, 1)",
  grid: "rgba(255, 255, 255, 0.05)",
  axis: "rgba(255, 255, 255, 0.18)",
  line: "rgba(245, 245, 245, 0.95)",
  fill: "rgba(255, 255, 255, 0.05)"
};

export function PacketCanvas({ ticks, className }) {
  const canvasRef = useRef(null);
  const { resolvedTheme } = useTheme();

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const rect = canvas.getBoundingClientRect();
    if (
      canvas.width !== Math.round(rect.width * dpr) ||
      canvas.height !== Math.round(rect.height * dpr)
    ) {
      canvas.width = Math.round(rect.width * dpr);
      canvas.height = Math.round(rect.height * dpr);
    }

    const ctx = canvas.getContext("2d");
    const palette = resolvedTheme === "dark" ? DARK : LIGHT;
    const width = canvas.width;
    const height = canvas.height;

    ctx.fillStyle = palette.bg;
    ctx.fillRect(0, 0, width, height);

    ctx.lineWidth = 1 * dpr;
    ctx.strokeStyle = palette.grid;
    for (let i = 1; i < 5; i += 1) {
      const y = Math.round((height / 5) * i) + 0.5;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(width, y);
      ctx.stroke();
    }
    for (let i = 1; i < 8; i += 1) {
      const x = Math.round((width / 8) * i) + 0.5;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, height);
      ctx.stroke();
    }

    if (ticks.length < 2) return;

    const padding = 8 * dpr;
    const innerH = height - padding * 2;
    const maxPps = Math.max(1000, ...ticks.map((t) => t.packets_per_second));

    ctx.lineWidth = 1.6 * dpr;
    ctx.strokeStyle = palette.line;
    ctx.fillStyle = palette.fill;

    const pts = ticks.map((tick, idx) => {
      const x = (idx / (ticks.length - 1)) * width;
      const y = height - padding - (tick.packets_per_second / maxPps) * innerH;
      return [x, y];
    });

    ctx.beginPath();
    ctx.moveTo(pts[0][0], height);
    pts.forEach(([x, y]) => ctx.lineTo(x, y));
    ctx.lineTo(pts[pts.length - 1][0], height);
    ctx.closePath();
    ctx.fill();

    ctx.beginPath();
    pts.forEach(([x, y], idx) => {
      if (idx === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();
  }, [ticks, resolvedTheme]);

  return (
    <canvas
      ref={canvasRef}
      className={cn(
        "block h-72 w-full rounded-2xl border border-border bg-card",
        className
      )}
    />
  );
}
