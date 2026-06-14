// GitHub API proxy for `serve.ts --api-proxy`. The browser forwards each
// GitHub request over a WebSocket; this performs the actual fetch server-side
// and forwards the response back. Shared by serve.ts and test-browser.ts.
//
// Token model — in production (serve.ts) the browser-forwarded Authorization
// header is passed through as-is: the server uses only the PAT the user pasted
// at the auth screen and never reads GITHUB_TOKEN, so no ambient creds leak
// into the session. The optional `token` below is exercised only by the test
// harness (test-browser.ts) to cover the server-side override path.
// AI/Anthropic calls never come here (they stay direct).
const GITHUB = "https://api.github.com";
const PROXY_PATH = "/__gh";

type WS = { send(data: string): void };
type Req = { id?: unknown; method?: string; path?: string; headers?: Record<string, string>; body?: string | null };

export type Proxy = {
  path: string;
  inject(html: string): string;
  websocket: { message(ws: WS, raw: string | Buffer): Promise<void> };
};

/** Build the proxy handler. `token`, when set, overrides the forwarded
 * Authorization header — used by the test harness; serve.ts passes none, so
 * the pasted token flows through unchanged. */
export function makeProxy(token?: string): Proxy {
  return {
    path: PROXY_PATH,
    inject: (html) =>
      html.replace(
        "</head>",
        `<script>window.__GITARIUM_PROXY__=${JSON.stringify(PROXY_PATH)}</script></head>`,
      ),
    websocket: {
      message: (ws, raw) => handle(ws, typeof raw === "string" ? raw : raw.toString(), token),
    },
  };
}

async function handle(ws: WS, raw: string, token?: string): Promise<void> {
  let req: Req;
  try {
    req = JSON.parse(raw);
  } catch {
    return; // unparseable frame: ignore (no id to reply to)
  }
  const id = req.id;
  if (typeof id !== "number") return;
  if (typeof req.path !== "string" || !req.path.startsWith("/")) {
    ws.send(JSON.stringify({ id, status: 0, error: "proxy: bad path" }));
    return;
  }

  const headers: Record<string, string> = { ...(req.headers ?? {}) };
  if (token) {
    // Server token wins: drop any forwarded auth, then set ours.
    for (const k of Object.keys(headers)) {
      if (k.toLowerCase() === "authorization") delete headers[k];
    }
    headers["Authorization"] = `Bearer ${token}`;
  }

  const method = req.method ?? "GET";
  const url = GITHUB + req.path;
  audit(method, url, req.body);

  try {
    const resp = await fetch(url, {
      method,
      headers,
      body: req.body ?? undefined,
    });
    ws.send(
      JSON.stringify({
        id,
        status: resp.status,
        body: await resp.text(),
        remaining: numHeader(resp, "x-ratelimit-remaining"),
        limit: numHeader(resp, "x-ratelimit-limit"),
      }),
    );
  } catch (e) {
    ws.send(JSON.stringify({ id, status: 0, error: String(e) }));
  }
}

/** Audit a forwarded call as one sanitized log line: the method, the full
 * target URL, and the first 100 chars of the request body (if any). The
 * Authorization header is never part of the entry; `clean` scrubs any token
 * that slips into the body or URL and flattens newlines, so each entry is
 * exactly one line and no secret reaches the log. */
function audit(method: string, url: string, body?: string | null): void {
  const snippet = body && body.length > 0 ? " " + clean(body.slice(0, 100)) : "";
  console.log(`[gitarium-proxy] ${method} ${clean(url)}${snippet}`);
}

/** Redact secret-looking substrings (GitHub PATs, bearer tokens) and collapse
 * runs of whitespace into single spaces so a value can't break the log line. */
export function clean(s: string): string {
  return s
    .replace(/gh[pousr]_[A-Za-z0-9]{16,}/g, "[REDACTED]")
    .replace(/github_pat_[A-Za-z0-9_]{16,}/g, "[REDACTED]")
    .replace(/(Bearer\s+)[A-Za-z0-9._-]+/gi, "$1[REDACTED]")
    .replace(/[\r\n\t]+/g, " ");
}

function numHeader(resp: Response, name: string): number | undefined {
  const v = resp.headers.get(name);
  return v == null ? undefined : Number(v);
}
