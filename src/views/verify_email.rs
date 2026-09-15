use dioxus::prelude::*;
use crate::api;
use crate::routes::Route;
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::button::Button;

#[component]
pub fn VerifyEmail(token: String) -> Element {
    let nav = use_navigator();

    let result = use_resource(move || {
        let token = token.clone();
        async move { api::verify_email(&token).await }
    });

    rsx! {
        div {
            style: "max-width: 400px; margin: 5rem auto;",
            Card {
                style: "width: 100%; max-width: 24rem;",
                CardHeader {
                    CardTitle { "Email verification" }
                }
                CardContent {
                    match &*result.read() {
                        None => rsx! { CardDescription { "Verifying your email..." } },
                        Some(Ok(message)) => rsx! { CardDescription { "{message}" } },
                        Some(Err(message)) => rsx! { CardDescription { "{message}" } },
                    }
                }
                CardFooter {
                    Button {
                        style: "width: 100%;",
                        onclick: move |_| { nav.push(Route::Login {}); },
                        "Back to login"
                    }
                }
            }
        }
    }
}
