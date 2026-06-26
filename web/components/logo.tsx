// A redstart is a bird (red-tailed songbird). Stylised swift-in-flight mark,
// with an optional ember-red tail accent.
export function Bird({
  className = "",
  glow = false,
}: {
  className?: string;
  glow?: boolean;
}) {
  return (
    <svg
      viewBox="0 0 32 32"
      aria-hidden
      className={className}
      style={glow ? { filter: "drop-shadow(0 0 10px rgba(255,51,85,0.55))" } : undefined}
    >
      {/* wings + body */}
      <path
        d="M16 17c-3.3 0-6.6-1.9-9-5-1-1.3-2.4-2.2-4-2.4 1.9-1.2 4-1 5.8.1 1.7 1 3.4 2 5.4 2.2-1.4-1.2-2.2-2.8-2.3-4.7 1.1 1.2 2.3 1.9 3.6 2.1.4-2.2 1.6-4 3.5-5.3-.2 1.3 0 2.5.6 3.6.6-1.1 1.5-2 2.7-2.6-.2 1.1-.1 2.1.3 3.1 1.1-.9 2.4-1.4 3.9-1.4-1 .8-1.6 1.8-1.8 3 1.6-.3 3.2 0 4.8 1-1.7.1-3 .9-4 2.3-2.3 3.1-5.6 5-9 5z"
        fill="currentColor"
      />
      {/* red tail flick */}
      <path
        d="M16 17c-1.1 1.9-2.7 3.2-4.8 3.9 1.3-1.6 2-3.2 2.2-4.9 1 .6 1.8.9 2.6 1z"
        fill="var(--color-ember, #ff7a45)"
      />
    </svg>
  );
}

export function Wordmark({ className = "" }: { className?: string }) {
  return (
    <span className={`inline-flex items-center gap-2 ${className}`}>
      <Bird className="h-[1.15em] w-[1.15em] text-red" />
      <span className="font-medium tracking-tight">Redstart</span>
    </span>
  );
}
