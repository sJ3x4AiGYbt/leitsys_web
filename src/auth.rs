use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct AuthState {
    pub is_authenticated: Signal<bool>,
}

impl AuthState {
    pub fn login(&mut self) {
        self.is_authenticated.set(true);
    }

    pub fn logout(&mut self) {
        self.is_authenticated.set(false);
    }

    pub fn is_logged_in(&self) -> bool {
        (self.is_authenticated)()
    }
}

pub fn provide_auth() {
    use_context_provider(|| AuthState {
        is_authenticated: Signal::new(false),
    });
}

pub fn use_auth() -> AuthState {
    use_context::<AuthState>()
}