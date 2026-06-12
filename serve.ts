// Static file server for the browser demo: run with `bun serve.ts`
const server = Bun.serve({
  port: 8080,
  async fetch(req) {
    let path = new URL(req.url).pathname;
    if (path === "/") path = "/index.html";
    const file = Bun.file(import.meta.dir + path);
    return (await file.exists())
      ? new Response(file)
      : new Response("Not found", { status: 404 });
  },
});
console.log(`Serving browser demo on ${server.url}`);
