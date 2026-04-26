import { IconShieldHalfFilled, IconAlertTriangle } from "@tabler/icons-react";
import { MinimalCard, MinimalCardHeader } from "./MinimalCard";
import { RippleButton } from "./RippleButton";

const TESTS = [
  {
    id: "dragonblood_sae_timing",
    title: "Dragonblood / SAE timing",
    description:
      "Controlled SAE handshake downgrade probe to evaluate WPA3 client resilience. Wraps dragondrain-ng. Lab use only."
  },
  {
    id: "krack_4way_replay",
    title: "KRACK 4-way replay",
    description:
      "Evil-twin 4-way handshake manipulation against the in-lab AP to replay key frames. Wraps krack-ft-test.py."
  }
];

export function VulnLabPanel({ onRunTest, busy }) {
  return (
    <MinimalCard id="vuln-lab">
      <MinimalCardHeader
        icon={IconShieldHalfFilled}
        eyebrow="Module 03"
        title="Vulnerability Re-implementation"
        description="Reproduce known weaknesses against opt-in lab APs to test modern client behaviour."
      />

      <ul className="space-y-3">
        {TESTS.map((t) => (
          <li
            key={t.id}
            className="flex flex-col gap-3 rounded-xl border border-border bg-background/60 p-4 sm:flex-row sm:items-center sm:justify-between dark:bg-neutral-900/40"
          >
            <div className="min-w-0">
              <p className="text-sm font-semibold text-foreground">{t.title}</p>
              <p className="mt-0.5 text-xs text-muted-foreground">{t.description}</p>
            </div>
            <RippleButton
              variant="outline"
              size="sm"
              onClick={() => onRunTest(t.id)}
              disabled={busy}
            >
              Run stub
            </RippleButton>
          </li>
        ))}
      </ul>

      <div className="mt-5 flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-xs text-destructive">
        <IconAlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" stroke={2} />
        <span>
          Stubs only. Active offensive logic is intentionally absent from this scaffold —
          wire it inside the Rust workspace under explicit lab authorization.
        </span>
      </div>
    </MinimalCard>
  );
}
