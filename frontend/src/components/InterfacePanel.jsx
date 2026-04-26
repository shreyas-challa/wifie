import { IconAntennaBars5, IconBroadcast, IconCircuitResistor } from "@tabler/icons-react";
import { MinimalCard, MinimalCardHeader } from "./MinimalCard";
import { RippleButton } from "./RippleButton";
import { FieldLabel, Select, Tag } from "./Field";
import { cn } from "../lib/utils";

const FREQUENCY_PRESETS = {
  "2.4 GHz": [2412, 2437, 2462],
  "5 GHz": [5180, 5200, 5500, 5745],
  "6 GHz": [5955, 6115, 6375, 6855]
};

export function InterfacePanel({
  interfaces,
  selectedInterface,
  selectedFreq,
  onSelectInterface,
  onSelectFreq,
  onToggleMonitor,
  onSetChannel,
  busy
}) {
  const current = interfaces.find((i) => i.name === selectedInterface);
  const isMonitor = current?.mode === "monitor";

  return (
    <MinimalCard id="interfaces">
      <MinimalCardHeader
        icon={IconAntennaBars5}
        eyebrow="Module 01"
        title="Advanced Interface Management"
        description="Bind, unbind, and pin radios across 2.4, 5, and 6 GHz via nl80211."
      />

      <div className="grid gap-4 md:grid-cols-2">
        <label className="space-y-1.5">
          <FieldLabel>Adapter</FieldLabel>
          <Select
            value={selectedInterface}
            onChange={(e) => onSelectInterface(e.target.value)}
          >
            {interfaces.map((i) => (
              <option key={i.name} value={i.name}>
                {i.name} — {i.phy}
              </option>
            ))}
          </Select>
        </label>

        <label className="space-y-1.5">
          <FieldLabel>Frequency</FieldLabel>
          <Select
            value={selectedFreq}
            onChange={(e) => onSelectFreq(Number(e.target.value))}
          >
            {Object.entries(FREQUENCY_PRESETS).map(([band, freqs]) => (
              <optgroup key={band} label={band}>
                {freqs.map((f) => (
                  <option key={f} value={f}>
                    {f} MHz
                  </option>
                ))}
              </optgroup>
            ))}
          </Select>
        </label>
      </div>

      <div className="mt-5 flex flex-wrap items-center gap-2">
        <Tag tone={isMonitor ? "monitor" : "neutral"}>
          <IconBroadcast className="h-3 w-3" stroke={2} />
          {isMonitor ? "Monitor" : "Managed"}
        </Tag>
        {current?.channel_mhz ? (
          <Tag>
            <IconCircuitResistor className="h-3 w-3" stroke={2} />
            {current.channel_mhz} MHz
          </Tag>
        ) : null}
        {current?.supported_bands_ghz?.map((b) => (
          <Tag key={b}>{b} GHz</Tag>
        ))}
      </div>

      <div className="mt-5 flex flex-wrap gap-3">
        <RippleButton
          variant="primary"
          onClick={() => onToggleMonitor(!isMonitor)}
          disabled={busy || !current}
        >
          {isMonitor ? "Disable Monitor" : "Enable Monitor"}
        </RippleButton>
        <RippleButton variant="outline" onClick={onSetChannel} disabled={busy || !current}>
          Set Channel
        </RippleButton>
      </div>

      <p className={cn("mt-4 text-xs text-muted-foreground")}>
        Hardware control is delegated to the Rust backend through the netlink_wi crate — no
        airmon-ng or external scripts.
      </p>
    </MinimalCard>
  );
}
