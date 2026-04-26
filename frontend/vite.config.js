import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const BACKEND = process.env.WIFIE_BACKEND_URL || "http://localhost:3000";
const BACKEND_WS = BACKEND.replace(/^http/, "ws");

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    host: true,
    proxy: {
      "/api": { target: BACKEND, changeOrigin: true },
      "/health": { target: BACKEND, changeOrigin: true },
      "/socket.io": { target: BACKEND_WS, ws: true, changeOrigin: true }
    }
  }
});
