# Subgraph verifier sandbox.
# Bakes a base project with graph-cli + graph-ts pre-installed so each verify
# request is just: copy base -> write files -> `graph codegen && graph build`.
# No per-request npm install, no network needed at request time.

FROM node:20-bookworm-slim

# graph-cli/asc are self-contained; ca-certificates only for the one-time install.
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

# --- base project (graph-cli + graph-ts in node_modules) ---
WORKDIR /opt/base
COPY base/package.json ./
RUN npm install --no-audit --no-fund --loglevel=error

# --- the service ---
WORKDIR /app
COPY package.json server.mjs ./

# Run unprivileged; jobs run in /tmp which stays writable.
RUN useradd -m verifier && chown -R verifier:verifier /opt/base /app
USER verifier

ENV BASE_PROJECT=/opt/base \
    PORT=8787 \
    NODE_ENV=production
EXPOSE 8787
CMD ["node", "server.mjs"]
