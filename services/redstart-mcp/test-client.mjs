// Quick MCP client smoke test against a running server.
//   URL=http://localhost:8788/mcp node test-client.mjs
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";

const URL = process.env.URL ?? "http://localhost:8788/mcp";
const token = process.env.MCP_TOKEN;

const transport = new StreamableHTTPClientTransport(new global.URL(URL), {
  requestInit: token ? { headers: { authorization: `Bearer ${token}` } } : undefined,
});
const client = new Client({ name: "smoke", version: "1.0.0" });
await client.connect(transport);
console.log("connected ✓");

const tools = await client.listTools();
console.log("tools:", tools.tools.map((t) => t.name).join(", "));

const resources = await client.listResources();
console.log("resources:", resources.resources.map((r) => r.uri).join(", "));

console.log("\n— fetch_contract WETH —");
const r = await client.callTool({
  name: "fetch_contract",
  arguments: { address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", chainId: 1 },
});
const info = JSON.parse(r.content[0].text);
console.log(`name=${info.name} kind=${info.kind} startBlock=${info.startBlock} events=${info.events?.map((e) => e.name).join(",")}`);

await client.close();
console.log("\nall good ✓");
