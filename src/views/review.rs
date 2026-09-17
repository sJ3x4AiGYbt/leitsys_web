use dioxus::prelude::*;
use dioxus_primitives::toast::{use_toast, ToastOptions};
use crate::api::{self, decode_claims, Question};
use crate::auth::use_auth;
use crate::routes::Route;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter};

#[component]
pub fn Review() -> Element {
    let auth = use_auth();
    let nav = use_navigator();
    let toast = use_toast();

    let due_questions = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_questions(claims.user_id, &token, false, true).await.ok()
    });

    let mut seeded = use_signal(|| false);
    let mut queue = use_signal(Vec::<Question>::new);

    use_effect(move || {
        if let Some(Some(qs)) = &*due_questions.read() {
            if !seeded() {
                queue.set(qs.clone());
                seeded.set(true);
            }
        }
    });

    let mut revealed = use_signal(|| false);
    let mut user_response = use_signal(String::new);
    let mut submitting = use_signal(|| false);

    let current = queue().first().cloned();

    let mut on_grade = move |is_correct: bool| {
        if submitting() {
            return;
        }
        let Some(question) = queue().first().cloned() else { return };
        let Some(token) = auth.token() else { return };
        submitting.set(true);
        let response = user_response();

        spawn(async move {
            let answer_result = api::create_answer(&token, question.id, &response, question.current_step_id, is_correct).await;

            let grading_result = match answer_result {
                Ok(answer_id) if is_correct => api::mark_answer_correct(answer_id, &token).await,
                Ok(answer_id) => api::mark_answer_incorrect(answer_id, &token).await,
                Err(message) => Err(message),
            };

            match grading_result {
                Ok(message) => {
                    toast.success(message, ToastOptions::new());
                    queue.write().retain(|q| q.id != question.id);
                    revealed.set(false);
                    user_response.set(String::new());
                    submitting.set(false);
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
            style: "max-width: 480px; margin: 3rem auto; display: flex; flex-direction: column; gap: 1.5rem;",
            div { style: "display: flex; align-items: center; justify-content: space-between;",
                h1 { "Review" }
                Button {
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| { nav.push(Route::Home {}); },
                    "Back home"
                }
            }

            match (&*due_questions.read(), current) {
                (None, _) => rsx! { p { "Loading..." } },
                (Some(None), _) => rsx! { p { "Unable to load your review queue." } },
                (Some(Some(_)), None) => rsx! {
                    Card {
                        CardHeader {
                            CardTitle { "Nothing to review right now" }
                            CardDescription { "Come back when a question is due, or add new ones." }
                        }
                        CardFooter {
                            Button {
                                onclick: move |_| { nav.push(Route::Home {}); },
                                "Back home"
                            }
                        }
                    }
                },
                (Some(Some(_)), Some(question)) => rsx! {
                    p { style: "color: #888; margin: 0;", "{queue().len()} left" }
                    Card {
                        CardHeader {
                            CardTitle { "{question.title}" }
                        }
                        CardContent { style: "display: flex; flex-direction: column; gap: 1rem;",
                            if revealed() {
                                div { style: "padding: 0.75rem; border-radius: 6px; background: rgba(127, 127, 127, 0.1);",
                                    "{question.answer}"
                                }
                            } else {
                                textarea {
                                    rows: "3",
                                    placeholder: "Your answer (optional)",
                                    style: "padding: 0.5rem; border-radius: 6px; border: 1px solid #ccc; font: inherit;",
                                    value: "{user_response}",
                                    oninput: move |e: FormEvent| user_response.set(e.value()),
                                }
                            }
                        }
                        CardFooter { style: "gap: 0.5rem;",
                            if revealed() {
                                Button {
                                    disabled: submitting(),
                                    onclick: move |_| on_grade(true),
                                    "I was right"
                                }
                                Button {
                                    variant: ButtonVariant::Destructive,
                                    disabled: submitting(),
                                    onclick: move |_| on_grade(false),
                                    "I got it wrong"
                                }
                            } else {
                                Button {
                                    onclick: move |_| revealed.set(true),
                                    "Show answer"
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}
