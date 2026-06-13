//! Parser tests (atlas-free): inline emphasis/links and block structure.

use super::block::parse_blocks;
use super::inline::parse_inline;
use super::*;

fn inl(s: &str) -> (Vec<Inline>, Vec<String>) {
    let mut urls = Vec::new();
    let runs = parse_inline(s, &mut urls);
    (runs, urls)
}

#[test]
fn emphasis_and_code() {
    let (r, _) = inl("plain **bold** and *it* and `code`");
    let bold = r.iter().find(|x| x.text == "bold").unwrap();
    assert!(bold.style.bold && !bold.style.italic);
    let it = r.iter().find(|x| x.text == "it").unwrap();
    assert!(it.style.italic && !it.style.bold);
    let code = r.iter().find(|x| x.text == "code").unwrap();
    assert!(code.style.code);
}

#[test]
fn bold_italic_combo() {
    let (r, _) = inl("***both***");
    let b = r.iter().find(|x| x.text == "both").unwrap();
    assert!(b.style.bold && b.style.italic);
}

#[test]
fn snake_case_not_italic() {
    let (r, _) = inl("call foo_bar_baz now");
    assert!(r.iter().all(|x| !x.style.italic));
    assert!(r.iter().any(|x| x.text.contains("foo_bar_baz")));
}

#[test]
fn links_and_autolinks() {
    let (r, urls) = inl("see [docs](https://x.com/a) and https://y.org");
    assert_eq!(urls.len(), 2);
    let d = r.iter().find(|x| x.text == "docs").unwrap();
    assert_eq!(d.style.link, Some(0));
    assert!(r.iter().any(|x| x.text == "https://y.org" && x.style.link == Some(1)));
}

#[test]
fn image_is_placeholder_not_link() {
    let (r, urls) = inl("before ![a cat](https://x.com/cat.png) after");
    assert!(urls.is_empty(), "images must not add a clickable url");
    assert!(r.iter().all(|x| x.style.link.is_none()));
    let joined: String = r.iter().map(|x| x.text.clone()).collect();
    assert!(joined.contains("a cat") && !joined.contains("cat.png"));
}

#[test]
fn strikethrough() {
    let (r, _) = inl("a ~~gone~~ b");
    assert!(r.iter().find(|x| x.text == "gone").unwrap().style.strike);
}

#[test]
fn escape() {
    let (r, _) = inl(r"not \*italic\*");
    assert!(r.iter().all(|x| !x.style.italic));
    let joined: String = r.iter().map(|x| x.text.clone()).collect();
    assert!(joined.contains("*italic*"));
}

#[test]
fn headings_and_rule() {
    let mut urls = Vec::new();
    let b = parse_blocks("# Title\n\n---\n\n## Sub", &mut urls);
    assert!(matches!(b[0], Block::Heading(1, _)));
    assert!(b.iter().any(|x| matches!(x, Block::Rule)));
    assert!(b.iter().any(|x| matches!(x, Block::Heading(2, _))));
}

#[test]
fn lists_with_nesting_and_tasks() {
    let mut urls = Vec::new();
    let b = parse_blocks("- a\n  - b\n- [x] done\n- [ ] todo", &mut urls);
    let items: Vec<&Block> = b.iter().filter(|x| matches!(x, Block::Item { .. })).collect();
    assert_eq!(items.len(), 4);
    if let Block::Item { depth, .. } = items[1] {
        assert_eq!(*depth, 1);
    }
    assert!(matches!(items[2], Block::Item { task: Some(true), .. }));
    assert!(matches!(items[3], Block::Item { task: Some(false), .. }));
}

#[test]
fn fenced_code_block() {
    let mut urls = Vec::new();
    let b = parse_blocks("```rust\nfn main() {}\n```", &mut urls);
    let code = b.iter().find_map(|x| match x {
        Block::Code(lang, lines) => Some((lang.is_some(), lines.clone())),
        _ => None,
    });
    let (has_lang, lines) = code.unwrap();
    assert!(has_lang);
    assert_eq!(lines, vec!["fn main() {}".to_string()]);
}

#[test]
fn gfm_table() {
    let mut urls = Vec::new();
    let src = "| A | B |\n|:--|--:|\n| 1 | 2 |\n| 3 | 4 |";
    let b = parse_blocks(src, &mut urls);
    let t = b.iter().find_map(|x| match x {
        Block::Table { aligns, header, body } => Some((aligns.clone(), header.len(), body.len())),
        _ => None,
    });
    let (aligns, ncols, nrows) = t.unwrap();
    assert_eq!(ncols, 2);
    assert_eq!(nrows, 2);
    assert_eq!(aligns[0], Align::Left);
    assert_eq!(aligns[1], Align::Right);
}

#[test]
fn blockquote() {
    let mut urls = Vec::new();
    let b = parse_blocks("> quoted line\n> more", &mut urls);
    assert!(b.iter().any(|x| matches!(x, Block::Quote(1, _))));
}

#[test]
fn paragraph_joins_soft_breaks() {
    let mut urls = Vec::new();
    let b = parse_blocks("one\ntwo\n\nthree", &mut urls);
    let paras: Vec<&Block> = b.iter().filter(|x| matches!(x, Block::Para(_))).collect();
    assert_eq!(paras.len(), 2);
}

#[test]
fn layout_produces_rows() {
    let atlas = Atlas::new().unwrap();
    let sizes = MdSizes {
        text_px: 14.0,
        mono_px: 13.0,
        indent: 18.0,
        h_px: [18.0, 16.0, 15.0, 14.0, 14.0, 14.0],
    };
    let mut urls = Vec::new();
    let src = "# Title\n\nA **bold** word and a [link](https://x.com).\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
    let rows = layout_blocks(&parse_blocks(src, &mut urls), 400.0, &sizes, &atlas);
    assert!(rows.iter().any(|r| matches!(r, MdRow::Line { deco: Deco::Heading(1), .. })));
    assert!(rows.iter().any(|r| matches!(r, MdRow::Code { .. })));
    assert!(rows.iter().any(|r| matches!(r, MdRow::Table { header: true, .. })));
    assert!(rows
        .iter()
        .any(|r| matches!(r, MdRow::Line { spans, .. } if spans.iter().any(|s| s.link.is_some()))));
    assert_eq!(urls.len(), 1);
}

#[test]
fn selection_text_and_geometry() {
    use super::select::{row_text, row_xs};
    let atlas = Atlas::new().unwrap();
    let sizes = MdSizes { text_px: 14.0, mono_px: 13.0, indent: 18.0, h_px: [16.0; 6] };
    let mut urls = Vec::new();
    let rows = layout_blocks(&parse_blocks("hello **world**", &mut urls), 500.0, &sizes, &atlas);
    let linerow = rows.iter().find(|r| matches!(r, MdRow::Line { .. })).unwrap();
    // Copy text concatenates the spans, stripped of markup.
    assert_eq!(row_text(linerow), "hello world");
    // One x boundary per char plus the trailing edge, left-to-right, indented.
    let xs = row_xs(&atlas, linerow, 12.0).unwrap();
    assert_eq!(xs.len(), "hello world".chars().count() + 1);
    assert!(xs.windows(2).all(|w| w[1] >= w[0]));
    assert_eq!(xs[0], 0.0); // a paragraph row starts flush at the left edge
}

#[test]
fn long_paragraph_wraps_to_multiple_rows() {
    let atlas = Atlas::new().unwrap();
    let sizes = MdSizes { text_px: 14.0, mono_px: 13.0, indent: 18.0, h_px: [18.0; 6] };
    let mut urls = Vec::new();
    let text = "word ".repeat(80);
    let rows = layout_blocks(&parse_blocks(&text, &mut urls), 200.0, &sizes, &atlas);
    let lines = rows.iter().filter(|r| matches!(r, MdRow::Line { .. })).count();
    assert!(lines > 1, "expected wrapping into multiple rows, got {}", lines);
}
