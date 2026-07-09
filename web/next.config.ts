import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Bake the deploy's commit SHA into the client bundle so it can tell when the
  // server has moved on to a newer deployment (see VersionWatcher). Vercel sets
  // VERCEL_GIT_COMMIT_SHA at build time; "dev" locally.
  env: {
    NEXT_PUBLIC_BUILD_ID:
      process.env.NEXT_PUBLIC_BUILD_ID || process.env.VERCEL_GIT_COMMIT_SHA || "dev",
  },
};

export default nextConfig;
