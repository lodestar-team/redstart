# Subgraph verifier sandbox.
# Ubuntu 22.04 base — Matchstick pins its prebuilt binary to Ubuntu versions
# (binary-linux-22) and needs libpq5 at runtime. Bakes a base project with
# graph-cli + graph-ts + matchstick-as, and warms the Matchstick binary into
# node_modules so per-job compiles never re-download it.
#
# Each verify: copy base -> write files -> `graph codegen && build && test`.
# No handler code runs untrusted: build runs asc; test runs Matchstick's mock store.

FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
  && apt-get install -y --no-install-recommends curl ca-certificates libpq5 \
  && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
  && apt-get install -y --no-install-recommends nodejs \
  && rm -rf /var/lib/apt/lists/*

# --- base project (graph-cli + graph-ts + matchstick-as) ---
WORKDIR /opt/base
COPY base/package.json ./
RUN npm install --no-audit --no-fund --loglevel=error

# --- warm the Matchstick binary into node_modules, then drop the dummy source ---
COPY warmup/ ./
RUN (npx --no-install graph codegen && npx --no-install graph test) || echo "warmup complete" \
  && rm -rf schema.graphql subgraph.yaml src tests abis generated build

# --- the service ---
WORKDIR /app
COPY package.json server.mjs ./

RUN useradd -m verifier && chown -R verifier:verifier /opt/base /app
USER verifier

ENV BASE_PROJECT=/opt/base \
    PORT=8787 \
    NODE_ENV=production
EXPOSE 8787
CMD ["node", "server.mjs"]
