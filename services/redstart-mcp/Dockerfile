FROM node:20-bookworm-slim

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY package.json ./
RUN npm install --no-audit --no-fund --omit=dev --loglevel=error
COPY server.mjs ./

RUN useradd -m mcp && chown -R mcp:mcp /app
USER mcp

ENV PORT=8788 NODE_ENV=production
EXPOSE 8788
CMD ["node", "server.mjs"]
