//! The auth screen: validating a pasted token (or entering anonymous mode)
//! and the key handling for the token field.

use crate::github;
use crate::ui::input::{Key, Mods};

use super::{App, LineInput, Loadable, Msg, Overlay, Route};

impl App {
    pub(super) fn validate_token(&mut self, t: String) {
        self.auth_busy = true;
        self.auth_error = None;
        let token = Some(t.clone());
        crate::spawn_msg(async move {
            let result = github::current_user(&token).await;
            Msg::TokenChecked { token, result }
        });
    }

    pub(super) fn on_token_checked(
        &mut self,
        token: Option<String>,
        result: Result<github::User, String>,
    ) {
        self.auth_busy = false;
        match result {
            Ok(user) => {
                self.token = token;
                self.login = Some(user.login);
                self.route = Route::Repos;
                self.load_repos();
            }
            Err(e) => {
                self.auth_error = Some(e);
                self.route = Route::Auth;
            }
        }
    }

    pub(super) fn auth_key(&mut self, key: Key, mods: Mods) -> bool {
        if self.auth_busy {
            return false;
        }
        match key {
            Key::Enter => {
                let t = self.token_input.text.trim().to_string();
                if t.is_empty() {
                    // Anonymous mode: public repos, read-only commits will fail.
                    self.token = None;
                    self.login = None;
                    self.route = Route::Repos;
                    self.repos = Loadable::Idle;
                    self.overlay = Some(Overlay::OpenRepo(LineInput::new(false)));
                } else {
                    self.validate_token(t);
                }
                true
            }
            k => self.token_input.handle_key(&k, mods),
        }
    }
}
