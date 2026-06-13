//! GitHub-style `:shortcode:` → emoji. The full set is ~1800 entries; this is
//! a curated subset of the most common ones (kept small to avoid bundle bloat).

/// If a `:name:` shortcode starts at `chars[i]` (a `:`), return its emoji and
/// the index just past the closing `:`.
pub(super) fn expand_at(chars: &[char], i: usize) -> Option<(&'static str, usize)> {
    let mut j = i + 1;
    while chars.get(j).is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-')) {
        j += 1;
    }
    if j == i + 1 || chars.get(j) != Some(&':') {
        return None;
    }
    let name: String = chars[i + 1..j].iter().collect();
    lookup(&name).map(|e| (e, j + 1))
}

fn lookup(name: &str) -> Option<&'static str> {
    Some(match name {
        "smile" => "😄", "smiley" => "😃", "grin" => "😁", "laughing" | "satisfied" => "😆",
        "wink" => "😉", "blush" => "😊", "joy" => "😂", "sweat_smile" => "😅", "sob" => "😭",
        "cry" => "😢", "thinking" => "🤔", "tada" => "🎉", "rocket" | "shipit" => "🚀",
        "fire" => "🔥", "sparkles" => "✨", "star" => "⭐", "zap" => "⚡", "boom" => "💥",
        "bulb" => "💡", "dizzy" => "💫", "100" => "💯", "heart" => "❤", "yellow_heart" => "💛",
        "thumbsup" | "+1" => "👍", "thumbsdown" | "-1" => "👎", "ok_hand" => "👌", "clap" => "👏",
        "wave" => "👋", "pray" => "🙏", "muscle" => "💪", "eyes" => "👀", "raised_hands" => "🙌",
        "point_right" => "👉", "point_left" => "👈", "bug" => "🐛", "memo" | "pencil" => "📝",
        "books" => "📚", "book" => "📖", "white_check_mark" => "✅", "heavy_check_mark" => "✔",
        "x" => "❌", "warning" => "⚠", "rotating_light" => "🚨", "construction" => "🚧",
        "lock" => "🔒", "key" => "🔑", "pushpin" => "📌", "hammer" => "🔨", "wrench" => "🔧",
        "package" => "📦", "art" => "🎨", "recycle" => "♻", "question" => "❓",
        "exclamation" => "❗", "checkered_flag" => "🏁", "poop" | "hankey" => "💩",
        "ghost" => "👻", "robot" => "🤖",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn expands_known_only() {
        let c: Vec<char> = ":rocket: and :nope: and 12:30".chars().collect();
        assert_eq!(super::expand_at(&c, 0), Some(("🚀", 8)));
        assert_eq!(super::expand_at(&c, 13), None); // unknown name
        assert_eq!(super::expand_at(&c, 26), None); // "12:30" — not a shortcode
    }
}
