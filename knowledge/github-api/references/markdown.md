# markdown

Legend: * required | :i int :n num :b bool :[] list | {k*} object (required keys) | (a|b) enum | =v default | [pg] paginated | ->NNN success code when not 200 | (GHEC) cloud-only

POST /markdown — Render a Markdown document
  b: text* context mode(markdown|gfm)

POST /markdown/raw — Render a Markdown document in raw mode
  b: raw (text/plain)
