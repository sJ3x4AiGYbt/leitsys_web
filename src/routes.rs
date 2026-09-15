use dioxus::prelude::*;
use crate::auth::use_auth;
use crate::views::{Login, Home, Settings, Contact, ResetPassword, VerifyEmail};

#[rustfmt::skip]
#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Root {},
    #[route("/login")]
    Login {},
    #[route("/reset-password?:token")]
    ResetPassword { token: String },
    #[route("/verify-email?:token")]
    VerifyEmail { token: String },
    #[layout(RequireAuth)]
        #[route("/home")]
        Home {},
        #[route("/settings")]
        Settings {},
        #[route("/contact")]
        Contact {},
}

#[component]
fn Root() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    use_effect(move || {
        // Wait for the silent-refresh attempt to settle — on page load the
        // token is still empty regardless of whether the refresh cookie
        // will restore a session a moment later.
        if auth.is_restoring() {
            return;
        }
        if auth.is_logged_in() {
            nav.replace(Route::Home {});
        } else {
            nav.replace(Route::Login {});
        }
    });

    rsx! { div {} }
}

/// Layout guarding `/home`, `/settings` and `/contact` — redirects to
/// `/login` if the user has no access token instead of rendering the
/// nested route.
#[component]
fn RequireAuth() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    use_effect(move || {
        if !auth.is_restoring() && !auth.is_logged_in() {
            nav.replace(Route::Login {});
        }
    });

    if auth.is_restoring() {
        // Avoid flashing the protected page's content before we know
        // whether the refresh cookie actually restores a session.
        rsx! { div {} }
    } else if auth.is_logged_in() {
        rsx! { Outlet::<Route> {} }
    } else {
        rsx! { div {} }
    }
}