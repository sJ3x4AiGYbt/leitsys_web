use dioxus::prelude::*;
use crate::auth::use_auth;
use crate::views::{Login, Home, Settings, Contact};

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[route("/")]
    Root {},
    #[route("/login")]
    Login {},
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
        if auth.is_logged_in() {
            nav.replace(Route::Home {});
        } else {
            nav.replace(Route::Login {});
        }
    });

    rsx! { div {} }
}