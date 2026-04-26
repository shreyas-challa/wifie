import { useRef, useState } from "react";
import {
  AnimatePresence,
  motion,
  useMotionValue,
  useSpring,
  useTransform
} from "motion/react";
import { IconLayoutNavbarCollapse } from "@tabler/icons-react";
import { cn } from "../lib/utils";

export function FloatingDock({ items, className, mobileClassName }) {
  return (
    <>
      <FloatingDockDesktop items={items} className={className} />
      <FloatingDockMobile items={items} className={mobileClassName} />
    </>
  );
}

function FloatingDockDesktop({ items, className }) {
  const mouseX = useMotionValue(Infinity);
  return (
    <motion.div
      onMouseMove={(e) => mouseX.set(e.pageX)}
      onMouseLeave={() => mouseX.set(Infinity)}
      className={cn(
        "mx-auto hidden h-16 items-end gap-4 rounded-2xl bg-gray-100 px-4 pb-3 dark:bg-neutral-900 md:flex",
        className
      )}
    >
      {items.map((item) => (
        <DockIcon key={item.id} mouseX={mouseX} {...item} />
      ))}
    </motion.div>
  );
}

function DockIcon({ mouseX, label, icon: Icon, active, onSelect }) {
  const ref = useRef(null);

  const distance = useTransform(mouseX, (val) => {
    const bounds = ref.current?.getBoundingClientRect() ?? { x: 0, width: 0 };
    return val - bounds.x - bounds.width / 2;
  });

  const widthSync = useTransform(distance, [-150, 0, 150], [40, 80, 40]);
  const heightSync = useTransform(distance, [-150, 0, 150], [40, 80, 40]);
  const widthIcon = useTransform(distance, [-150, 0, 150], [20, 40, 20]);
  const heightIcon = useTransform(distance, [-150, 0, 150], [20, 40, 20]);

  const width = useSpring(widthSync, { mass: 0.1, stiffness: 150, damping: 12 });
  const height = useSpring(heightSync, { mass: 0.1, stiffness: 150, damping: 12 });
  const iconWidth = useSpring(widthIcon, { mass: 0.1, stiffness: 150, damping: 12 });
  const iconHeight = useSpring(heightIcon, { mass: 0.1, stiffness: 150, damping: 12 });

  const [hovered, setHovered] = useState(false);

  return (
    <button type="button" onClick={onSelect} className="appearance-none">
      <motion.div
        ref={ref}
        style={{ width, height }}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        className={cn(
          "relative flex aspect-square items-center justify-center rounded-full",
          "bg-gray-200 dark:bg-neutral-800",
          active && "ring-2 ring-foreground/40"
        )}
      >
        <AnimatePresence>
          {hovered && (
            <motion.div
              initial={{ opacity: 0, y: 6, x: "-50%" }}
              animate={{ opacity: 1, y: 0, x: "-50%" }}
              exit={{ opacity: 0, y: 4, x: "-50%" }}
              className="absolute -top-9 left-1/2 w-fit whitespace-pre rounded-md border border-border bg-gray-100 px-2 py-0.5 text-xs text-neutral-700 dark:bg-neutral-800 dark:text-neutral-200"
            >
              {label}
            </motion.div>
          )}
        </AnimatePresence>
        <motion.div
          style={{ width: iconWidth, height: iconHeight }}
          className="flex items-center justify-center text-neutral-500 dark:text-neutral-300"
        >
          <Icon className="h-full w-full" stroke={1.75} />
        </motion.div>
      </motion.div>
    </button>
  );
}

function FloatingDockMobile({ items, className }) {
  const [open, setOpen] = useState(false);
  return (
    <div className={cn("relative block md:hidden", className)}>
      <AnimatePresence>
        {open && (
          <motion.div
            layoutId="dock-mobile"
            className="absolute bottom-full mb-2 inset-x-0 flex flex-col items-center gap-2"
          >
            {items.map((item, idx) => (
              <motion.div
                key={item.id}
                initial={{ opacity: 0, y: 10, scale: 0.5 }}
                animate={{ opacity: 1, y: 0, scale: 1 }}
                exit={{
                  opacity: 0,
                  y: 10,
                  scale: 0.5,
                  transition: { delay: idx * 0.04 }
                }}
                transition={{ delay: (items.length - 1 - idx) * 0.04 }}
              >
                <button
                  type="button"
                  onClick={() => {
                    item.onSelect?.();
                    setOpen(false);
                  }}
                  className="flex h-10 w-10 items-center justify-center rounded-full bg-gray-100 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-200"
                  aria-label={item.label}
                >
                  <item.icon className="h-5 w-5" stroke={1.75} />
                </button>
              </motion.div>
            ))}
          </motion.div>
        )}
      </AnimatePresence>
      <button
        type="button"
        onClick={() => setOpen((s) => !s)}
        className="flex h-10 w-10 items-center justify-center rounded-full bg-gray-100 text-neutral-700 dark:bg-neutral-800 dark:text-neutral-200"
        aria-label="Open navigation"
      >
        <IconLayoutNavbarCollapse className="h-5 w-5" stroke={1.75} />
      </button>
    </div>
  );
}
