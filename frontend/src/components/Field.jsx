import { cn } from "../lib/utils";

const SHADOW_INPUT =
  "shadow-[0px_2px_3px_-1px_rgba(0,0,0,0.05),0px_1px_0px_0px_rgba(25,28,33,0.02),0px_0px_0px_1px_rgba(25,28,33,0.08)]";

export function FieldLabel({ children, className }) {
  return (
    <span
      className={cn(
        "block text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground",
        className
      )}
    >
      {children}
    </span>
  );
}

export function TextInput({ className, ...rest }) {
  return (
    <input
      {...rest}
      className={cn(
        "h-10 w-full rounded-lg bg-background px-3 text-sm text-foreground outline-none",
        "placeholder:text-muted-foreground/70",
        "focus-visible:ring-[3px] focus-visible:ring-ring/40",
        "dark:bg-neutral-800/60",
        SHADOW_INPUT,
        className
      )}
    />
  );
}

export function Select({ className, children, ...rest }) {
  return (
    <select
      {...rest}
      className={cn(
        "wifie-select h-10 w-full appearance-none rounded-lg bg-background px-3 pr-9 text-sm text-foreground outline-none",
        "focus-visible:ring-[3px] focus-visible:ring-ring/40",
        "dark:bg-neutral-800/60",
        SHADOW_INPUT,
        className
      )}
    >
      {children}
    </select>
  );
}

export function Tag({ tone = "neutral", children, className }) {
  const tones = {
    neutral: "bg-secondary text-secondary-foreground",
    monitor: "bg-foreground text-background",
    danger: "bg-destructive/10 text-destructive"
  };
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[11px] font-medium tracking-tight",
        tones[tone],
        className
      )}
    >
      {children}
    </span>
  );
}
