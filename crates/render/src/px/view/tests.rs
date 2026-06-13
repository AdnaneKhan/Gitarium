use super::links::wrap_links;
use super::text::wrap_chars;

fn w(text: &str, cols: usize) -> Vec<String> {
    let mut v = Vec::new();
    wrap_chars(text, cols, &mut v);
    v
}

/// Wrap one line with link detection, returning (segments, per-segment link
/// spans, distinct urls) for assertions.
#[allow(clippy::type_complexity)]
fn wl(text: &str, cols: usize) -> (Vec<String>, Vec<Vec<(usize, usize, usize)>>, Vec<String>) {
    let mut urls = Vec::new();
    let segs = wrap_links(text, cols, &mut urls);
    let (text, links): (Vec<_>, Vec<_>) = segs.into_iter().unzip();
    (text, links, urls)
}

#[test]
fn wrap_soft_and_hard_breaks() {
    assert_eq!(w("hello world foo", 8), vec!["hello ", "world ", "foo"]);
    assert_eq!(w("abcdefghij", 8), vec!["abcdefgh", "ij"]);
    assert_eq!(w("x abcdefghijk", 8), vec!["x ", "abcdefgh", "ijk"]);
    assert_eq!(w("abcdefgh", 8), vec!["abcdefgh"]);
}

#[test]
fn wrap_preserves_whitespace() {
    // Leading indentation, whitespace-only lines, runs of spaces.
    assert_eq!(w("    foo", 20), vec!["    foo"]);
    assert_eq!(w("   ", 8), vec!["   "]);
    assert_eq!(w("a  b", 8), vec!["a  b"]);
    assert_eq!(w("a\n\nb", 8), vec!["a", "", "b"]);
}

#[test]
fn wrap_normalizes_tabs_and_crs() {
    assert_eq!(w("a\tb", 20), vec!["a    b"]);
    assert_eq!(w("line\r", 20), vec!["line"]);
    assert_eq!(w("\r", 20), vec![""]);
}

#[test]
fn wrap_zero_cols_no_spurious_line() {
    assert_eq!(w("ab", 0), vec!["a", "b"]);
    assert_eq!(w("a", 0), vec!["a"]);
}

#[test]
fn wrap_segments_concat_to_source() {
    // Copy reconstruction relies on segments of one source line
    // concatenating back to it verbatim (modulo tab/CR normalization).
    for s in [
        "The quick brown fox jumps over the lazy dog",
        "  indented continuation of a long wrapped paragraph",
        "supercalifragilisticexpialidocious tail",
        "a b c d e f g h i j k l m n o p",
        "word",
        " ",
    ] {
        for cols in 1..14 {
            assert_eq!(w(s, cols).concat(), s, "cols={}", cols);
        }
    }
}

#[test]
fn link_bare_url_spans_the_url() {
    let (text, links, urls) = wl("see https://x.com/a now", 80);
    assert_eq!(text, vec!["see https://x.com/a now"]);
    assert_eq!(urls, vec!["https://x.com/a"]);
    // "see " is 4 chars; the URL runs 4..19.
    assert_eq!(links, vec![vec![(4, 19, 0)]]);
}

#[test]
fn link_markdown_shows_text_hides_url() {
    let (text, links, urls) = wl("read [the docs](https://x.com/d) here", 80);
    assert_eq!(text, vec!["read the docs here"]);
    assert_eq!(urls, vec!["https://x.com/d"]);
    assert_eq!(links, vec![vec![(5, 13, 0)]]); // "the docs" at cols 5..13
}

#[test]
fn link_trailing_punctuation_trimmed() {
    let (_t, _l, urls) = wl("(https://x.com).", 80);
    assert_eq!(urls, vec!["https://x.com"]);
}

#[test]
fn link_spans_split_across_wraps() {
    // A bare URL wrapped mid-token keeps a span on each segment, with
    // columns local to that segment, all pointing at the one url.
    let (text, links, urls) = wl("https://example.com/long", 10);
    assert_eq!(urls.len(), 1);
    assert_eq!(text.concat(), "https://example.com/long");
    for (seg, spans) in text.iter().zip(&links) {
        assert_eq!(spans, &vec![(0, seg.chars().count(), 0)]);
    }
}

#[test]
fn link_plain_text_has_no_spans() {
    let (_t, links, urls) = wl("just some prose, no links here", 80);
    assert!(urls.is_empty());
    assert!(links.iter().all(|s| s.is_empty()));
}

#[test]
fn link_url_stops_at_html_attribute_quotes() {
    // URLs embedded in HTML attributes must not swallow the surrounding tag.
    let mut urls = Vec::new();
    let spans = super::links::url_spans(
        r#"<a href="https://bun.sh"><img src="https://github.com/u/a-b-c"></a>"#,
        &mut urls,
    );
    assert_eq!(urls, vec!["https://bun.sh", "https://github.com/u/a-b-c"]);
    assert_eq!(spans.len(), 2);
}

#[test]
fn link_bare_url_offsets_track_chars_not_bytes() {
    // A multibyte char before the URL must not shift its char-column span.
    let (_t, links, urls) = wl("café https://x.com/z", 80);
    assert_eq!(urls, vec!["https://x.com/z"]);
    assert_eq!(links, vec![vec![(5, 20, 0)]]); // "café " = 5 chars
}
