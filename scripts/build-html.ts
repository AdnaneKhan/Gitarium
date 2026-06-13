// Builds a self-contained single-file HTML app: wasm-bindgen glue inlined,
// wasm embedded as base64. Output works from file:// or any static host.
// Usage: bun scripts/build-html.ts [--test] [--obfuscate]
//   --test       adds a self-driving harness
//   --obfuscate  runs the obfuscator/ tool over the wasm first (data-section
//                encryption + section stripping + call aliasing + constant
//                encoding). Safe here: nothing re-optimizes the wasm after.

import { gzipSync } from "node:zlib";
import { fileURLToPath } from "node:url";
import { tmpdir } from "node:os";
import { join } from "node:path";

const rel = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// Pick the wasm to embed: the pristine build, or an obfuscated copy in a temp
// file (the glue is untouched — export/import names are preserved by design).
let wasmPath = rel("../pkg/gitarium_bg.wasm");
if (process.argv.includes("--obfuscate")) {
  const obfOut = join(tmpdir(), "gitarium_bg.obf.wasm");
  console.log("obfuscating wasm (encrypt + strip + alias-calls + obf-consts)…");
  const r = Bun.spawnSync(
    [
      "cargo", "run", "--release", "--quiet",
      "--manifest-path", rel("../obfuscator/Cargo.toml"), "--",
      "--alias-calls", "--obf-consts", wasmPath, obfOut,
    ],
    { stdout: "inherit", stderr: "inherit" },
  );
  if (!r.success) throw new Error("obfuscator failed");
  wasmPath = obfOut;
}

const glue = await Bun.file(rel("../pkg/gitarium.js")).text();
const wasm = await Bun.file(wasmPath).arrayBuffer();
// Embed the wasm gzip-compressed (~57% smaller than raw base64); the host
// gunzips it in-page via DecompressionStream. Brotli would be ~20% smaller
// still, but browsers expose no native JS brotli decoder — served paths use
// brotli over the wire (serve.ts) instead, where the browser decodes it.
const b64 = gzipSync(Buffer.from(wasm), { level: 9 }).toString("base64");

// The glue's default export name (init), so host code can call it directly
// when concatenated into the same module scope.
const initName =
  glue.match(/export default (\w+);/)?.[1] ??
  glue.match(/export \{[^}]*?(\w+) as default[^}]*?\}/)?.[1];
if (!initName) throw new Error("could not find init function in pkg/gitarium.js");
if (glue.includes("</script")) throw new Error("glue contains </script>");

const test = process.argv.includes("--test");

const host = `
// ---- single-file host -----------------------------------------------------
// Async results are queued wasm-side and drained by web_frame; rAF is
// paused in hidden tabs, so schedule an explicit frame on wake.
globalThis.host_wake = () => setTimeout(() => web_frame(performance.now()), 0);

// Embedded wasm is gzip-compressed base64; gunzip natively before instantiate.
const gz = Uint8Array.from(atob("${b64}"), (c) => c.charCodeAt(0));
const wasmBuf = await new Response(
  new Response(gz).body.pipeThrough(new DecompressionStream("gzip")),
).arrayBuffer();

await ${initName}({ module_or_path: wasmBuf });

const canvas = document.getElementById("screen");
// Read fresh on every use — zoom/monitor changes alter it at runtime.
const dpr = () => window.devicePixelRatio || 1;
const sizeCanvas = () => {
  canvas.width = Math.floor(canvas.clientWidth * dpr());
  canvas.height = Math.floor(canvas.clientHeight * dpr());
};
sizeCanvas();

let token;
try { token = localStorage.getItem("gitarium_token") || undefined; } catch {}
web_start("screen", 15 * dpr(), token);

const resized = () => { sizeCanvas(); web_resize(canvas.width, canvas.height); };
window.addEventListener("resize", resized);
// dpr can change without a resize (zoom, monitor move); a one-shot media
// query on the current ratio fires on any change, then re-arms.
const watchDpr = () => {
  matchMedia("(resolution: " + dpr() + "dppx)").addEventListener(
    "change",
    () => { web_set_font_px(15 * dpr()); resized(); watchDpr(); },
    { once: true },
  );
};
watchDpr();

// Hidden input that holds focus so IME composition works; composed text
// arrives via compositionend and is routed like a paste.
const ime = document.createElement("input");
ime.setAttribute("autocomplete", "off");
ime.setAttribute("autocapitalize", "off");
ime.setAttribute("spellcheck", "false");
ime.style.cssText = "position:fixed;top:0;left:-9999px;width:1px;height:1px;opacity:0";
document.body.appendChild(ime);
let composing = false;
ime.addEventListener("compositionstart", () => (composing = true));
ime.addEventListener("compositionend", (e) => {
  composing = false;
  if (e.data) web_paste(e.data);
  ime.value = "";
});
ime.addEventListener("input", () => {
  if (!composing) ime.value = "";
});
ime.focus({ preventScroll: true });

window.addEventListener("keydown", (e) => {
  if (e.isComposing || e.keyCode === 229) return; // IME owns these
  // AltGr reports ctrl+alt (or the AltGraph modifier); strip both so chars
  // like @ { € type on intl layouts. Cmd acts as Ctrl on macOS.
  const altgr =
    e.key.length === 1 &&
    (e.getModifierState?.("AltGraph") || (e.ctrlKey && e.altKey));
  const ctrl = !altgr && (e.ctrlKey || e.metaKey);
  const alt = !altgr && e.altKey;
  if (ctrl && e.key.toLowerCase() === "v") return; // native paste
  if (web_key(e.key, ctrl, alt, e.shiftKey)) e.preventDefault();
});
window.addEventListener("paste", (e) => {
  const t = e.clipboardData?.getData("text");
  if (t) web_paste(t);
  e.preventDefault();
});

// Last canvas-relative position, for releases outside the canvas where
// offsetX/Y would be relative to whatever element is under the cursor.
let lastX = 0;
let lastY = 0;
canvas.addEventListener("mousedown", (e) => {
  if (e.button !== 0) return; // primary button only
  e.preventDefault(); // keep focus on the IME input
  ime.focus({ preventScroll: true });
  lastX = e.offsetX;
  lastY = e.offsetY;
  web_mouse_down(e.offsetX * dpr(), e.offsetY * dpr());
});
canvas.addEventListener("mousemove", (e) => {
  lastX = e.offsetX;
  lastY = e.offsetY;
  web_mouse_move(e.offsetX * dpr(), e.offsetY * dpr());
  canvas.style.cursor = web_cursor_style();
});
canvas.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  web_context_menu(e.offsetX * dpr(), e.offsetY * dpr());
});
// On window so drags released outside the canvas still end.
window.addEventListener("mouseup", (e) => {
  if (e.button !== 0) return;
  const onCanvas = e.target === canvas;
  web_mouse_up((onCanvas ? e.offsetX : lastX) * dpr(), (onCanvas ? e.offsetY : lastY) * dpr());
});
// Released outside the browser window entirely: end any drag.
window.addEventListener("blur", () => web_mouse_up(lastX * dpr(), lastY * dpr()));

canvas.addEventListener("wheel", (e) => {
  // deltaMode: 0=pixels, 1=lines, 2=pages → normalize to CSS px.
  const unit = e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? canvas.clientHeight : 1;
  web_wheel(e.offsetX * dpr(), e.offsetY * dpr(), e.deltaY * unit * dpr());
  e.preventDefault();
}, { passive: false });

const loop = (t) => { web_frame(t); requestAnimationFrame(loop); };
requestAnimationFrame(loop);
`;

const drive = `
// ---- self-test drive (--test build only) ----------------------------------
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const enter = () => web_key("Enter", false, false, false);
const until = async (needle) => {
  for (let i = 0; i < 600; i++) {
    if (web_debug_text().includes(needle)) return true;
    await sleep(100);
  }
  return false;
};
await sleep(100);
enter();
await until("owner/repo or organization:");
web_paste("octocat/Hello-World");
enter();
const treeOk = await until("README");
enter();
const fileOk = await until("Hello World!");
document.title = treeOk && fileOk ? "SELFTEST-OK" : "SELFTEST-FAIL";
`;

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<title>Gitarium — GitHub</title>
<style>
html, body { margin: 0; height: 100%; background: #0d1117; overflow: hidden; }
canvas { display: block; width: 100vw; height: 100vh; }
</style>
</head>
<body>
<canvas id="screen"></canvas>
<script type="module">
${glue}
${host}
${test ? drive : ""}
</script>
</body>
</html>
`;

const out = test ? "dist/gitarium-test.html" : "dist/gitarium.html";
await Bun.write(new URL("../" + out, import.meta.url), html);
console.log(`${out}: ${(html.length / 1024).toFixed(0)} KB`);
