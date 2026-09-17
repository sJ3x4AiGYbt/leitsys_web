use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, decode_claims, Category, Question};
use crate::auth::use_auth;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};
use crate::components::input::Input;
use crate::components::label::Label;

#[component]
pub fn Questions() -> Element {
    let auth = use_auth();
    let toast = use_toast();

    let categories = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_categories(claims.user_id, &token).await.ok()
    });

    let mut view_archived = use_signal(|| false);

    let mut questions = use_resource(move || {
        let archived = view_archived();
        async move {
            let token = auth.token()?;
            let claims = decode_claims(&token)?;
            api::get_my_questions(claims.user_id, &token, archived, false).await.ok()
        }
    });

    let mut new_title = use_signal(String::new);
    let mut new_answer = use_signal(String::new);
    let mut new_category_id = use_signal(|| None::<i64>);
    let mut creating = use_signal(|| false);

    // Default the create-form category to the first one once categories load.
    use_effect(move || {
        if new_category_id().is_none() {
            if let Some(Some(cats)) = &*categories.read() {
                if let Some(first) = cats.first() {
                    new_category_id.set(Some(first.id));
                }
            }
        }
    });

    let on_create = move |evt: FormEvent| {
        evt.prevent_default();
        if creating() || new_title().trim().is_empty() || new_answer().trim().is_empty() {
            return;
        }
        let Some(category_id) = new_category_id() else { return };
        let Some(token) = auth.token() else { return };
        creating.set(true);

        spawn(async move {
            match api::create_question(&token, new_title().trim(), new_answer().trim(), category_id).await {
                Ok(message) => {
                    creating.set(false);
                    new_title.set(String::new());
                    new_answer.set(String::new());
                    toast.success(message, ToastOptions::new());
                    questions.restart();
                }
                Err(message) => {
                    creating.set(false);
                    toast.error(message, ToastOptions::new());
                }
            }
        });
    };

    let cats_for_form: Vec<Category> = match &*categories.read() {
        Some(Some(cats)) => cats.clone(),
        _ => Vec::new(),
    };
    let has_categories = !cats_for_form.is_empty();

    rsx! {
        div {
            style: "max-width: 640px; margin: 3rem auto; display: flex; flex-direction: column; gap: 1.5rem;",
            h1 { "Questions" }

            div { style: "display: flex; gap: 0.5rem;",
                Button {
                    variant: if view_archived() { ButtonVariant::Outline } else { ButtonVariant::Primary },
                    onclick: move |_| view_archived.set(false),
                    "Active"
                }
                Button {
                    variant: if view_archived() { ButtonVariant::Primary } else { ButtonVariant::Outline },
                    onclick: move |_| view_archived.set(true),
                    "Mastered"
                }
            }

            if !view_archived() {
            Card {
                CardHeader {
                    CardTitle { "New question" }
                    CardDescription { "Added to the first review step by default" }
                }
                CardContent {
                    if has_categories {
                        form {
                            id: "question-form",
                            onsubmit: on_create,
                            div { style: "display: flex; flex-direction: column; gap: 1rem;",
                                div { style: "display: grid; gap: 0.5rem;",
                                    Label { html_for: "question-title", "Title" }
                                    Input {
                                        id: "question-title",
                                        r#type: "text",
                                        value: "{new_title}",
                                        oninput: move |e: FormEvent| new_title.set(e.value()),
                                    }
                                }
                                div { style: "display: grid; gap: 0.5rem;",
                                    Label { html_for: "question-answer", "Answer" }
                                    textarea {
                                        id: "question-answer",
                                        rows: "3",
                                        style: "padding: 0.5rem; border-radius: 6px; border: 1px solid #ccc; font: inherit;",
                                        value: "{new_answer}",
                                        oninput: move |e: FormEvent| new_answer.set(e.value()),
                                    }
                                }
                                div { style: "display: grid; gap: 0.5rem;",
                                    Label { html_for: "question-category", "Category" }
                                    select {
                                        id: "question-category",
                                        onchange: move |e: FormEvent| new_category_id.set(e.value().parse().ok()),
                                        for cat in cats_for_form.clone() {
                                            option {
                                                value: "{cat.id}",
                                                selected: new_category_id() == Some(cat.id),
                                                "{cat.title}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        p { style: "color: #888;", "Create a category first before adding questions." }
                    }
                }
                CardFooter {
                    Button {
                        r#type: "submit",
                        form: "question-form",
                        disabled: creating() || !has_categories,
                        if creating() { "Creating..." } else { "Create question" }
                    }
                }
            }
            }

            if view_archived() {
                match &*questions.read() {
                    Some(Some(qs)) if !qs.is_empty() => rsx! {
                        div { style: "display: flex; flex-direction: column; gap: 0.75rem;",
                            for question in qs.clone() {
                                ArchivedQuestionRow {
                                    category: cats_for_form.iter().find(|c| c.id == question.category_id).cloned(),
                                    question,
                                    on_changed: move |_| { questions.restart(); },
                                }
                            }
                        }
                    },
                    Some(Some(_)) => rsx! { p { style: "color: #888;", "No mastered questions yet." } },
                    Some(None) => rsx! { p { "Unable to load your questions." } },
                    None => rsx! { p { "Loading..." } },
                }
            } else {
                match &*questions.read() {
                    Some(Some(qs)) if !qs.is_empty() => rsx! {
                        div { style: "display: flex; flex-direction: column; gap: 0.75rem;",
                            for question in qs.clone() {
                                QuestionRow {
                                    category: cats_for_form.iter().find(|c| c.id == question.category_id).cloned(),
                                    question,
                                    categories: cats_for_form.clone(),
                                    on_changed: move |_| { questions.restart(); },
                                }
                            }
                        }
                    },
                    Some(Some(_)) => rsx! { p { style: "color: #888;", "No questions yet. Create your first one above." } },
                    Some(None) => rsx! { p { "Unable to load your questions." } },
                    None => rsx! { p { "Loading..." } },
                }
            }
        }
    }
}

#[component]
fn ArchivedQuestionRow(question: Question, category: Option<Category>, on_changed: EventHandler<()>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let question_id = question.id;

    let mut confirming_delete = use_signal(|| false);
    let mut delete_submitting = use_signal(|| false);

    let on_delete = move |_| {
        if delete_submitting() {
            return;
        }
        let Some(token) = auth.token() else { return };
        delete_submitting.set(true);

        spawn(async move {
            match api::delete_question(question_id, &token).await {
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
            CardContent { style: "display: flex; flex-direction: column; gap: 0.5rem; padding: 1rem;",
                div { style: "display: flex; align-items: center; gap: 0.75rem;",
                    if let Some(category) = &category {
                        span { style: "width: 0.75rem; height: 0.75rem; border-radius: 999px; background: {category.color_code}; flex-shrink: 0;" }
                        span { style: "font-size: 0.75rem; color: #888;", "{category.title}" }
                    }
                }
                div { style: "font-weight: 600;", "{question.title}" }
                div { style: "color: #888;", "{question.answer}" }
                div { style: "display: flex; gap: 0.5rem;",
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

#[component]
fn QuestionRow(question: Question, category: Option<Category>, categories: Vec<Category>, on_changed: EventHandler<()>) -> Element {
    let auth = use_auth();
    let toast = use_toast();
    let question_id = question.id;
    let due_date = question.next_review_date.get(..10).unwrap_or(&question.next_review_date).to_string();

    let mut is_editing = use_signal(|| false);
    let mut edit_title = use_signal(|| question.title.clone());
    let mut edit_answer = use_signal(|| question.answer.clone());
    let mut edit_category_id = use_signal(|| Some(question.category_id));
    let mut edit_submitting = use_signal(|| false);

    let mut confirming_delete = use_signal(|| false);
    let mut delete_submitting = use_signal(|| false);

    let on_save = move |_| {
        if edit_submitting() || edit_title().trim().is_empty() || edit_answer().trim().is_empty() {
            return;
        }
        let Some(token) = auth.token() else { return };
        edit_submitting.set(true);

        spawn(async move {
            match api::update_question(question_id, &token, Some(edit_title().trim()), Some(edit_answer().trim()), edit_category_id()).await {
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
            match api::delete_question(question_id, &token).await {
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
            CardContent { style: "display: flex; flex-direction: column; gap: 0.75rem; padding: 1rem;",
                if is_editing() {
                    Input {
                        r#type: "text",
                        value: "{edit_title}",
                        oninput: move |e: FormEvent| edit_title.set(e.value()),
                    }
                    textarea {
                        rows: "2",
                        style: "padding: 0.5rem; border-radius: 6px; border: 1px solid #ccc; font: inherit;",
                        value: "{edit_answer}",
                        oninput: move |e: FormEvent| edit_answer.set(e.value()),
                    }
                    select {
                        onchange: move |e: FormEvent| edit_category_id.set(e.value().parse().ok()),
                        for cat in categories.clone() {
                            option {
                                value: "{cat.id}",
                                selected: edit_category_id() == Some(cat.id),
                                "{cat.title}"
                            }
                        }
                    }
                    div { style: "display: flex; gap: 0.5rem;",
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
                    }
                } else {
                    div { style: "display: flex; align-items: center; gap: 0.75rem;",
                        if let Some(category) = &category {
                            span { style: "width: 0.75rem; height: 0.75rem; border-radius: 999px; background: {category.color_code}; flex-shrink: 0;" }
                            span { style: "font-size: 0.75rem; color: #888;", "{category.title}" }
                        }
                        span { style: "margin-left: auto; font-size: 0.75rem; color: #888;", "Due {due_date}" }
                    }
                    div { style: "font-weight: 600;", "{question.title}" }
                    div { style: "display: flex; gap: 0.5rem;",
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
}
