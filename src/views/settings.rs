use dioxus::prelude::*;
use crate::api::{self, decode_claims};
use crate::auth::use_auth;
use crate::routes::Route;
use crate::components::button::{Button, ButtonVariant};

#[component]
pub fn Settings() -> Element {
    let mut auth = use_auth();
    let nav = use_navigator();

    let user = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_user(claims.user_id, &token).await.ok()
    });

    let on_logout = move |_| {
        spawn(async move {
            // The frontend token is cleared either way — a failed request
            // to revoke the refresh cookie shouldn't trap the user in a
            // logged-in state they can't get out of.
            let _ = api::logout().await;
            auth.logout();
            nav.push(Route::Login {});
        });
    };

    rsx! {
        h1 { "Parameters" }
        match &*user.read() {
            Some(Some(user)) => rsx! {
                p { "Username: {user.username}" }
                p { "Email: {user.email}" }
            },
            Some(None) => rsx! { p { "Unable to load your profile." } },
            None => rsx! { p { "Loading..." } },
        }
        Button {
            variant: ButtonVariant::Outline,
            onclick: on_logout,
            "Log out"
        }
    }
}
