//! Status-bar key hints, one line per screen/mode. Kept apart from the
//! chrome drawing so each stays focused.

use crate::app::{App, RepoSource, Route, Tab};

pub(super) fn route_hints(app: &App) -> &'static str {
    match app.route {
        Route::Auth => "[ENTER] CONTINUE",
        Route::Repos => {
            if app.filter_active {
                "[ENTER] APPLY · [ESC] CLEAR"
            } else if app.repo_source != RepoSource::Mine {
                "[/] FILTER · [O] OPEN · [G] SEARCH · [S] SORT · [F] FORKS · [X] ARCHIVED · [ESC] MY REPOS"
            } else {
                "[/] FILTER · [O] OPEN · [G] CODE SEARCH · [S] SORT · [F/X] FORKS/ARCHIVED · [I] AGENT · [?] HELP"
            }
        }
        Route::Repo => repo_hints(app),
        Route::Agent => {
            if app.anthropic_key.is_none() {
                "[ENTER] SAVE · [TAB] SWITCH FIELD · [ESC] BACK"
            } else if app.agent.busy {
                "[ESC] CANCEL"
            } else {
                "[ENTER] SEND · [ESC] BACK"
            }
        }
    }
}

fn repo_hints(app: &App) -> &'static str {
    if app.in_editor() {
        return "[CTRL+S] COMMIT · [CTRL+Z] UNDO · [ESC] VIEW MODE";
    }
    let rv = app.rv.as_ref();
    if let Some(d) = rv.and_then(|rv| rv.detail.as_ref()) {
        if d.search.is_some() {
            return "TYPE TO SEARCH · [↑↓/ENTER] NEXT/PREV · [ESC] CLOSE";
        }
        return if d.is_pr {
            "[/] SEARCH · [A] APPROVE · [M] MERGE · [↑↓] SCROLL · [ESC] BACK"
        } else {
            "[/] SEARCH · [↑↓] SCROLL · [I] AGENT · [ESC] BACK"
        };
    }
    match rv.map(|rv| rv.tab) {
        Some(Tab::Actions) => "[ENTER] JOBS · [R] REFRESH · [T] ISSUES · [P] PULLS · [ESC] CODE",
        Some(Tab::Issues) | Some(Tab::Pulls) => {
            "[ENTER] OPEN · [R] REFRESH · [T] ISSUES · [P] PULLS · [A] ACTIONS · [ESC] CODE"
        }
        _ => "[ENTER] OPEN · [/] FIND · [G] SEARCH · [E] EDIT · [B] BRANCH · [T] ISSUES · [P] PULLS · [A] ACTIONS · [I] AGENT · [ESC] BACK",
    }
}
