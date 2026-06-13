// Static file server for the browser demo: run with `bun scripts/serve.ts`.
//
// Negotiates Content-Encoding for compressible assets — brotli (preferred,
// the smallest) or gzip — so the ~2.7 MB wasm ships as ~0.86 MB (br) or
// ~1.1 MB (gzip); the browser decodes it transparently. Compressed payloads
// are cached per file (keyed on mtime) so brotli q11 runs once per asset,
// not per request.
import { brotliCompressSync, gzipSync, constants } from "node:zlib";
import { makeProxy } from "./proxy-server";

const TYPE: Record<string, string> = {
  html: "text/html; charset=utf-8",
  js: "text/javascript; charset=utf-8",
  wasm: "application/wasm",
  css: "text/css; charset=utf-8",
  json: "application/json; charset=utf-8",
  map: "application/json; charset=utf-8",
  svg: "image/svg+xml",
  txt: "text/plain; charset=utf-8",
};
// Types worth compressing (skip already-compressed binaries like png/woff2).
const COMPRESSIBLE = new Set(Object.keys(TYPE));

type Entry = { mtime: number; raw: Uint8Array; br?: Uint8Array; gz?: Uint8Array };
const cache = new Map<string, Entry>();

async function load(fsPath: string): Promise<Entry | null> {
  const file = Bun.file(fsPath);
  if (!(await file.exists())) return null;
  const mtime = file.lastModified;
  const hit = cache.get(fsPath);
  if (hit && hit.mtime === mtime) return hit;
  const entry: Entry = { mtime, raw: new Uint8Array(await file.arrayBuffer()) };
  cache.set(fsPath, entry);
  return entry;
}

// `--api-proxy` routes the browser's GitHub calls through this server over a
// WebSocket; GITHUB_TOKEN (if set) overrides the forwarded token.
const proxy = process.argv.includes("--api-proxy")
  ? makeProxy(process.env.GITHUB_TOKEN?.trim() || undefined)
  : null;

const server = Bun.serve({
  port: 8080,
  ...(proxy ? { websocket: proxy.websocket } : {}),
  async fetch(req, server) {
    let path = new URL(req.url).pathname;
    if (proxy && path === proxy.path) {
      return server.upgrade(req)
        ? undefined
        : new Response("expected a websocket upgrade", { status: 426 });
    }
    if (path === "/") path = "/index.html";
    // App assets sit at the repo root (this script lives in scripts/); the
    // test harness page lives in tests/.
    const base = import.meta.dir + (path === "/browser-test.html" ? "/../tests" : "/..");
    const entry = await load(base + path);
    if (!entry) return new Response("Not found", { status: 404 });

    const ext = path.slice(path.lastIndexOf(".") + 1).toLowerCase();
    // Inject the proxy global into served HTML; skip compression for it (HTML
    // is tiny — avoids polluting the mtime-keyed compressed cache).
    if (proxy && ext === "html") {
      return new Response(proxy.inject(new TextDecoder().decode(entry.raw)), {
        headers: { "content-type": TYPE.html },
      });
    }
    const headers: Record<string, string> = {
      "content-type": TYPE[ext] ?? "application/octet-stream",
    };

    // Negotiate compression. Tiny files aren't worth the framing overhead.
    const accept = req.headers.get("accept-encoding") ?? "";
    if (COMPRESSIBLE.has(ext) && entry.raw.length > 1024) {
      headers["vary"] = "accept-encoding";
      if (/\bbr\b/.test(accept)) {
        entry.br ??= brotliCompressSync(entry.raw, {
          params: { [constants.BROTLI_PARAM_QUALITY]: 11 },
        });
        headers["content-encoding"] = "br";
        return new Response(entry.br, { headers });
      }
      if (/\bgzip\b/.test(accept)) {
        entry.gz ??= gzipSync(entry.raw, { level: 9 });
        headers["content-encoding"] = "gzip";
        return new Response(entry.gz, { headers });
      }
    }
    return new Response(entry.raw, { headers });
  },
});
console.log(`Serving browser demo on ${server.url}`);
