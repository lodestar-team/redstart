// The currently-deployed build id, read at runtime. A client that was served an
// older deployment will see a different id here (the alias always routes to the
// newest deployment) and can prompt the user to reload. Never cached.

export const dynamic = "force-dynamic";

export function GET(): Response {
  const id = process.env.VERCEL_GIT_COMMIT_SHA || "dev";
  return new Response(JSON.stringify({ id }), {
    headers: {
      "content-type": "application/json",
      "cache-control": "no-store, max-age=0",
    },
  });
}
