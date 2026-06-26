"use client";

import { useEffect, useRef, useState, type ReactNode } from "react";

/** Reveal-on-scroll wrapper. Animates transform + opacity once, on entry. */
export function Reveal({
  children,
  i = 0,
  as: Tag = "div",
  className = "",
}: {
  children: ReactNode;
  i?: number;
  as?: "div" | "section" | "li" | "article";
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [shown, setShown] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          setShown(true);
          io.disconnect();
        }
      },
      { threshold: 0.12, rootMargin: "0px 0px -8% 0px" },
    );
    io.observe(el);
    return () => io.disconnect();
  }, []);

  return (
    <Tag
      ref={ref as never}
      className={`reveal ${shown ? "in" : ""} ${className}`}
      style={{ "--i": i } as React.CSSProperties}
    >
      {children}
    </Tag>
  );
}
