// A hexagonal node-network — a nod to The Graph's mark, recoloured red and set
// in space. Outer ring of nodes + a glowing core, lines as the "graph".
export function Constellation({ className = "" }: { className?: string }) {
  const cx = 110;
  const cy = 110;
  const R = 78;
  const nodes = Array.from({ length: 6 }, (_, i) => {
    const a = (Math.PI / 3) * i - Math.PI / 2;
    return { x: cx + R * Math.cos(a), y: cy + R * Math.sin(a) };
  });
  return (
    <svg viewBox="0 0 220 220" className={className} aria-hidden>
      <defs>
        <radialGradient id="core" cx="50%" cy="50%" r="50%">
          <stop offset="0%" stopColor="#ff7a45" />
          <stop offset="60%" stopColor="#ff3355" />
          <stop offset="100%" stopColor="#a60f33" />
        </radialGradient>
      </defs>

      {/* edges: ring + spokes */}
      <g stroke="rgba(255,90,118,0.32)" strokeWidth="1">
        {nodes.map((n, i) => {
          const m = nodes[(i + 1) % 6];
          return <line key={`r${i}`} x1={n.x} y1={n.y} x2={m.x} y2={m.y} />;
        })}
        {nodes.map((n, i) => (
          <line key={`s${i}`} x1={cx} y1={cy} x2={n.x} y2={n.y} />
        ))}
      </g>

      {/* outer nodes */}
      {nodes.map((n, i) => (
        <circle key={i} cx={n.x} cy={n.y} r="5" fill="#0d0d14" stroke="#ff5e76" strokeWidth="1.5">
          <animate
            attributeName="opacity"
            values="0.55;1;0.55"
            dur={`${3 + (i % 3)}s`}
            begin={`${i * 0.3}s`}
            repeatCount="indefinite"
          />
        </circle>
      ))}

      {/* glowing core */}
      <circle cx={cx} cy={cy} r="22" fill="url(#core)" opacity="0.25" />
      <circle cx={cx} cy={cy} r="9" fill="url(#core)" />
    </svg>
  );
}
