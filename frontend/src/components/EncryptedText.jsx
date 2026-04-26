import { useEffect, useRef, useState } from "react";
import { cn } from "../lib/utils";

const CHARSET = "ABCDEFGHJKLMNPQRSTUVWXYZ0123456789@#$%&*";

export function EncryptedText({
  text,
  className,
  duration = 1200,
  startOnMount = true
}) {
  const [output, setOutput] = useState(text.replace(/[^\s]/g, " "));
  const containerRef = useRef(null);
  const armed = useRef(false);

  useEffect(() => {
    if (!startOnMount) return;
    const node = containerRef.current;
    if (!node) return;

    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting && !armed.current) {
            armed.current = true;
            run();
            observer.disconnect();
          }
        }
      },
      { threshold: 0.4 }
    );
    observer.observe(node);
    return () => observer.disconnect();
  }, [startOnMount, text]);

  function run() {
    const start = performance.now();
    const total = text.length;

    function frame(now) {
      const progress = Math.min(1, (now - start) / duration);
      const revealCount = Math.floor(progress * total);
      let next = "";
      for (let i = 0; i < total; i += 1) {
        const ch = text[i];
        if (i < revealCount || ch === " ") {
          next += ch;
        } else {
          next += CHARSET[Math.floor(Math.random() * CHARSET.length)];
        }
      }
      setOutput(next);
      if (progress < 1) requestAnimationFrame(frame);
      else setOutput(text);
    }

    requestAnimationFrame(frame);
  }

  return (
    <span ref={containerRef} className={cn("font-mono tracking-tight", className)}>
      {output}
    </span>
  );
}
