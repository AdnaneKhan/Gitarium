//! Checks which UI symbols each embedded font actually covers, using the
//! same engine (fontdue) the renderer uses. Run on the host target:
//! `cargo test --test glyphs -- --nocapture`

#[test]
fn symbol_coverage() {
    let ui = fontdue::Font::from_bytes(
        include_bytes!("../assets/Rajdhani-Regular.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .unwrap();
    let mono = fontdue::Font::from_bytes(
        include_bytes!("../assets/JetBrainsMono-Regular.ttf") as &[u8],
        fontdue::FontSettings::default(),
    )
    .unwrap();
    for c in ['★', '●', '▸', '▾', '✓', '✗', '⚠', '·', '…', '↑', '↓', '⑂', '⎇', '◆', '○'] {
        println!(
            "{}  rajdhani={}  jbmono={}",
            c,
            ui.lookup_glyph_index(c) != 0,
            mono.lookup_glyph_index(c) != 0
        );
    }
}
