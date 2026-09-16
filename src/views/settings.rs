use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, decode_claims};
use crate::auth::use_auth;
use crate::routes::Route;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::input::Input;
use crate::components::label::Label;

#[component]
pub fn Settings() -> Element {
    let mut auth = use_auth();
    let nav = use_navigator();
    let toast = use_toast();

    let mut user = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_user(claims.user_id, &token).await.ok()
    });

    // Only seeded from the loaded profile once, so editing doesn't get
    // clobbered if `user` re-runs (e.g. after a successful profile save).
    let mut profile_initialized = use_signal(|| false);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut profile_submitting = use_signal(|| false);

    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut password_submitting = use_signal(|| false);

    let mut confirming_delete = use_signal(|| false);
    let mut delete_submitting = use_signal(|| false);

    use_effect(move || {
        if let Some(Some(u)) = &*user.read() {
            if !profile_initialized() {
                username.set(u.username.clone());
                email.set(u.email.clone());
                profile_initialized.set(true);
            }
        }
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

    let on_save_profile = move |evt: FormEvent| {
        evt.prevent_default();
        if profile_submitting() {
            return;
        }
        let Some(token) = auth.token() else { return };
        let Some(claims) = decode_claims(&token) else { return };
        profile_submitting.set(true);

        spawn(async move {
            match api::update_profile(claims.user_id, &token, &username(), &email()).await {
                Ok(message) => {
                    profile_submitting.set(false);
                    toast.success(message, ToastOptions::new());
                    user.restart();
                }
                Err(message) => {
                    profile_submitting.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    let on_change_password = move |evt: FormEvent| {
        evt.prevent_default();
        if password_submitting() {
            return;
        }
        if new_password() != confirm_password() {
            toast.error("Passwords do not match.".to_string(), ToastOptions::new());
            return;
        }
        let Some(token) = auth.token() else { return };
        let Some(claims) = decode_claims(&token) else { return };
        password_submitting.set(true);

        spawn(async move {
            match api::change_password(claims.user_id, &token, &new_password()).await {
                Ok(message) => {
                    password_submitting.set(false);
                    new_password.set(String::new());
                    confirm_password.set(String::new());
                    toast.success(message, ToastOptions::new());
                }
                Err(message) => {
                    password_submitting.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    let on_delete_account = move |_| {
        if delete_submitting() {
            return;
        }
        let Some(token) = auth.token() else { return };
        let Some(claims) = decode_claims(&token) else { return };
        delete_submitting.set(true);

        spawn(async move {
            match api::delete_user(claims.user_id, &token).await {
                Ok(_) => {
                    auth.logout();
                    nav.push(Route::Login {});
                }
                Err(message) => {
                    delete_submitting.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    rsx! {
        div {
            style: "max-width: 480px; margin: 3rem auto; display: flex; flex-direction: column; gap: 1.5rem;",
            h1 { "Parameters" }

            match &*user.read() {
                Some(Some(_)) => rsx! {
                    Card {
                        CardHeader {
                            CardTitle { "Profile" }
                            CardDescription { "Update your username and email" }
                        }
                        CardContent {
                            form {
                                id: "profile-form",
                                onsubmit: on_save_profile,
                                div { style: "display: flex; flex-direction: column; gap: 1.5rem;",
                                    div { style: "display: grid; gap: 0.5rem;",
                                        Label { html_for: "username", "Username" }
                                        Input {
                                            id: "username",
                                            r#type: "text",
                                            value: "{username}",
                                            oninput: move |e: FormEvent| username.set(e.value()),
                                        }
                                    }
                                    div { style: "display: grid; gap: 0.5rem;",
                                        Label { html_for: "email", "Email" }
                                        Input {
                                            id: "email",
                                            r#type: "email",
                                            value: "{email}",
                                            oninput: move |e: FormEvent| email.set(e.value()),
                                        }
                                    }
                                }
                            }
                        }
                        CardFooter {
                            Button {
                                r#type: "submit",
                                form: "profile-form",
                                disabled: profile_submitting(),
                                if profile_submitting() { "Saving..." } else { "Save changes" }
                            }
                        }
                    }

                    Card {
                        CardHeader {
                            CardTitle { "Password" }
                            CardDescription { "Change your password" }
                        }
                        CardContent {
                            form {
                                id: "password-form",
                                onsubmit: on_change_password,
                                div { style: "display: flex; flex-direction: column; gap: 1.5rem;",
                                    div { style: "display: grid; gap: 0.5rem;",
                                        Label { html_for: "new-password", "New password" }
                                        Input {
                                            id: "new-password",
                                            r#type: "password",
                                            value: "{new_password}",
                                            oninput: move |e: FormEvent| new_password.set(e.value()),
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
                        CardFooter {
                            Button {
                                r#type: "submit",
                                form: "password-form",
                                disabled: password_submitting(),
                                if password_submitting() { "Changing..." } else { "Change password" }
                            }
                        }
                    }

                    Card {
                        CardHeader {
                            CardTitle { "Delete account" }
                            CardDescription { "This permanently deletes your account and all your data." }
                        }
                        CardFooter { style: "gap: 0.5rem;",
                            if confirming_delete() {
                                Button {
                                    variant: ButtonVariant::Destructive,
                                    disabled: delete_submitting(),
                                    onclick: on_delete_account,
                                    if delete_submitting() { "Deleting..." } else { "Confirm delete" }
                                }
                                Button {
                                    variant: ButtonVariant::Ghost,
                                    disabled: delete_submitting(),
                                    onclick: move |_| confirming_delete.set(false),
                                    "Cancel"
                                }
                            } else {
                                Button {
                                    variant: ButtonVariant::Destructive,
                                    onclick: move |_| confirming_delete.set(true),
                                    "Delete account"
                                }
                            }
                        }
                    }
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
}
