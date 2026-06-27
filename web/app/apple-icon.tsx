import { ImageResponse } from "next/og";

export const size = { width: 180, height: 180 };
export const contentType = "image/png";

// Apple touch icon — the Redstart bird, rendered to PNG at build time.
export default function AppleIcon() {
  return new ImageResponse(
    (
      <div
        style={{
          width: "100%",
          height: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          background: "#0d0d14",
        }}
      >
        <svg width="132" height="132" viewBox="0 0 32 32">
          <defs>
            <linearGradient id="bird" x1="0" y1="0" x2="1" y2="1">
              <stop offset="0" stopColor="#ff5e76" />
              <stop offset="1" stopColor="#ff7a45" />
            </linearGradient>
          </defs>
          <g transform="translate(0 2)">
            <path
              d="M16 17c-3.3 0-6.6-1.9-9-5-1-1.3-2.4-2.2-4-2.4 1.9-1.2 4-1 5.8.1 1.7 1 3.4 2 5.4 2.2-1.4-1.2-2.2-2.8-2.3-4.7 1.1 1.2 2.3 1.9 3.6 2.1.4-2.2 1.6-4 3.5-5.3-.2 1.3 0 2.5.6 3.6.6-1.1 1.5-2 2.7-2.6-.2 1.1-.1 2.1.3 3.1 1.1-.9 2.4-1.4 3.9-1.4-1 .8-1.6 1.8-1.8 3 1.6-.3 3.2 0 4.8 1-1.7.1-3 .9-4 2.3-2.3 3.1-5.6 5-9 5z"
              fill="url(#bird)"
            />
            <path
              d="M16 17c-1.1 1.9-2.7 3.2-4.8 3.9 1.3-1.6 2-3.2 2.2-4.9 1 .6 1.8.9 2.6 1z"
              fill="#ff7a45"
            />
          </g>
        </svg>
      </div>
    ),
    { ...size },
  );
}
