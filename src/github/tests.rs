use super::search::fragment_line;

#[test]
fn no_range_picks_first_nonempty_line() {
    let (line, range) = fragment_line("\n   \nfn main() {}\n", None);
    assert_eq!(line, "fn main() {}");
    assert_eq!(range, None);
    assert_eq!(fragment_line("", None), (String::new(), None));
}

#[test]
fn match_mid_line() {
    let frag = "alpha\n  beta gamma\ndelta";
    let ms = frag.find("gamma").unwrap();
    let (line, range) = fragment_line(frag, Some((ms, ms + "gamma".len())));
    assert_eq!(line, "beta gamma");
    assert_eq!(range, Some((5, 10)));
}

#[test]
fn match_at_fragment_start_and_end() {
    let frag = "hello\nworld";
    assert_eq!(fragment_line(frag, Some((0, 5))), ("hello".to_string(), Some((0, 5))));
    // Zero-width match at the very end stays on the last line.
    assert_eq!(fragment_line(frag, Some((11, 11))), ("world".to_string(), Some((5, 5))));
}

#[test]
fn match_on_newline_picks_following_line() {
    let frag = "first\nsecond";
    // Match text begins at the '\n' (covers "\nsec").
    let (line, range) = fragment_line(frag, Some((5, 9)));
    assert_eq!(line, "second");
    assert_eq!(range, Some((0, 3)));
    // Match that is exactly the newline maps to the start of the
    // following line.
    let (line, range) = fragment_line(frag, Some((5, 6)));
    assert_eq!(line, "second");
    assert_eq!(range, Some((0, 0)));
}

#[test]
fn match_on_crlf_and_blank_lines_skips_to_content() {
    // "\r\nb" — CRLF terminator stepped over as one unit.
    let (line, range) = fragment_line("a\r\nb", Some((1, 4)));
    assert_eq!(line, "b");
    assert_eq!(range, Some((0, 1)));
    // "\n\nbc" — blank line between match start and content.
    let (line, range) = fragment_line("a\n\nbc", Some((1, 5)));
    assert_eq!(line, "bc");
    assert_eq!(range, Some((0, 2)));
}

#[test]
fn trailing_newline_match_yields_empty_following_line() {
    let (line, range) = fragment_line("abc\n", Some((3, 4)));
    assert_eq!(line, "");
    assert_eq!(range, Some((0, 0)));
}

#[test]
fn multibyte_around_match() {
    let frag = "héllo wörld\nnaïve";
    let ms = frag.find("wörld").unwrap();
    let (line, range) = fragment_line(frag, Some((ms, ms + "wörld".len())));
    assert_eq!(line, "héllo wörld");
    // Char indices, not byte indices.
    assert_eq!(range, Some((6, 11)));
}

#[test]
fn offsets_inside_utf8_sequence_snap_to_boundaries() {
    // Byte 2 sits inside 'é' (bytes 1..3): start snaps down, end up.
    let (line, range) = fragment_line("héllo", Some((2, 2)));
    assert_eq!(line, "héllo");
    assert_eq!(range, Some((1, 2)));
}

#[test]
fn out_of_range_and_reversed_offsets_are_clamped() {
    let frag = "abc\ndef";
    let (line, range) = fragment_line(frag, Some((50, 99)));
    assert_eq!(line, "def");
    assert_eq!(range, Some((3, 3)));
    let (line, range) = fragment_line(frag, Some((2, 1)));
    assert_eq!(line, "abc");
    assert_eq!(range, Some((2, 2)));
    assert_eq!(fragment_line("", Some((3, 7))), (String::new(), Some((0, 0))));
}

#[test]
fn indented_line_range_is_relative_to_trimmed_line() {
    let frag = "fn x() {\n    let y = 1;\n}";
    let ms = frag.find("let").unwrap();
    let (line, range) = fragment_line(frag, Some((ms, ms + 3)));
    assert_eq!(line, "let y = 1;");
    assert_eq!(range, Some((0, 3)));
}
