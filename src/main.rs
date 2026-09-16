mod api;
mod auth;
mod calendar;
mod routes;
mod views;
mod components;

use dioxus::prelude::*;
use routes::Route;
use auth::provide_auth;
use components::toast::ToastProvider;

const FAVICON: Asset = asset!("/assets/favicon.ico");
const DX_COMPONENTS_CSS: Asset = asset!("/assets/dx-components-theme.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut auth = provide_auth();

    use_effect(move || {
        spawn(async move {
            if let Ok(token) = api::refresh().await {
                auth.login(token);
            }
            auth.finish_restoring();
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: DX_COMPONENTS_CSS }
        ToastProvider {
            Router::<Route> {}
        }
    }
}