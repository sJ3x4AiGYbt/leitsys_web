use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, decode_claims, Step};
use crate::auth::use_auth;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::input::Input;
use crate::components::label::Label;

const DEFAULT_COLOR: &str = "#95a5a6";

#[component]
pub fn Steps() -> Element {
    let auth = use_auth();
    let toast = use_toast();

    let mut steps = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_steps(claims.user_id, &token).await.ok()
    });

    let mut new_title = use_signal(String::new);
    let mut new_spacing_days = use_signal(|| "1".to_string());
    let mut new_color = use_signal(|| DEFAULT_COLOR.to_string());
    let mut creating = use_signal(|| false);

    let on_create = move |evt: FormEvent| {
        evt.prevent_default();
        if creating() || new_title().trim().is_empty() {
            return;
        }
        let Ok(spacing_days) = new_spacing_days().trim().parse::<i64>() else { return };
        if spacing_days < 1 {
            return;
        }
        let Some(token) = auth.token() else { return };
        creating.set(true);

        spawn(async move {
            match api::create_step(&token, new_title().trim(), spacing_days, Some(&new_color())).await {
                Ok(message) => {
                    creating.set(false);
                    new_title.set(String::new());
                    new_spacing_days.set("1".to_string());
                    new_color.set(DEFAULT_COLOR.to_string());
                    toast.success(message, ToastOptions::new());
                    steps.restart();
                }
                Err(message) => {
                    creating.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    let mut sorted: Vec<Step> = match &*steps.read() {
        Some(Some(s)) => s.clone(),
        _ => Vec::new(),
    };
    sorted.sort_by_key(|s| s.step_order);
    let count = sorted.len();

    rsx! {
        div {
            class: "page",
            h1 { "Steps" }
            p { class: "muted", "Questions move through these steps as you review them. Spacing is in days." }

            Card {
                CardHeader {
                    CardTitle { "New step" }
                    CardDescription { "Added at the end of the sequence" }
                }
                CardContent {
                    form {
                        id: "step-form",
                        onsubmit: on_create,
                        div { class: "row-end",
                            div { class: "field grow",
                                Label { html_for: "step-title", "Title" }
                                Input {
                                    id: "step-title",
                                    r#type: "text",
                                    value: "{new_title}",
                                    oninput: move |e: FormEvent| new_title.set(e.value()),
                                }
                            }
                            div { class: "field", style: "width: 6rem;",
                                Label { html_for: "step-spacing", "Days" }
                                input {
                                    id: "step-spacing",
                                    r#type: "number",
                                    min: "1",
                                    value: "{new_spacing_days}",
                                    oninput: move |e: FormEvent| new_spacing_days.set(e.value()),
                                }
                            }
                            div { class: "field",
                                Label { html_for: "step-color", "Color" }
                                input {
                                    id: "step-color",
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
                        form: "step-form",
                        disabled: creating(),
                        if creating() { "Creating..." } else { "Create step" }
                    }
                }
            }

            match &*steps.read() {
                Some(Some(_)) if count > 0 => rsx! {
                    div { class: "stack-sm",
                        for (i, step) in sorted.clone().into_iter().enumerate() {
                            StepRow {
                                step,
                                is_first: i == 0,
                                is_last: i == count - 1,
                                on_changed: move |_| { steps.restart(); },
                            }
                        }
                    }
                },
                Some(Some(_)) => rsx! { p { class: "muted", "No steps yet. Create your first one above." } },
                Some(None) => rsx! { p { "Unable to load your steps." } },
                None => rsx! { p { "Loading..." } },
            }
        }
    }
}

#[component]
fn StepRow(step: Step, is_first: bool, is_last: bool, on_changed: EventHandler<()>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let step_id = step.id;
    let step_order = step.step_order;

    let mut is_editing = use_signal(|| false);
    let mut edit_title = use_signal(|| step.title.clone());
    let mut edit_spacing_days = use_signal(|| step.spacing_days.to_string());
    let mut edit_color = use_signal(|| step.color_code.clone());
    let mut edit_submitting = use_signal(|| false);

    let mut confirming_delete = use_signal(|| false);
    let mut delete_submitting = use_signal(|| false);

    let mut moving = use_signal(|| false);

    let on_save = move |_| {
        if edit_submitting() || edit_title().trim().is_empty() {
            return;
        }
        let Ok(spacing_days) = edit_spacing_days().trim().parse::<i64>() else { return };
        if spacing_days < 1 {
            return;
        }
        let Some(token) = auth.token() else { return };
        edit_submitting.set(true);

        spawn(async move {
            match api::update_step(step_id, &token, Some(edit_title().trim()), None, Some(spacing_days), Some(&edit_color())).await {
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

    let mut on_move = move |delta: i64| {
        if moving() {
            return;
        }
        let Some(token) = auth.token() else { return };
        moving.set(true);

        spawn(async move {
            match api::update_step(step_id, &token, None, Some(step_order + delta), None, None).await {
                Ok(_) => {
                    moving.set(false);
                    on_changed.call(());
                }
                Err(message) => {
                    moving.set(false);
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
            match api::delete_step(step_id, &token).await {
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
        Card { "data-size": "compact",
            CardContent { style: "display: flex; align-items: center; gap: 0.75rem;",
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
                    input {
                        r#type: "number",
                        min: "1",
                        style: "width: 5rem;",
                        value: "{edit_spacing_days}",
                        oninput: move |e: FormEvent| edit_spacing_days.set(e.value()),
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
                    div { style: "display: flex; flex-direction: column;",
                        button {
                            class: "icon-button icon-button-sm",
                            title: "Move up",
                            disabled: is_first || moving(),
                            onclick: move |_| on_move(-1),
                            "▲"
                        }
                        button {
                            class: "icon-button icon-button-sm",
                            title: "Move down",
                            disabled: is_last || moving(),
                            onclick: move |_| on_move(1),
                            "▼"
                        }
                    }
                    span { class: "dot dot-lg", style: "background: {step.color_code};" }
                    span { class: "grow", "{step.title}" }
                    span { class: "muted small", "{step.spacing_days} day(s)" }
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
