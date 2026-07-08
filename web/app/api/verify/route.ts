import type { NextRequest } from "next/server";

// Proxies a compile-verification request to the sandbox verifier on the VPS.
// The browser posts the generated files here; we forward to COMPILER_URL with the
// shared token. Keeps the VPS URL/token server-side and avoids CORS.

export const maxDuration = 300;

export async function POST(request: NextRequest) {
  const base = process.env.COMPILER_URL;
  if (!base) {
    return Response.json(
      { ok: false, stage: "config", error: "Verifier not configured (COMPILER_URL unset)." },
      { status: 503 },
    );
  }

  let payload: unknown;
  try {
    payload = await request.json();
  } catch {
    return Response.json({ ok: false, stage: "input", error: "Invalid JSON." }, { status: 400 });
  }

  const files = (payload as { files?: unknown })?.files;
  if (!files || typeof files !== "object") {
    return Response.json({ ok: false, stage: "input", error: "Missing files." }, { status: 400 });
  }

  const headers: Record<string, string> = { "content-type": "application/json" };
  if (process.env.COMPILER_TOKEN) headers.authorization = `Bearer ${process.env.COMPILER_TOKEN}`;

  try {
    const res = await fetch(`${base.replace(/\/$/, "")}/verify`, {
      method: "POST",
      headers,
      body: JSON.stringify({ files }),
      signal: AbortSignal.timeout(290_000),
    });
    const data = await res.json();
    return Response.json(data, { status: res.ok ? 200 : 502 });
  } catch (e) {
    return Response.json(
      { ok: false, stage: "error", error: `Verifier unreachable: ${(e as Error).message}` },
      { status: 502 },
    );
  }
}
