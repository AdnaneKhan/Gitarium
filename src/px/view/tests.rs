use super::text::wrap_chars;

fn w(text: &str, cols: usize) -> Vec<String> {
    let mut v = Vec::new();
    wrap_chars(text, cols, &mut v);
    v
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
