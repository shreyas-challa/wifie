import { IconKey, IconClipboardList, IconDownload } from "@tabler/icons-react";
import { MinimalCard, MinimalCardHeader } from "./MinimalCard";
import { RippleButton } from "./RippleButton";
import { FieldLabel, Select, Tag, TextInput } from "./Field";

const CAPTURE_TYPES = [
  { id: "wpa2_handshake", label: "WPA2 4-way handshake" },
  { id: "wpa3_pmkid", label: "WPA3 PMKID" }
];

function ArtifactLink({ taskId, kind, label }) {
  return (
    <a
      href={`/api/captures/handshake/${taskId}/artifact?kind=${kind}`}
      className="inline-flex items-center gap-1 rounded-md border border-border bg-background/60 px-2 py-1 text-xs font-medium text-foreground transition hover:bg-foreground hover:text-background dark:bg-neutral-900/40"
    >
      <IconDownload className="h-3 w-3" stroke={2} />
      {label}
    </a>
  );
}

export function HandshakePanel({
  targetBssid,
  onTargetBssidChange,
  targetClient,
  onTargetClientChange,
  captureType,
  onCaptureTypeChange,
  onStartCapture,
  recentCaptures,
  busy
}) {
  return (
    <MinimalCard id="capture">
      <MinimalCardHeader
        icon={IconKey}
        eyebrow="Module 02"
        title="Automated Handshake Capture"
        description="Trigger targeted deauths, watch the reconnection, save the handshake artifact locally."
      />

      <div className="grid gap-4 md:grid-cols-2">
        <label className="space-y-1.5">
          <FieldLabel>Target BSSID</FieldLabel>
          <TextInput
            value={targetBssid}
            onChange={(e) => onTargetBssidChange(e.target.value)}
            placeholder="AA:BB:CC:DD:EE:FF"
            spellCheck={false}
          />
        </label>
        <label className="space-y-1.5">
          <FieldLabel>Client (optional)</FieldLabel>
          <TextInput
            value={targetClient}
            onChange={(e) => onTargetClientChange(e.target.value)}
            placeholder="Leave empty for passive capture"
            spellCheck={false}
          />
        </label>
        <label className="space-y-1.5 md:col-span-2">
          <FieldLabel>Capture mode</FieldLabel>
          <Select
            value={captureType}
            onChange={(e) => onCaptureTypeChange(e.target.value)}
          >
            {CAPTURE_TYPES.map((c) => (
              <option key={c.id} value={c.id}>
                {c.label}
              </option>
            ))}
          </Select>
        </label>
      </div>

      <div className="mt-5 flex flex-wrap gap-3">
        <RippleButton variant="primary" onClick={onStartCapture} disabled={busy}>
          Queue capture
        </RippleButton>
        <RippleButton variant="ghost" onClick={() => onTargetClientChange("")}>
          Reset client
        </RippleButton>
      </div>

      <div className="mt-6">
        <div className="flex items-center gap-2 text-xs uppercase tracking-[0.14em] text-muted-foreground">
          <IconClipboardList className="h-3.5 w-3.5" stroke={1.75} />
          Recent tasks
        </div>
        <ul className="mt-3 space-y-2">
          {recentCaptures.length === 0 ? (
            <li className="rounded-lg border border-dashed border-border px-3 py-2 text-sm text-muted-foreground">
              No captures queued yet.
            </li>
          ) : (
            recentCaptures.map((task) => {
              const hasPcap = !!task.artifact_path;
              const has22000 = !!task.hashcat_22000_path;
              const isComplete = task.status === "complete";
              return (
                <li
                  key={task.id}
                  className="rounded-lg border border-border bg-background/60 px-3 py-2 text-sm dark:bg-neutral-900/40"
                >
                  <div className="flex items-center justify-between gap-3">
                    <div className="min-w-0">
                      <p className="truncate font-medium text-foreground">{task.target_bssid}</p>
                      <p className="text-xs text-muted-foreground">
                        {task.capture_type} · {task.interface} · {task.eapol_frames_seen ?? 0} EAPOL
                      </p>
                    </div>
                    <Tag tone={task.status === "failed" ? "danger" : "neutral"}>
                      {task.status}
                    </Tag>
                  </div>
                  {(isComplete && hasPcap) || has22000 ? (
                    <div className="mt-2 flex flex-wrap gap-2">
                      {isComplete && hasPcap && (
                        <ArtifactLink taskId={task.id} kind="pcap" label=".pcap" />
                      )}
                      {has22000 && (
                        <ArtifactLink taskId={task.id} kind="22000" label=".22000" />
                      )}
                    </div>
                  ) : null}
                  {task.error && (
                    <p className="mt-2 text-xs text-destructive">{task.error}</p>
                  )}
                  {task.conversion_error && (
                    <p className="mt-1 text-xs text-muted-foreground">
                      hashcat 22000: {task.conversion_error}
                    </p>
                  )}
                </li>
              );
            })
          )}
        </ul>
      </div>
    </MinimalCard>
  );
}
