use super::Editor;
use crate::ui::input::{Key, Mods};

const CTRL: Mods = Mods { ctrl: true, alt: false, shift: false };

fn editable(text: &str) -> Editor {
    let mut ed = Editor::from_text(text);
    ed.read_only = false;
    ed
}

// D1: a stale selection pointing past EOF must degrade to no-copy, not panic.
#[test]
fn selection_text_stale_selection_no_panic() {
    let mut ed = Editor::from_text("ab\ncd");
    ed.anchor = Some((0, 1));
    ed.cursor = (5, 0); // invariant violation: row past the last line
    assert_eq!(ed.selection_text(), None);
}

#[test]
fn selection_text_in_bounds_still_works() {
    let mut ed = Editor::from_text("ab\ncd");
    ed.anchor = Some((0, 1));
    ed.cursor = (1, 1);
    assert_eq!(ed.selection_text().as_deref(), Some("b\nc"));
}

// D2: read-only undo/redo must not report the key as consumed.
#[test]
fn read_only_undo_redo_not_consumed() {
    let mut ed = Editor::from_text("ab");
    assert!(ed.read_only);
    assert!(!ed.handle_key(&Key::Char('z'), CTRL, 10));
    assert!(!ed.handle_key(&Key::Char('y'), CTRL, 10));
    ed.read_only = false;
    assert!(ed.handle_key(&Key::Char('z'), CTRL, 10));
    assert!(ed.handle_key(&Key::Char('y'), CTRL, 10));
}

// D3: pasting over a selection is one undo step; one Ctrl+Z fully restores.
#[test]
fn replace_selection_is_single_undo_step() {
    let mut ed = editable("hello world");
    ed.anchor = Some((0, 0));
    ed.cursor = (0, 5);
    ed.insert_text("bye");
    assert_eq!(ed.to_text(), "bye world");
    ed.undo();
    assert_eq!(ed.to_text(), "hello world");
    ed.undo(); // stack exhausted — nothing else to unwind
    assert_eq!(ed.to_text(), "hello world");
    ed.redo();
    assert_eq!(ed.to_text(), "bye world");
    assert_eq!(ed.cursor, (0, 3));
}

// D3: multi-line replace round-trips through repeated undo/redo.
#[test]
fn replace_selection_multiline_roundtrip() {
    let mut ed = editable("ab\ncd\nef\n");
    ed.anchor = Some((0, 1));
    ed.cursor = (2, 1);
    ed.insert_text("X\nY");
    assert_eq!(ed.to_text(), "aX\nYf\n");
    for _ in 0..2 {
        ed.undo();
        assert_eq!(ed.to_text(), "ab\ncd\nef\n");
        ed.redo();
        assert_eq!(ed.to_text(), "aX\nYf\n");
    }
}

// D3: typing a single char over a selection is also one undo step.
#[test]
fn typed_char_over_selection_single_undo() {
    let mut ed = editable("hello");
    ed.anchor = Some((0, 1));
    ed.cursor = (0, 4); // selects "ell"
    assert!(ed.handle_key(&Key::Char('X'), Mods::NONE, 10));
    assert_eq!(ed.to_text(), "hXo");
    ed.undo();
    assert_eq!(ed.to_text(), "hello");
}

// D3: typing after a replace stays a separate step (no coalesce into group).
#[test]
fn typing_after_replace_is_separate_step() {
    let mut ed = editable("hello");
    ed.anchor = Some((0, 0));
    ed.cursor = (0, 5);
    assert!(ed.handle_key(&Key::Char('A'), Mods::NONE, 10));
    assert!(ed.handle_key(&Key::Char('B'), Mods::NONE, 10));
    assert_eq!(ed.to_text(), "AB");
    ed.undo();
    assert_eq!(ed.to_text(), "A");
    ed.undo();
    assert_eq!(ed.to_text(), "hello");
}

// Regression guard: plain typed runs still coalesce into one undo step.
#[test]
fn typed_run_still_coalesces() {
    let mut ed = editable("");
    for c in ['a', 'b', 'c'] {
        assert!(ed.handle_key(&Key::Char(c), Mods::NONE, 10));
    }
    assert_eq!(ed.to_text(), "abc");
    ed.undo();
    assert_eq!(ed.to_text(), "");
}
