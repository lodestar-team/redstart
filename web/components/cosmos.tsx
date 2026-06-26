// Fixed deep-space backdrop: red nebula glows + a deterministic starfield.
// Positions come from a seeded LCG so server and client render identically.

function stars(count: number, seed: number) {
  let s = seed;
  const rand = () => {
    s = (s * 1664525 + 1013904223) % 4294967296;
    return s / 4294967296;
  };
  return Array.from({ length: count }, (_, i) => ({
    key: i,
    x: rand() * 100,
    y: rand() * 100,
    r: rand() * 1.3 + 0.3,
    o: rand() * 0.6 + 0.15,
    tw: rand() > 0.7,
    d: (rand() * 4).toFixed(2),
  }));
}

export function Cosmos() {
  const field = stars(110, 20260626);
  return (
    <div className="pointer-events-none fixed inset-0 -z-10 overflow-hidden bg-bg">
      {/* nebula glows */}
      <div
        className="absolute -left-[10%] -top-[20%] h-[70vh] w-[70vh] rounded-full"
        style={{ background: "radial-gradient(closest-side, rgba(255,51,85,0.16), transparent)" }}
      />
      <div
        className="absolute right-[-15%] top-[10%] h-[60vh] w-[60vh] rounded-full"
        style={{ background: "radial-gradient(closest-side, rgba(255,122,69,0.10), transparent)" }}
      />
      <div
        className="absolute bottom-[-25%] left-[30%] h-[60vh] w-[80vh] rounded-full"
        style={{ background: "radial-gradient(closest-side, rgba(166,15,51,0.14), transparent)" }}
      />

      {/* starfield */}
      <svg className="absolute inset-0 h-full w-full" preserveAspectRatio="xMidYMid slice" viewBox="0 0 100 100">
        {field.map((st) => (
          <circle
            key={st.key}
            cx={st.x}
            cy={st.y}
            r={st.r * 0.12}
            fill="#ffffff"
            opacity={st.o}
            style={st.tw ? { animation: `twinkle ${2 + Number(st.d)}s ease-in-out ${st.d}s infinite` } : undefined}
          />
        ))}
      </svg>

      {/* subtle vignette to seat the content */}
      <div
        className="absolute inset-0"
        style={{ background: "radial-gradient(120% 80% at 50% 0%, transparent 50%, rgba(0,0,0,0.5))" }}
      />
    </div>
  );
}
