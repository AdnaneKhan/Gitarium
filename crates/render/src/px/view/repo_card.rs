//! One repository card: name, badges, description, indicators.

use super::*;

impl View {
    /// Draw one card of the repo list (rect and selection precomputed).
    pub(super) fn repo_card(&mut self, dl: &mut DrawList, atlas: &mut Atlas, repo: &crate::github::Repo, card: RectF, selected: bool, fi: usize) {
        let hv = self.hover_amt(wid(Z_REPO, fi), card.contains(self.hot.0, self.hot.1));

        if selected {
            dl.glow(card, self.f(4.0), with_a(CYAN, 0.10), self.f(14.0));
        }
        dl.rrect(card, self.f(4.0), BG1, 1.0);
        if hv > 0.01 {
            dl.rrect(card, self.f(4.0), with_a(CYAN, 0.045 * hv), 1.0);
        }
        let border_c = if selected {
            with_a(CYAN, 0.85)
        } else if hv > 0.01 {
            with_a(CYAN, 0.2 + 0.3 * hv)
        } else {
            BORDER_BRIGHT
        };
        dl.border(card, self.f(4.0), 1.0, border_c);
        if selected {
            self.brackets(dl, card, self.f(9.0), with_a(CYAN, 0.8));
        }

        let pad = self.f(16.0);
        let inner_w = card.w - pad * 2.0;

        // Line 1: name + badges (left), pushed age (right).
        let l1 = card.y + self.f(24.0);
        let name = dl.fit(atlas, UI_BOLD, self.f(15.5), &repo.full_name, inner_w * 0.5);
        let mut x = dl.text(
            atlas,
            UI_BOLD,
            self.f(15.5),
            card.x + pad,
            l1,
            &name,
            if selected { CYAN } else { TEXT },
            0.0,
        );
        let badges: [(bool, &str, Color); 3] = [
            (repo.private, "PRIVATE", MAGENTA),
            (repo.fork, "FORK", with_a(TEXT, 0.55)),
            (repo.archived, "ARCHIVED", YELLOW),
        ];
        for (on, label, color) in badges {
            if !on {
                continue;
            }
            let bpx = self.f(9.5);
            let tw = dl.text_width(atlas, UI, bpx, label, self.f(1.2));
            let br = RectF::new(x + self.f(10.0), card.y + self.f(11.0), tw + self.f(10.0), self.f(16.0));
            dl.border(br, self.f(2.0), 1.0, with_a(color, 0.8));
            dl.text(atlas, UI, bpx, br.x + self.f(5.0), card.y + self.f(23.0), label, color, self.f(1.2));
            x = br.right();
        }
        if let Some(p) = &repo.pushed_at {
            let age = format!("pushed {}", crate::app::fmt_age(p));
            let tw = dl.text_width(atlas, UI, self.f(11.5), &age, 0.0);
            dl.text(atlas, UI, self.f(11.5), card.right() - pad - tw, l1, &age, FAINT, 0.0);
        }

        // Line 2: description, one line across the full width.
        if let Some(d) = &repo.description {
            let msg = dl.fit(atlas, UI, self.f(12.5), d, inner_w);
            dl.text(atlas, UI, self.f(12.5), card.x + pad, card.y + self.f(46.0), &msg, DIM, 0.0);
        }

        // Line 3: indicators (left), branch chip (right).
        let iy = card.y + self.f(70.0);
        let mut ix = card.x + pad;
        if let Some(lang) = &repo.language {
            let dot = RectF::new(ix, iy - self.f(7.0), self.f(8.0), self.f(8.0));
            dl.rrect(dot, self.f(4.0), lang_color(lang), 1.0);
            ix = dl.text(atlas, UI, self.f(12.0), ix + self.f(13.0), iy, lang, with_a(TEXT, 0.75), 0.0)
                + self.f(16.0);
        }
        if repo.stargazers_count > 0 {
            ix = dl.text(atlas, MONO, self.f(11.5), ix, iy, &format!("*{}", repo.stargazers_count), DIM, 0.0)
                + self.f(16.0);
        }
        if repo.forks_count > 0 {
            ix = dl.text(atlas, UI, self.f(12.0), ix, iy, &format!("{} forks", repo.forks_count), DIM, 0.0)
                + self.f(16.0);
        }
        if repo.open_issues_count > 0 {
            ix = dl.text(atlas, UI, self.f(12.0), ix, iy, &format!("{} issues", repo.open_issues_count), DIM, 0.0)
                + self.f(16.0);
        }
        if let Some(l) = repo.license.as_ref().and_then(|l| l.spdx_id.as_deref()) {
            if l != "NOASSERTION" && ix < card.right() - pad - self.f(120.0) {
                dl.text(atlas, UI, self.f(12.0), ix, iy, l, FAINT, 0.0);
            }
        }
        let chip = dl.fit(atlas, MONO, self.f(11.0), &format!("[{}]", repo.default_branch), inner_w * 0.3);
        let cw = dl.text_width(atlas, MONO, self.f(11.0), &chip, 0.0);
        dl.text(atlas, MONO, self.f(11.0), card.right() - pad - cw, iy, &chip, with_a(CYAN, 0.6), 0.0);

        self.clicks.push((card, Click::Repo(fi)));
    }
}

/// GitHub-style language dot colors, nudged lighter where the official
/// color would vanish on a near-black background.
fn lang_color(lang: &str) -> Color {
    match lang {
        "Rust" => rgba(0xde, 0xa5, 0x84, 1.0),
        "JavaScript" => rgba(0xf1, 0xe0, 0x5a, 1.0),
        "TypeScript" => rgba(0x31, 0x78, 0xc6, 1.0),
        "Python" => rgba(0x4b, 0x8b, 0xbe, 1.0),
        "Go" => rgba(0x00, 0xad, 0xd8, 1.0),
        "C" => rgba(0x9a, 0x9a, 0x9a, 1.0),
        "C++" => rgba(0xf3, 0x4b, 0x7d, 1.0),
        "C#" => rgba(0x2f, 0xa8, 0x35, 1.0),
        "Java" => rgba(0xb0, 0x72, 0x19, 1.0),
        "Kotlin" => rgba(0xa9, 0x7b, 0xff, 1.0),
        "Swift" => rgba(0xf0, 0x51, 0x38, 1.0),
        "Ruby" => rgba(0xcc, 0x34, 0x2d, 1.0),
        "PHP" => rgba(0x77, 0x7b, 0xb4, 1.0),
        "Shell" | "Bash" => rgba(0x89, 0xe0, 0x51, 1.0),
        "HTML" => rgba(0xe3, 0x4c, 0x26, 1.0),
        "CSS" => rgba(0x7e, 0x5c, 0xb8, 1.0),
        "Zig" => rgba(0xec, 0x91, 0x5c, 1.0),
        "Lua" => rgba(0x55, 0x66, 0xcc, 1.0),
        "Dockerfile" => rgba(0x5b, 0x7c, 0x88, 1.0),
        "Vue" => rgba(0x41, 0xb8, 0x83, 1.0),
        "Dart" => rgba(0x00, 0xb4, 0xab, 1.0),
        _ => DIM,
    }
}
