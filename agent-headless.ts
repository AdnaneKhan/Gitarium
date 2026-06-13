// Headless autonomous agent: loads the headless-only RustVM wasm
// (crates/headless — agent + foundation, no rendering) and drives the SAME
// agent loop as the in-app `i` window — identical tools (github_api,
// code_search, bash/grep/find over the in-wasm shell), the compiled knowledge
// bundle, the shell VFS, and context compaction — but UI-free and
// self-driving. Given a goal it works without a human in the loop until it
// reports the goal achieved/blocked (a GOAL_ACHIEVED / GOAL_BLOCKED sentinel
// line) or a turn cap is reached.
//
// Build the wasm once:  wasm-pack build crates/headless --target web
//
// Usage:
//   GITHUB_TOKEN=ghp_…  ANTHROPIC_API_KEY=sk-ant-… \
//     bun agent-headless.ts "<goal prompt>"
//
// Environment:
//   ANTHROPIC_API_KEY   required — the Claude key that drives the agent.
//   GITHUB_TOKEN        the PAT the agent operates GitHub with. Optional →
//                       anonymous (read-only public access; writes fail).
//   ANTHROPIC_BASE_URL  optional Messages API base override (proxy/gateway).
//   AGENT_MAX_TURNS     optional safety cap on model turns (default 60).
//
// Exit code: 0 when the goal is achieved, 1 otherwise (blocked / turn cap).

import init, { agent_run_headless } from "./crates/headless/pkg/rustvm_headless.js";

const goal = process.argv.slice(2).join(" ").trim();
if (!goal) {
  console.error('usage: bun agent-headless.ts "<goal prompt>"');
  process.exit(2);
}

const apiKey = process.env.ANTHROPIC_API_KEY?.trim();
if (!apiKey) {
  console.error("ANTHROPIC_API_KEY is required (the Claude key that drives the agent)");
  process.exit(2);
}

const token = process.env.GITHUB_TOKEN?.trim() || undefined;
if (!token) {
  console.error("(no GITHUB_TOKEN — anonymous: read-only public access, writes will fail)");
}
const baseUrl = process.env.ANTHROPIC_BASE_URL?.trim() || undefined;
const maxTurns = Math.max(0, Number(process.env.AGENT_MAX_TURNS ?? 60) | 0);

const wasm = await Bun.file(
  new URL("./crates/headless/pkg/rustvm_headless_bg.wasm", import.meta.url),
).arrayBuffer();
await init({ module_or_path: wasm });

// Render the agent's JSON event stream: assistant prose → stdout, progress →
// stderr, so `> out.txt` captures just the agent's replies.
const emit = (line: string) => {
  let e: any;
  try {
    e = JSON.parse(line);
  } catch {
    console.log(line);
    return;
  }
  switch (e.type) {
    case "start":
      console.error(`▸ agent ${e.login ? `as ${e.login}` : "(anonymous)"} · cap ${maxTurns} turns`);
      break;
    case "turn":
      process.stderr.write(`\n── turn ${e.n} ──\n`);
      break;
    case "text":
      process.stdout.write(`${e.text}\n`);
      break;
    case "tool":
      process.stderr.write(`  · ${e.label}\n`);
      break;
    case "tool_done":
      if (!e.ok) process.stderr.write(`    ! tool failed\n`);
      break;
    case "compact":
      process.stderr.write("  · compacting context…\n");
      break;
    case "limit":
      process.stderr.write(`  · hit the ${e.turns}-turn cap\n`);
      break;
    case "error":
      process.stderr.write(`  ! ${e.message}\n`);
      break;
  }
};

const resultJson = (await agent_run_headless(
  goal,
  token,
  apiKey,
  baseUrl,
  maxTurns,
  emit,
)) as string;

const { outcome, detail } = JSON.parse(resultJson);
console.error(`\n■ ${String(outcome).toUpperCase()}: ${detail}`);
process.exit(outcome === "achieved" ? 0 : 1);
