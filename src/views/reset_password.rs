use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api;
use crate::routes::Route;
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::button::Button;
use crate::components::input::Input;
use crate::components::label::Label;

#[component]
pub fn ResetPassword(token: String) -> Element {
    let nav = use_navigator();
    let toast = use_toast();

    let mut password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut submitting = use_signal(|| false);

    let on_submit = move |evt: FormEvent| {
        evt.prevent_default();
        if submitting() {
            return;
        }
        if password() != confirm_password() {
            toast.error("Passwords do not match.".to_string(), ToastOptions::new());
            return;
        }
        submitting.set(true);

        let token = token.clone();
        spawn(async move {
            match api::reset_password(&token, &password()).await {
                Ok(message) => {
                    toast.success(message, ToastOptions::new());
                    nav.push(Route::Login {});
                }
                Err(message) => {
                    toast.error(message, ToastOptions::new());
                    submitting.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            style: "max-width: 400px; margin: 5rem auto;",
            Card {
                style: "width: 100%; max-width: 24rem;",
                CardHeader {
                    CardTitle { "Reset password" }
                    CardDescription { "Enter a new password for your account" }
                }
                CardContent {
                    form {
                        id: "reset-password-form",
                        onsubmit: on_submit,
                        div { style: "display: flex; flex-direction: column; gap: 1.5rem;",
                            div { style: "display: grid; gap: 0.5rem;",
                                Label { html_for: "password", "New password" }
                                Input {
                                    id: "password",
                                    r#type: "password",
                                    value: "{password}",
                                    oninput: move |e: FormEvent| password.set(e.value()),
                                }
                            }
                            div { style: "display: grid; gap: 0.5rem;",
                                Label { html_for: "confirm-password", "Confirm password" }
                                Input {
                                    id: "confirm-password",
                                    r#type: "password",
                                    value: "{confirm_password}",
                                    oninput: move |e: FormEvent| confirm_password.set(e.value()),
                                }
                            }
                        }
                    }
                }
                CardFooter { style: "flex-direction: column; gap: 0.5rem;",
                    Button {
                        r#type: "submit",
                        form: "reset-password-form",
                        style: "width: 100%;",
                        disabled: submitting(),
                        if submitting() { "Resetting..." } else { "Reset password" }
                    }
                }
            }
        }
    }
}
