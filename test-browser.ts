// Headless browser regression suite: serves the project, drives
// browser-test.html?mode=suite in Chrome, and reports the PASS/FAIL lines
// the page logs to the console. Run with `bun test-browser.ts`.

const PORT = 8123;
const CHROME =
  process.env.CHROME ??
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

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

const proc = Bun.spawn(
  [
    CHROME,
    "--headless=new",
    "--no-first-run",
    "--enable-unsafe-swiftshader",
    "--window-size=1280,800",
    "--virtual-time-budget=180000",
    "--screenshot=/tmp/rustvm-suite.png",
    "--enable-logging=stderr",
    "--v=0",
    `http://localhost:${PORT}/browser-test.html?mode=suite`,
  ],
  { stdout: "ignore", stderr: "pipe" },
);

const killer = setTimeout(() => proc.kill(), 240_000);
const err = await new Response(proc.stderr).text();
await proc.exited;
clearTimeout(killer);
server.stop();

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
for (const l of lines) console.log(l);

const summary = lines.find((l) => l.startsWith("SUITE:"));
const failed = lines.some((l) => l.startsWith("FAIL:"));
if (!summary || failed) {
  console.error(summary ? "suite had failures" : "suite did not complete");
  process.exitCode = 1;
}
