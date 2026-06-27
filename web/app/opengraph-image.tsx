import { ImageResponse } from "next/og";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

export const alt = "Redstart — one language for The Graph subgraphs";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

const BG = "#08080c";
const TEXT = "#ededf2";
const MUTED = "#9a9aab";
const RED = "#ff3355";
const EMBER = "#ff7a45";

function Bird({ s, color }: { s: number; color: string }) {
  return (
    <svg width={s} height={s} viewBox="0 0 32 32" fill={color}>
      <path d="M16 17c-3.3 0-6.6-1.9-9-5-1-1.3-2.4-2.2-4-2.4 1.9-1.2 4-1 5.8.1 1.7 1 3.4 2 5.4 2.2-1.4-1.2-2.2-2.8-2.3-4.7 1.1 1.2 2.3 1.9 3.6 2.1.4-2.2 1.6-4 3.5-5.3-.2 1.3 0 2.5.6 3.6.6-1.1 1.5-2 2.7-2.6-.2 1.1-.1 2.1.3 3.1 1.1-.9 2.4-1.4 3.9-1.4-1 .8-1.6 1.8-1.8 3 1.6-.3 3.2 0 4.8 1-1.7.1-3 .9-4 2.3-2.3 3.1-5.6 5-9 5z" />
    </svg>
  );
}

export default async function Image() {
  const [display, displayBold, mono] = await Promise.all([
    readFile(join(process.cwd(), "assets/SpaceGrotesk-Medium.woff")),
    readFile(join(process.cwd(), "assets/SpaceGrotesk-Bold.woff")),
    readFile(join(process.cwd(), "assets/GeistMono-Medium.ttf")),
  ]);

  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          flexDirection: "column",
          justifyContent: "space-between",
          background: BG,
          padding: "72px 80px",
          fontFamily: "Space Grotesk",
          position: "relative",
        }}
      >
        {/* nebula glows */}
        <div style={{ position: "absolute", top: -260, left: -160, width: 700, height: 700, borderRadius: 9999, background: "radial-gradient(closest-side, rgba(255,51,85,0.22), transparent)" }} />
        <div style={{ position: "absolute", bottom: -300, right: -120, width: 640, height: 640, borderRadius: 9999, background: "radial-gradient(closest-side, rgba(255,122,69,0.16), transparent)" }} />

        {/* wordmark */}
        <div style={{ display: "flex", alignItems: "center", gap: 16 }}>
          <Bird s={36} color={RED} />
          <span style={{ fontFamily: "Geist Mono", fontSize: 28, letterSpacing: "0.01em", color: TEXT }}>Redstart</span>
        </div>

        {/* tagline */}
        <div style={{ display: "flex", flexDirection: "column", fontFamily: "Space Grotesk" }}>
          <span style={{ fontSize: 88, fontWeight: 700, lineHeight: 1.0, color: TEXT, letterSpacing: "-0.03em" }}>
            Write the subgraph once.
          </span>
          <span style={{ fontSize: 88, fontWeight: 700, lineHeight: 1.0, color: EMBER, letterSpacing: "-0.03em" }}>
            Properly.
          </span>
        </div>

        {/* footer */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            borderTop: "1px solid rgba(255,255,255,0.1)",
            paddingTop: 28,
            fontFamily: "Geist Mono",
            fontSize: 23,
          }}
        >
          <span style={{ color: MUTED }}>The most performant &amp; secure language for The Graph subgraphs</span>
          <span style={{ color: TEXT }}>redstart-lang.com</span>
        </div>
      </div>
    ),
    {
      ...size,
      fonts: [
        { name: "Space Grotesk", data: display, style: "normal", weight: 500 },
        { name: "Space Grotesk", data: displayBold, style: "normal", weight: 700 },
        { name: "Geist Mono", data: mono, style: "normal", weight: 500 },
      ],
    },
  );
}
