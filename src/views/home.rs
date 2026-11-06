use dioxus::prelude::*;
use time::Date;
use crate::api::{self, decode_claims};
use crate::auth::use_auth;
use crate::calendar::{self, DayCell};
use crate::routes::Route;
use crate::components::button::{Button, ButtonVariant};
use crate::components::card::{Card, CardContent};

const WEEKDAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    let questions = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_questions(claims.user_id, &token, false, false).await.ok()
    });

    let today = calendar::today();
    let mut displayed = use_signal(|| (today.year(), today.month()));
    let (year, month) = displayed();
    let mut selected_day = use_signal(|| None::<Date>);

    let due_dates: Vec<Date> = match &*questions.read() {
        Some(Some(qs)) => qs.iter().filter_map(|q| calendar::parse_date(&q.next_review_date)).collect(),
        _ => Vec::new(),
    };

    let late_count = due_dates.iter().filter(|d| **d < today).count();
    let today_count = due_dates.iter().filter(|d| **d == today).count();
    let month_count = due_dates.iter().filter(|d| d.year() == year && d.month() == month).count();
    let active_count = due_dates.len();

    let day_titles: Vec<String> = match (selected_day(), &*questions.read()) {
        (Some(date), Some(Some(qs))) => qs
            .iter()
            .filter(|q| calendar::parse_date(&q.next_review_date) == Some(date))
            .map(|q| q.title.clone())
            .collect(),
        _ => Vec::new(),
    };

    rsx! {
        div { class: "page",
            h1 { "Today" }

            div { class: "stat-grid",
                StatTile { label: "Late", value: late_count, alert: late_count > 0 }
                StatTile { label: "Due today", value: today_count }
                StatTile { label: "This month", value: month_count }
                StatTile { label: "Active", value: active_count }
            }

            div { class: "home-actions",
                Button {
                    onclick: move |_| { nav.push(Route::Review {}); },
                    "Start review"
                }
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| { nav.push(Route::Quiz {}); },
                    "Multiple-choice quiz"
                }
            }

            Card {
                CardContent { div { class: "stack",
                    div { class: "calendar-head",
                        h2 { "{month} {year}" }
                        div { class: "row", style: "gap: 2px;",
                            button {
                                class: "icon-button",
                                title: "Previous month",
                                onclick: move |_| { let (y, m) = displayed(); displayed.set(calendar::previous_month(y, m)); },
                                "‹"
                            }
                            button {
                                class: "icon-button",
                                title: "Next month",
                                onclick: move |_| { let (y, m) = displayed(); displayed.set(calendar::next_month(y, m)); },
                                "›"
                            }
                        }
                    }

                    div { class: "calendar-grid",
                        for label in WEEKDAY_LABELS {
                            div { class: "calendar-weekday", "{label}" }
                        }
                        for cell in calendar::month_grid(year, month) {
                            DayTile {
                                cell,
                                count: due_dates.iter().filter(|d| **d == cell.date).count(),
                                is_today: cell.date == today,
                                is_selected: selected_day() == Some(cell.date),
                                onclick: move |date| {
                                    selected_day.set(if selected_day() == Some(date) { None } else { Some(date) });
                                },
                            }
                        }
                    }

                    if let Some(date) = selected_day() {
                        div { class: "answer-box stack-sm",
                            div { class: "row",
                                strong { class: "grow", "Due {date}" }
                                button {
                                    class: "icon-button",
                                    title: "Close",
                                    onclick: move |_| selected_day.set(None),
                                    "×"
                                }
                            }
                            if day_titles.is_empty() {
                                p { class: "muted", "Nothing due this day." }
                            } else {
                                for title in day_titles {
                                    p { style: "margin: 0;", "{title}" }
                                }
                            }
                        }
                    }
                } }
            }
        }
    }
}

#[component]
fn StatTile(label: &'static str, value: usize, #[props(default)] alert: bool) -> Element {
    rsx! {
        div { class: "stat", "data-tone": if alert { "danger" } else { "" },
            div { class: "stat-value", "{value}" }
            div { class: "stat-label", "{label}" }
        }
    }
}

#[component]
fn DayTile(cell: DayCell, count: usize, is_today: bool, is_selected: bool, onclick: EventHandler<Date>) -> Element {
    rsx! {
        div {
            class: "day",
            "data-outside": "{!cell.in_month}",
            "data-today": "{is_today}",
            "data-selected": "{is_selected}",
            onclick: move |_| onclick.call(cell.date),
            span { class: "day-number", "{cell.date.day()}" }
            if count > 0 {
                span { class: "day-count", "{count}" }
            }
        }
    }
}
