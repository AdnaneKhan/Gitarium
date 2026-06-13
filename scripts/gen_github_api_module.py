#!/usr/bin/env python3
"""Generate knowledge/github-api/ from GitHub's official OpenAPI description.

Usage:
  curl -sL -o /tmp/api.json https://raw.githubusercontent.com/github/\
rest-api-description/main/descriptions/api.github.com/api.github.com.json
  python3 scripts/gen_github_api_module.py /tmp/api.json

Rewrites SKILL.md and references/*.md; references/conventions.md is
hand-maintained and left untouched. Re-run when GitHub bumps the API version.
"""
import datetime, json, os, sys
from collections import defaultdict

API_VERSION = "2026-03-10"
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
OUT = os.path.join(ROOT, "knowledge", "github-api")
REF_CAP = 200   # spec: reference files <= 200 lines
SKILL_CAP = 100  # spec: SKILL.md <= 100 lines
METHODS = ("get", "put", "post", "patch", "delete", "head")
PAGINATION = {"per_page", "page"}
LEGEND = ("Legend: * required | :i int :n num :b bool :[] list | {k*} object "
          "(required keys) | (a|b) enum | =v default | [pg] paginated | "
          "->NNN success code when not 200 | (GHEC) cloud-only")


def deref(spec, node):
    while isinstance(node, dict) and "$ref" in node:
        cur = spec
        for part in node["$ref"].lstrip("#/").split("/"):
            cur = cur[part]
        node = cur
    return node if isinstance(node, dict) else {}


def short_type(spec, sch):
    """Compact type tag; empty string means plain string (the default)."""
    sch = deref(spec, sch)
    t = sch.get("type")
    if isinstance(t, list):
        t = next((x for x in t if x != "null"), None)
    if t == "array":
        return "[%s]" % short_type(spec, sch.get("items", {}))
    if t == "object" or "properties" in sch:
        return "o"
    return {"integer": "i", "number": "n", "boolean": "b"}.get(t, "")


def enum_of(spec, sch):
    e = sch.get("enum") or deref(spec, sch.get("items", {})).get("enum")
    vals = [v if isinstance(v, str) else json.dumps(v) for v in (e or []) if v is not None]
    if len(vals) > 6:
        vals = vals[:5] + ["…"]
    return "(%s)" % "|".join(vals) if vals else ""


def annotate(spec, name, sch, required, with_default=True):
    sch = deref(spec, sch)
    s = name + ("*" if required else "")
    t, e = short_type(spec, sch), enum_of(spec, sch)
    if t == "o":
        kids = sch.get("properties") or {}
        req = [k + "*" for k in sch.get("required") or [] if k in kids]
        s += "{%s}" % ",".join(req)
    elif t and not e:
        s += ":" + t
    s += e
    d = sch.get("default")
    if with_default and d is not None and not isinstance(d, (dict, list)):
        s += "=" + (d if isinstance(d, str) else json.dumps(d))
    return s


def render_op(spec, method, path, op, shared_params):
    gh = op.get("x-github") or {}
    params = [deref(spec, p) for p in shared_params + (op.get("parameters") or [])]
    flags = []
    succ = next((c for c in sorted(op.get("responses") or {}) if c.startswith("2")), "")
    if succ and succ != "200":
        flags.append("->" + succ)
    if any(p.get("name") in PAGINATION for p in params):
        flags.append("[pg]")
    if op.get("deprecated"):
        flags.append("(deprecated)")
    if gh.get("githubCloudOnly"):
        flags.append("(GHEC)")
    head = "%s %s — %s" % (method.upper(), path, op.get("summary") or op.get("operationId", ""))
    lines = [(head + " " + " ".join(flags)).rstrip()]
    q = [annotate(spec, p["name"], p.get("schema") or {}, p.get("required"))
         for p in params if p.get("in") == "query" and p["name"] not in PAGINATION]
    if q:
        lines.append("  q: " + " ".join(q))
    content = deref(spec, op.get("requestBody") or {}).get("content") or {}
    if "application/json" in content:
        sch = deref(spec, content["application/json"].get("schema") or {})
        props, req, note = sch.get("properties"), set(sch.get("required") or []), ""
        if not props and (sch.get("oneOf") or sch.get("anyOf")):
            props = {}
            for v in sch.get("oneOf") or sch.get("anyOf"):
                props.update(deref(spec, v).get("properties") or {})
            note = " (one-of)"
        if props:
            items = sorted(props.items(), key=lambda kv: (kv[0] not in req, kv[0]))
            lines.append("  b: " + " ".join(
                annotate(spec, k, v, k in req, with_default=False) for k, v in items) + note)
        elif sch:
            lines.append("  b: json %s" % (short_type(spec, sch) or "string"))
    elif content:
        lines.append("  b: raw (%s)" % next(iter(content)))
    return lines


def file_stem(cat, sub):
    if not sub or sub in (cat, "-"):
        return cat
    return sub if sub.startswith(cat + "-") else "%s-%s" % (cat, sub)


def main():
    src = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ghapi/api.json"
    with open(src) as f:
        spec = json.load(f)
    groups, n_ops = defaultdict(list), 0
    for path in sorted(spec["paths"]):
        item = spec["paths"][path]
        for method in METHODS:
            op = item.get(method)
            if not isinstance(op, dict):
                continue
            n_ops += 1
            gh = op.get("x-github") or {}
            cat = gh.get("category") or "misc"
            stem = file_stem(cat, gh.get("subcategory"))
            groups[(cat, stem)].append(
                render_op(spec, method, path, op, item.get("parameters") or []))

    refs = os.path.join(OUT, "references")
    os.makedirs(refs, exist_ok=True)
    for f in os.listdir(refs):
        if f.endswith(".md") and f != "conventions.md":
            os.remove(os.path.join(refs, f))
    toc, sizes = defaultdict(list), {}
    for (cat, stem), ops in sorted(groups.items()):
        chunks, cur = [], []
        for op in ops:
            if cur and len(cur) + len(op) + 1 > REF_CAP - 4:
                chunks.append(cur)
                cur = []
            cur.extend([""] + op if cur else op)
        chunks.append(cur)
        for i, chunk in enumerate(chunks):
            name = stem if i == 0 else "%s-%d" % (stem, i + 1)
            text = "# %s\n\n%s\n\n%s\n" % (name, LEGEND, "\n".join(chunk))
            with open(os.path.join(refs, name + ".md"), "w") as f:
                f.write(text)
            toc[cat].append(name)
            sizes[name + ".md"] = text.count("\n")

    desc = ("Condensed GitHub REST API reference (X-GitHub-Api-Version %s): every "
            "endpoint with query/body parameters, enums and defaults, grouped by "
            "topic. Consult before composing non-trivial API calls; see "
            "references/conventions.md for auth, pagination, rate limits, errors."
            % API_VERSION)
    skill = ["---", "name: github-api", "description: " + desc,
             "source: https://docs.github.com/en/rest?apiVersion=" + API_VERSION,
             "openapi: github/rest-api-description v" + spec["info"]["version"],
             "fetched: " + datetime.date.today().isoformat(), "---", "",
             "# GitHub REST API — condensed reference", "",
             "Each references/ file lists endpoints as:", "",
             "    METHOD /path — summary [flags]",
             "      q: query params | b: body fields", "", LEGEND, "",
             "Start with references/conventions.md (auth, versioning, pagination,",
             "rate limits, media types, errors). Find an endpoint:",
             "grep -i <keyword> /knowledge/github-api/references/", "", "## Topics", ""]
    skill += ["- %s: %s" % (cat, ", ".join(toc[cat])) for cat in sorted(toc)]
    text = "\n".join(skill) + "\n"
    with open(os.path.join(OUT, "SKILL.md"), "w") as f:
        f.write(text)
    sizes["SKILL.md"] = text.count("\n")

    toml = os.path.join(ROOT, "knowledge.toml")
    if not os.path.exists(toml):
        with open(toml, "w") as f:
            f.write('modules = ["github-api"]\n')

    total = sum(os.path.getsize(os.path.join(dp, f))
                for dp, _, fs in os.walk(OUT) for f in fs)
    print("%d ops -> %d files, %d KB total" % (n_ops, len(sizes), total // 1024))
    print("largest:", sorted(sizes.items(), key=lambda kv: -kv[1])[:5])
    bad = [f for f, n in sizes.items()
           if n > (SKILL_CAP if f == "SKILL.md" else REF_CAP)]
    if bad or total > 1 << 20:
        sys.exit("CAP VIOLATION: %s total=%dKB" % (bad, total // 1024))
    if total > 256 << 10:
        print("warning: module over 256 KB (soft cap)")


if __name__ == "__main__":
    main()
