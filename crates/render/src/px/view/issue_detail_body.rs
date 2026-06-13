//! Builds the issue/PR detail's scrollable content as markdown rows: the PR
//! merge requirements (mergeability, checks, reviews) are hand-built colored
//! rows; the description and each comment body are rendered through the
//! markdown engine (`md`). Returns the rows plus their shared link table.

use super::issues_pane::{label_color, readable};
use super::md::{layout_blocks, parse_blocks, Deco, MdRow, MdSizes, Span};
use super::*;

fn span(text: &str, font: u8, px: f32, color: Color) -> Span {
    Span { text: text.to_string(), font, px, color, code: false, strike: false, link: None }
}

fn line(spans: Vec<Span>, indent: f32) -> MdRow {
    MdRow::Line { spans, indent, deco: Deco::None }
}

fn plain(text: &str, color: Color, s: &MdSizes) -> MdRow {
    line(vec![span(text, UI, s.text_px, color)], 0.0)
}

fn heading(text: &str, s: &MdSizes) -> MdRow {
    line(vec![span(text, UI_BOLD, s.h_px[2], with_a(CYAN, 0.85))], 0.0)
}

pub(super) fn detail_rows(d: &Detail, width: f32, s: &MdSizes, atlas: &Atlas) -> (Vec<MdRow>, Vec<String>) {
    let mut urls: Vec<String> = Vec::new();
    let mut rows: Vec<MdRow> = Vec::new();
    if !d.labels.is_empty() {
        let mut spans: Vec<Span> = Vec::new();
        for l in &d.labels {
            let col = label_color(&l.color);
            spans.push(span("● ", UI, s.text_px, col));
            spans.push(span(&format!("{}   ", l.name), UI_BOLD, s.text_px, readable(col)));
        }
        rows.push(line(spans, 0.0));
        rows.push(MdRow::Blank);
    }
    if d.is_pr {
        pr_meta(d, s, &mut rows);
    }
    rows.push(heading("DESCRIPTION", s));
    if d.body.trim().is_empty() {
        rows.push(plain("(no description provided)", DIM, s));
    } else {
        rows.extend(layout_blocks(&parse_blocks(&d.body, &mut urls), width, s, atlas));
    }
    rows.push(MdRow::Blank);
    match &d.comments {
        Loadable::Loading => rows.push(plain("loading comments…", DIM, s)),
        Loadable::Failed(e) => rows.push(plain(&format!("comments: {}", e), RED, s)),
        Loadable::Ready(cs) => {
            rows.push(heading(&format!("COMMENTS ({})", cs.len()), s));
            if cs.is_empty() {
                rows.push(plain("none", DIM, s));
            }
            for cm in cs {
                let who = cm.user.as_ref().map(|u| u.login.clone()).unwrap_or_default();
                let head = format!("@{} · {}", who, crate::app::fmt_age(&cm.created_at));
                rows.push(line(vec![span(&head, UI_BOLD, s.text_px, with_a(MAGENTA, 0.9))], 0.0));
                rows.extend(layout_blocks(&parse_blocks(&cm.body, &mut urls), width, s, atlas));
                rows.push(MdRow::Blank);
            }
        }
        Loadable::Idle => {}
    }
    (rows, urls)
}

/// The PR-only merge-requirement rows: diff stats, mergeability, checks, and
/// reviews — colored by status.
fn pr_meta(d: &Detail, s: &MdSizes, rows: &mut Vec<MdRow>) {
    if let Loadable::Ready(p) = &d.pr {
        let stats = format!("+{}  -{}  ·  {} files changed", p.additions, p.deletions, p.changed_files);
        rows.push(plain(&stats, DIM, s));
        let ms = if p.mergeable_state.is_empty() { "unknown" } else { p.mergeable_state.as_str() };
        rows.push(line(
            vec![
                span("MERGEABILITY: ", UI_BOLD, s.text_px, with_a(CYAN, 0.85)),
                span(&ms.to_uppercase(), UI_BOLD, s.text_px, merge_color(&p.mergeable_state)),
            ],
            0.0,
        ));
        rows.push(MdRow::Blank);
    }
    match &d.checks {
        Loadable::Loading => rows.push(plain("checks loading…", DIM, s)),
        Loadable::Failed(e) => rows.push(plain(&format!("checks: {}", e), RED, s)),
        Loadable::Ready(cs) if !cs.is_empty() => {
            rows.push(heading(&format!("CHECKS ({})", cs.len()), s));
            for c in cs {
                let (icon, rgb) = run_icon(&c.status, c.conclusion.as_deref());
                let txt = format!("{} {}", icon, c.name);
                rows.push(line(vec![span(&txt, UI, s.text_px, crate::px::theme::c(rgb, 1.0))], s.indent));
            }
            rows.push(MdRow::Blank);
        }
        _ => {}
    }
    if let Loadable::Ready(rs) = &d.reviews {
        if !rs.is_empty() {
            rows.push(heading("REVIEWS", s));
            for r in rs {
                let who = r.user.as_ref().map(|u| u.login.as_str()).unwrap_or("?");
                let (color, label) = review_badge(&r.state);
                rows.push(line(vec![span(&format!("{} @{}", label, who), UI, s.text_px, color)], s.indent));
            }
            rows.push(MdRow::Blank);
        }
    }
}

fn merge_color(state: &str) -> Color {
    match state {
        "clean" => GREEN,
        "blocked" | "dirty" => RED,
        "behind" | "unstable" => YELLOW,
        _ => DIM,
    }
}

fn review_badge(state: &str) -> (Color, &'static str) {
    match state {
        "APPROVED" => (GREEN, "✓ approved"),
        "CHANGES_REQUESTED" => (RED, "✗ changes requested"),
        "DISMISSED" => (DIM, "• dismissed"),
        "COMMENTED" => (DIM, "• commented"),
        _ => (DIM, "• pending"),
    }
}
