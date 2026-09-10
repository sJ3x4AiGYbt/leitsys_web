use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct AuthState {
    access_token: Signal<Option<String>>,
}

impl AuthState {
    pub fn login(&mut self, access_token: String) {
        self.access_token.set(Some(access_token));
    }

    pub fn logout(&mut self) {
        self.access_token.set(None);
    }

    pub fn is_logged_in(&self) -> bool {
        self.access_token.read().is_some()
    }

    pub fn token(&self) -> Option<String> {
        self.access_token.read().clone()
    }
}

pub fn provide_auth() {
    use_context_provider(|| AuthState {
        access_token: Signal::new(None),
    });
}

pub fn use_auth() -> AuthState {
    use_context::<AuthState>()
}