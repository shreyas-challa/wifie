import { cn } from "../lib/utils";

const SHADOW_LIGHT =
  "shadow-[0px_1px_1px_0px_rgba(0,0,0,0.05),0px_1px_1px_0px_rgba(255,252,240,0.5)_inset,0px_0px_0px_1px_hsla(0,0%,100%,0.1)_inset,0px_0px_1px_0px_rgba(28,27,26,0.5)]";
const SHADOW_RING =
  "shadow-[rgba(17,24,28,0.08)_0_0_0_1px,rgba(17,24,28,0.08)_0_1px_2px_-1px,rgba(17,24,28,0.04)_0_2px_4px]";
const SHADOW_DARK =
  "dark:shadow-[0_1px_0_0_rgba(255,255,255,0.03)_inset,0_0_0_1px_rgba(255,255,255,0.03)_inset,0_0_0_1px_rgba(0,0,0,0.1),0_2px_2px_0_rgba(0,0,0,0.1),0_4px_4px_0_rgba(0,0,0,0.1),0_8px_8px_0_rgba(0,0,0,0.1)]";

export function MinimalCard({ as: Tag = "section", className, children, ...rest }) {
  return (
    <Tag
      className={cn(
        "group rounded-[24px] bg-neutral-50 p-6 transition-colors duration-200",
        "hover:bg-neutral-100 dark:bg-neutral-900 dark:hover:bg-neutral-900/80",
        SHADOW_LIGHT,
        SHADOW_RING,
        SHADOW_DARK,
        className
      )}
      {...rest}
    >
      {children}
    </Tag>
  );
}

export function MinimalCardHeader({ icon: Icon, eyebrow, title, description, className }) {
  return (
    <header className={cn("mb-5 flex items-start gap-3", className)}>
      {Icon ? (
        <span
          className={cn(
            "mt-0.5 inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-full",
            "bg-secondary text-secondary-foreground"
          )}
        >
          <Icon className="h-4 w-4" stroke={1.75} />
        </span>
      ) : null}
      <div className="min-w-0 flex-1">
        {eyebrow ? (
          <p className="text-xs font-medium uppercase tracking-[0.14em] text-muted-foreground">
            {eyebrow}
          </p>
        ) : null}
        <h2 className="mt-1 text-lg font-semibold leading-tight tracking-tight text-foreground">
          {title}
        </h2>
        {description ? (
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        ) : null}
      </div>
    </header>
  );
}
