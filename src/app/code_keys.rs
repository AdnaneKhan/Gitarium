//! Key handling on the Repo route's Code tab: the editor pass-through and
//! the browse-mode bindings.

use crate::ui::input::{Key, Mods};
use crate::ui::lineinput::LineInput;

use super::file_msgs::rehighlight;
use super::keys::plain;
use super::{App, ConfirmAction, Loadable, Overlay, RepoFocus, Route, SearchScope, Tab};

impl App {
    pub(super) fn repo_key(&mut self, key: Key, mods: Mods) -> bool {
        let Some(rv) = &mut self.rv else { return false };
        match rv.tab {
            Tab::Code => self.code_key(key, mods),
            Tab::Actions => self.actions_key(key, mods),
        }
    }

    pub(super) fn code_key(&mut self, key: Key, mods: Mods) -> bool {
        let in_editor = self.in_editor();
        let Some(rv) = self.rv.as_mut() else { return false };

        // Editor consumes nearly everything while editing.
        if in_editor {
            if key == Key::Char('s') && mods.ctrl {
                self.begin_commit();
                return true;
            }
            if key == Key::Esc {
                if let Some(f) = &mut rv.file {
                    f.editing = false;
                    f.editor.read_only = true;
                    if f.editor.modified {
                        self.toast =
                            Some(("buffer modified — press c to commit".into(), false));
                    }
                }
                return true;
            }
            let lay = self.layout;
            if let Some(f) = &mut rv.file {
                let changed = f.editor.handle_key(
                    &key,
                    mods,
                    lay.content_text.h.max(1) as usize,
                );
                if changed {
                    f.editor.ensure_visible(
                        lay.content_text.h.max(1) as usize,
                        lay.content_text.w.max(1) as usize,
                    );
                    rehighlight(f);
                }
                // Plain keys belong to the editor even when they change
                // nothing (arrow at a boundary); modified combos it didn't
                // act on stay with the browser.
                return changed || plain(mods);
            }
            return false;
        }

        match key {
            Key::Char('?') if plain(mods) => self.overlay = Some(Overlay::Help),
            Key::Char('/') if plain(mods) => {
                self.overlay = Some(Overlay::FileSearch { input: LineInput::new(false), sel: 0 });
            }
            Key::Char('g') if plain(mods) => {
                if self.token.is_none() {
                    self.toast = Some(("code search requires an access token".into(), true));
                } else {
                    self.overlay = Some(Overlay::CodeSearch {
                        input: LineInput::new(false),
                        sel: 0,
                        searched: String::new(),
                        results: Loadable::Idle,
                        scope: SearchScope::Repo,
                        page: 0,
                        more: false,
                        loading_more: false,
                    });
                }
            }
            Key::Char('b') if plain(mods) => {
                if rv.branches.ready().is_some() {
                    let sel = rv
                        .branches
                        .ready()
                        .and_then(|bs| bs.iter().position(|b| b.name == rv.branch))
                        .unwrap_or(0);
                    // Open with the current branch near the top of the list.
                    self.overlay = Some(Overlay::BranchPick { sel, scroll: sel.saturating_sub(3) });
                }
            }
            Key::Char('a') if plain(mods) => {
                rv.tab = Tab::Actions;
                if matches!(rv.runs, Loadable::Idle) {
                    self.load_runs();
                }
            }
            Key::Char('e') if plain(mods) => self.begin_edit(),
            Key::Char('c') if plain(mods) => self.begin_commit(),
            Key::Char('i') if plain(mods) => self.open_agent(),
            Key::Tab => {
                rv.focus = match rv.focus {
                    RepoFocus::Tree if rv.file.is_some() => RepoFocus::Content,
                    _ => RepoFocus::Tree,
                };
            }
            Key::Esc => {
                if rv.focus == RepoFocus::Content {
                    rv.focus = RepoFocus::Tree;
                    return true;
                }
                let modified = rv
                    .file
                    .as_ref()
                    .map(|f| f.editor.modified)
                    .unwrap_or(false);
                if modified {
                    self.overlay = Some(Overlay::Confirm {
                        msg: "discard unsaved edits and leave repo?".into(),
                        action: ConfirmAction::LeaveRepo,
                    });
                } else {
                    self.route = Route::Repos;
                    self.rv = None;
                }
            }
            _ => {
                return match rv.focus {
                    RepoFocus::Tree => self.tree_key(key),
                    RepoFocus::Content => self.viewer_key(key),
                }
            }
        }
        true
    }

    pub(super) fn viewer_key(&mut self, key: Key) -> bool {
        let lay = self.layout;
        let Some(rv) = self.rv.as_mut() else { return false };
        let Some(f) = &mut rv.file else { return false };
        let h = lay.content_text.h.max(1) as usize;
        match key {
            Key::Up => f.editor.scroll_by(-1, h),
            Key::Down => f.editor.scroll_by(1, h),
            Key::PageUp => f.editor.scroll_by(-(h as i32), h),
            Key::PageDown => f.editor.scroll_by(h as i32, h),
            Key::Home => f.editor.scroll = 0,
            Key::End => f.editor.scroll = f.editor.line_count().saturating_sub(h),
            _ => return false,
        }
        true
    }
}
