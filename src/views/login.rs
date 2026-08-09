use dioxus::prelude::*;
use crate::auth::use_auth;
use crate::routes::Route;
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};

#[derive(Clone, Copy, PartialEq)]
enum AuthView {
    Login,
    Signup,
    ForgotPassword,
}

#[component]
pub fn Login() -> Element {
    let view = use_signal(|| AuthView::Login);

    rsx! {
        div {
            style: "max-width: 400px; margin: 5rem auto;",
            Card {
                match view() {
                    AuthView::Login => rsx! { LoginForm { view } },
                    AuthView::Signup => rsx! { SignupForm { view } },
                    AuthView::ForgotPassword => rsx! { ForgotPasswordForm { view } },
                }
            }
        }
    }
}

#[component]
fn LoginForm(mut view: Signal<AuthView>) -> Element {
    let mut auth = use_auth();
    let nav = use_navigator();

    let on_submit = move |_| {
        // ... credential verification ...
        auth.login();
        nav.push(Route::Home {});
    };

    rsx! {
        CardHeader {
            CardTitle { "Log in" }
            CardDescription { "Enter your credentials to continue" }
        }
        CardContent {
            input { r#type: "email", placeholder: "Email" }
            input { r#type: "password", placeholder: "Password" }
            button { onclick: on_submit, "Log in" }
        }
        CardFooter {
            style: "display: flex; justify-content: space-between; width: 100%;",
            a {
                onclick: move |_| view.set(AuthView::ForgotPassword),
                "Forgot password?"
            }
            a {
                onclick: move |_| view.set(AuthView::Signup),
                "Create an account"
            }
        }
    }
}

#[component]
fn SignupForm(mut view: Signal<AuthView>) -> Element {
    let on_submit = move |_| {
        // ... account creation ...
    };

    rsx! {
        CardHeader {
            CardTitle { "Create an account" }
            CardDescription { "Fill in your information" }
        }
        CardContent {
            input { r#type: "text", placeholder: "Name" }
            input { r#type: "email", placeholder: "Email" }
            input { r#type: "password", placeholder: "Password" }
            button { onclick: on_submit, "Sign up" }
        }
        CardFooter {
            a {
                onclick: move |_| view.set(AuthView::Login),
                "Already have an account? Log in"
            }
        }
    }
}

#[component]
fn ForgotPasswordForm(mut view: Signal<AuthView>) -> Element {
    let on_submit = move |_| {
        // ... send mail for reset ...
    };

    rsx! {
        CardHeader {
            CardTitle { "Forgot password" }
            CardDescription { "We'll send you a reset link" }
        }
        CardContent {
            input { r#type: "email", placeholder: "Email" }
            button { onclick: on_submit, "Send link" }
        }
        CardFooter {
            a {
                onclick: move |_| view.set(AuthView::Login),
                "Back to login"
            }
        }
    }
}