use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, decode_claims, Category};
use crate::auth::use_auth;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::input::Input;
use crate::components::label::Label;

const DEFAULT_COLOR: &str = "#3498db";

#[component]
pub fn Categories() -> Element {
    let auth = use_auth();
    let toast = use_toast();

    let mut categories = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_categories(claims.user_id, &token).await.ok()
    });

    let mut new_title = use_signal(String::new);
    let mut new_color = use_signal(|| DEFAULT_COLOR.to_string());
    let mut creating = use_signal(|| false);

    let on_create = move |evt: FormEvent| {
        evt.prevent_default();
        if creating() || new_title().trim().is_empty() {
            return;
        }
        let Some(token) = auth.token() else { return };
        creating.set(true);

        spawn(async move {
            match api::create_category(&token, new_title().trim(), Some(&new_color())).await {
                Ok(message) => {
                    creating.set(false);
                    new_title.set(String::new());
                    new_color.set(DEFAULT_COLOR.to_string());
                    toast.success(message, ToastOptions::new());
                    categories.restart();
                }
                Err(message) => {
                    creating.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    rsx! {
        div {
            style: "max-width: 480px; margin: 3rem auto; display: flex; flex-direction: column; gap: 1.5rem;",
            h1 { "Categories" }

            Card {
                CardHeader {
                    CardTitle { "New category" }
                    CardDescription { "Group your questions by topic" }
                }
                CardContent {
                    form {
                        id: "category-form",
                        onsubmit: on_create,
                        div { style: "display: flex; gap: 1rem; align-items: flex-end;",
                            div { style: "flex: 1; display: grid; gap: 0.5rem;",
                                Label { html_for: "category-title", "Title" }
                                Input {
                                    id: "category-title",
                                    r#type: "text",
                                    value: "{new_title}",
                                    oninput: move |e: FormEvent| new_title.set(e.value()),
                                }
                            }
                            div { style: "display: grid; gap: 0.5rem;",
                                Label { html_for: "category-color", "Color" }
                                input {
                                    id: "category-color",
                                    r#type: "color",
                                    value: "{new_color}",
                                    oninput: move |e: FormEvent| new_color.set(e.value()),
                                }
                            }
                        }
                    }
                }
                CardFooter {
                    Button {
                        r#type: "submit",
                        form: "category-form",
                        disabled: creating(),
                        if creating() { "Creating..." } else { "Create category" }
                    }
                }
            }

            match &*categories.read() {
                Some(Some(cats)) if !cats.is_empty() => rsx! {
                    div { style: "display: flex; flex-direction: column; gap: 0.75rem;",
                        for category in cats.clone() {
                            CategoryRow {
                                category,
                                on_changed: move |_| { categories.restart(); },
                            }
                        }
                    }
                },
                Some(Some(_)) => rsx! { p { style: "color: #888;", "No categories yet. Create your first one above." } },
                Some(None) => rsx! { p { "Unable to load your categories." } },
                None => rsx! { p { "Loading..." } },
            }
        }
    }
}

#[component]
fn CategoryRow(category: Category, on_changed: EventHandler<()>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let category_id = category.id;

    let mut is_editing = use_signal(|| false);
    let mut edit_title = use_signal(|| category.title.clone());
    let mut edit_color = use_signal(|| category.color_code.clone());
    let mut edit_submitting = use_signal(|| false);

    let mut confirming_delete = use_signal(|| false);
    let mut delete_submitting = use_signal(|| false);

    let on_save = move |_| {
        if edit_submitting() || edit_title().trim().is_empty() {
            return;
        }
        let Some(token) = auth.token() else { return };
        edit_submitting.set(true);

        spawn(async move {
            match api::update_category(category_id, &token, Some(edit_title().trim()), Some(&edit_color())).await {
                Ok(message) => {
                    edit_submitting.set(false);
                    is_editing.set(false);
                    toast.success(message, ToastOptions::new());
                    on_changed.call(());
                }
                Err(message) => {
                    edit_submitting.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    let on_delete = move |_| {
        if delete_submitting() {
            return;
        }
        let Some(token) = auth.token() else { return };
        delete_submitting.set(true);

        spawn(async move {
            match api::delete_category(category_id, &token).await {
                Ok(message) => {
                    delete_submitting.set(false);
                    confirming_delete.set(false);
                    toast.success(message, ToastOptions::new());
                    on_changed.call(());
                }
                Err(message) => {
                    delete_submitting.set(false);
                    confirming_delete.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    rsx! {
        Card {
            CardContent { style: "display: flex; align-items: center; gap: 1rem; padding: 1rem;",
                if is_editing() {
                    input {
                        r#type: "color",
                        value: "{edit_color}",
                        oninput: move |e: FormEvent| edit_color.set(e.value()),
                    }
                    Input {
                        r#type: "text",
                        value: "{edit_title}",
                        oninput: move |e: FormEvent| edit_title.set(e.value()),
                        style: "flex: 1;",
                    }
                    Button {
                        disabled: edit_submitting(),
                        onclick: on_save,
                        if edit_submitting() { "Saving..." } else { "Save" }
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        disabled: edit_submitting(),
                        onclick: move |_| is_editing.set(false),
                        "Cancel"
                    }
                } else {
                    span { style: "width: 1rem; height: 1rem; border-radius: 999px; background: {category.color_code}; flex-shrink: 0;" }
                    span { style: "flex: 1;", "{category.title}" }
                    if confirming_delete() {
                        Button {
                            variant: ButtonVariant::Destructive,
                            disabled: delete_submitting(),
                            onclick: on_delete,
                            if delete_submitting() { "Deleting..." } else { "Confirm" }
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            disabled: delete_submitting(),
                            onclick: move |_| confirming_delete.set(false),
                            "Cancel"
                        }
                    } else {
                        Button {
                            variant: ButtonVariant::Outline,
                            onclick: move |_| is_editing.set(true),
                            "Edit"
                        }
                        Button {
                            variant: ButtonVariant::Destructive,
                            onclick: move |_| confirming_delete.set(true),
                            "Delete"
                        }
                    }
                }
            }
        }
    }
}
