// Headless browser regression suite: serves the project, drives
// browser-test.html in Chrome, and reports the PASS/FAIL lines the page
// logs to the console. Run with `bun test-browser.ts`.
//
// Three passes: the full live-API suite on the default (WebGL2) context,
// then API-free boot smokes with WebGL2 hidden (?gl=1 → WebGL1 fallback)
// and with all WebGL hidden (?gl=0 → Canvas2D software backend).
//
// A GITHUB_TOKEN in .env.test authenticates the suite pass so it isn't
// throttled by the anonymous 60 req/hour rate limit.

import { makeProxy } from "./proxy-server";

const PORT = 8123;
const CHROME =
  process.env.CHROME ??
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

// CI injects the token via $GITHUB_TOKEN (from a repo secret); locally it
// comes from the gitignored .env.test. Env wins so CI never reads the file.
async function resolveToken(): Promise<string | undefined> {
  if (process.env.GITHUB_TOKEN) return process.env.GITHUB_TOKEN;
  const f = Bun.file(import.meta.dir + "/.env.test");
  if (!(await f.exists())) return undefined;
  for (const line of (await f.text()).split("\n")) {
    const m = line.match(/^\s*(?:export\s+)?GITHUB_TOKEN\s*=\s*"?([^"\s#]+)/);
    if (m) return m[1];
  }
  return undefined;
}

const server = Bun.serve({
  port: PORT,
  async fetch(req) {
    let path = new URL(req.url).pathname;
    if (path === "/") path = "/index.html";
    const file = Bun.file(import.meta.dir + path);
    return (await file.exists())
      ? new Response(file)
      : new Response("Not found", { status: 404 });
  },
});

async function run(label: string, query: string, shot: string): Promise<boolean> {
  const proc = Bun.spawn(
    [
      CHROME,
      "--headless=new",
      "--no-first-run",
      "--enable-unsafe-swiftshader",
      "--window-size=1280,800",
      "--virtual-time-budget=180000",
      `--screenshot=${shot}`,
      "--enable-logging=stderr",
      "--v=0",
      `http://localhost:${PORT}/browser-test.html?${query}`,
    ],
    { stdout: "ignore", stderr: "pipe" },
  );

  const killer = setTimeout(() => proc.kill(), 240_000);
  const err = await new Response(proc.stderr).text();
  await proc.exited;
  clearTimeout(killer);

  const lines = err
    .split("\n")
    .filter((l) => /"(PASS|FAIL|SUITE):/.test(l))
    .map((l) =>
      l
        // Chrome logs either `CONSOLE(43)]` or `CONSOLE:43]` depending on version.
        .replace(/^.*CONSOLE[:(]\d+\)?\]\s*"/, "")
        .replace(/",?\s*source:.*$/, "")
        .replace(/"$/, ""),
    );
  console.log(`--- ${label} ---`);
  for (const l of lines) console.log(l);

  const summary = lines.find((l) => l.startsWith("SUITE:"));
  const failed = lines.some((l) => l.startsWith("FAIL:"));
  if (!summary || failed) {
    console.error(summary ? "run had failures" : "run did not complete");
    return false;
  }
  return true;
}

// Proxy transport check (no Chrome): a Bun WebSocket client drives the same
// proxy `serve.ts --api-proxy` exposes, against the live API. This covers the
// wire protocol + token handling that the headless suite cannot — Chrome's
// virtual time doesn't pause for WebSocket frames, so the budget expires before
// proxied replies land. The WASM-side client is covered by manual browser runs.
async function proxyTransport(token?: string): Promise<boolean> {
  const proxy = makeProxy(token); // token present → exercises server-side override
  const srv = Bun.serve({
    port: PORT + 1,
    websocket: proxy.websocket,
    fetch(req, server) {
      return new URL(req.url).pathname === proxy.path
        ? server.upgrade(req)
          ? undefined
          : new Response("expected a websocket upgrade", { status: 426 })
        : new Response("ok");
    },
  });
  const reply: any = await new Promise((resolve) => {
    const ws = new WebSocket(`ws://localhost:${PORT + 1}${proxy.path}`);
    const timer = setTimeout(() => resolve({ status: 0, error: "timeout" }), 15000);
    ws.onopen = () =>
      ws.send(JSON.stringify({
        id: 1, method: "GET", path: "/repos/octocat/Hello-World",
        headers: { Accept: "application/vnd.github+json" }, body: null,
      }));
    ws.onmessage = (e) => {
      clearTimeout(timer);
      ws.close();
      resolve(JSON.parse(String(e.data)));
    };
    ws.onerror = () => resolve({ status: 0, error: "socket error" });
  });
  srv.stop();
  const ok =
    reply.status === 200 &&
    (() => {
      try {
        return JSON.parse(reply.body).full_name === "octocat/Hello-World";
      } catch {
        return false;
      }
    })();
  console.log("--- api proxy transport ---");
  console.log(ok ? "PASS: github over websocket" : `FAIL: proxy (status ${reply.status}${reply.error ? ", " + reply.error : ""})`);
  return ok;
}

// `--smoke` runs only the API-free boot checks — the CI default when no
// token is configured, since the full suite drives the live GitHub API.
const smokeOnly = process.argv.includes("--smoke");
const token = await resolveToken();

const boots: Array<() => Promise<boolean>> = [
  () => run("boot smoke (forced webgl1)", "mode=boot&gl=1", "/tmp/rustvm-gl1.png"),
  () => run("boot smoke (forced canvas2d)", "mode=boot&gl=0", "/tmp/rustvm-canvas2d.png"),
];

let ok = true;
if (smokeOnly) {
  console.log("(--smoke: API-free boot checks only, skipping live-API suite)");
} else {
  if (!token) console.log("(no GITHUB_TOKEN — suite runs anonymous, rate-limited)");
  const suiteQuery = `mode=suite${token ? `&token=${encodeURIComponent(token)}` : ""}`;
  ok = await run("full suite (webgl2)", suiteQuery, "/tmp/rustvm-suite.png");
}
for (const b of boots) ok = (await b()) && ok;
ok = (await proxyTransport(token)) && ok;

server.stop();
if (!ok) process.exitCode = 1;
