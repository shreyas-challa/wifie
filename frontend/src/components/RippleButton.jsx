import { forwardRef, useCallback, useRef, useState } from "react";
import { cn } from "../lib/utils";

let RIPPLE_ID = 0;

const VARIANTS = {
  primary:
    "bg-primary text-primary-foreground border-transparent hover:bg-primary/90",
  outline:
    "bg-background text-foreground border-border hover:bg-accent hover:text-accent-foreground dark:bg-input/30 dark:hover:bg-input/50",
  ghost:
    "bg-transparent text-foreground border-transparent hover:bg-accent hover:text-accent-foreground",
  danger:
    "bg-background text-destructive border-destructive/30 hover:bg-destructive/10"
};

const SIZES = {
  sm: "h-8 px-3 text-sm rounded-md",
  md: "h-9 px-4 text-sm rounded-lg",
  lg: "h-10 px-6 text-sm rounded-lg"
};

export const RippleButton = forwardRef(function RippleButton(
  {
    children,
    className,
    variant = "outline",
    size = "md",
    rippleColor = "#ADD8E6",
    onClick,
    type = "button",
    disabled,
    ...rest
  },
  ref
) {
  const containerRef = useRef(null);
  const [ripples, setRipples] = useState([]);

  const setRefs = useCallback(
    (node) => {
      containerRef.current = node;
      if (typeof ref === "function") ref(node);
      else if (ref) ref.current = node;
    },
    [ref]
  );

  const handleClick = (event) => {
    if (disabled) return;
    const node = containerRef.current;
    if (node) {
      const rect = node.getBoundingClientRect();
      const size = Math.max(rect.width, rect.height) * 1.6;
      const id = RIPPLE_ID++;
      const ripple = {
        id,
        x: event.clientX - rect.left - size / 2,
        y: event.clientY - rect.top - size / 2,
        size
      };
      setRipples((prev) => [...prev, ripple]);
      window.setTimeout(() => {
        setRipples((prev) => prev.filter((r) => r.id !== id));
      }, 650);
    }
    onClick?.(event);
  };

  return (
    <button
      ref={setRefs}
      type={type}
      disabled={disabled}
      onClick={handleClick}
      className={cn(
        "relative inline-flex select-none items-center justify-center gap-2 overflow-hidden whitespace-nowrap border font-medium",
        "transition-all duration-200 focus-visible:outline-none focus-visible:ring-[3px] focus-visible:ring-ring/50",
        "disabled:pointer-events-none disabled:opacity-50",
        SIZES[size],
        VARIANTS[variant],
        className
      )}
      {...rest}
    >
      <span className="relative z-10 inline-flex items-center gap-2">{children}</span>
      <span className="pointer-events-none absolute inset-0 z-0">
        {ripples.map((r) => (
          <span
            key={r.id}
            style={{
              left: r.x,
              top: r.y,
              width: r.size,
              height: r.size,
              backgroundColor: rippleColor
            }}
            className="absolute rounded-full opacity-30 [animation:wifie-ripple_600ms_ease-out_forwards]"
          />
        ))}
      </span>
      <style>{`@keyframes wifie-ripple { from { transform: scale(0); opacity: 0.35; } to { transform: scale(1); opacity: 0; } }`}</style>
    </button>
  );
});
