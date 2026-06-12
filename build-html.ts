// Builds a self-contained single-file HTML app: wasm-bindgen glue inlined,
// wasm embedded as base64. Output works from file:// or any static host.
// Usage: bun build-html.ts [--test]   (--test adds a self-driving harness)

const glue = await Bun.file(new URL("./pkg/rustvm.js", import.meta.url)).text();
const wasm = await Bun.file(new URL("./pkg/rustvm_bg.wasm", import.meta.url)).arrayBuffer();
const b64 = Buffer.from(wasm).toString("base64");

// The glue's default export name (init), so host code can call it directly
// when concatenated into the same module scope.
const initName =
  glue.match(/export default (\w+);/)?.[1] ??
  glue.match(/export \{[^}]*?(\w+) as default[^}]*?\}/)?.[1];
if (!initName) throw new Error("could not find init function in pkg/rustvm.js");
if (glue.includes("</script")) throw new Error("glue contains </script>");

const test = process.argv.includes("--test");

const host = `
// ---- single-file host -----------------------------------------------------
globalThis.host_wake = () => {};

const b64 = "${b64}";
const bin = atob(b64);
const bytes = new Uint8Array(bin.length);
for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);

await ${initName}({ module_or_path: bytes.buffer });

const canvas = document.getElementById("screen");
const dpr = window.devicePixelRatio || 1;
const sizeCanvas = () => {
  canvas.width = Math.floor(canvas.clientWidth * dpr);
  canvas.height = Math.floor(canvas.clientHeight * dpr);
};
sizeCanvas();

let token;
try { token = localStorage.getItem("rustvm_token") || undefined; } catch {}
web_start("screen", 15 * dpr, token);

window.addEventListener("resize", () => { sizeCanvas(); web_resize(canvas.width, canvas.height); });
window.addEventListener("keydown", (e) => {
  if (e.metaKey) return;
  if (e.ctrlKey && e.key === "v") return;
  if (web_key(e.key, e.ctrlKey, e.altKey, e.shiftKey)) e.preventDefault();
});
window.addEventListener("paste", (e) => {
  const t = e.clipboardData?.getData("text");
  if (t) web_paste(t);
  e.preventDefault();
});
canvas.addEventListener("mousedown", (e) => web_mouse(e.offsetX * dpr, e.offsetY * dpr));
canvas.addEventListener("mousemove", (e) => {
  web_mouse_move(e.offsetX * dpr, e.offsetY * dpr);
  canvas.style.cursor = web_cursor_style();
});
canvas.addEventListener("wheel", (e) => {
  web_wheel(e.offsetX * dpr, e.offsetY * dpr, e.deltaY * dpr);
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
<title>RustVM — GitHub</title>
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

const out = test ? "dist/rustvm-test.html" : "dist/rustvm.html";
await Bun.write(new URL(out, import.meta.url), html);
console.log(`${out}: ${(html.length / 1024).toFixed(0)} KB`);
