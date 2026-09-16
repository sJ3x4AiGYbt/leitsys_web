use dioxus::prelude::*;
use time::Date;
use crate::api::{self, decode_claims};
use crate::auth::use_auth;
use crate::calendar::{self, DayCell};
use crate::routes::Route;

const WEEKDAY_LABELS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let nav = use_navigator();

    let questions = use_resource(move || async move {
        let token = auth.token()?;
        let claims = decode_claims(&token)?;
        api::get_my_questions(claims.user_id, &token).await.ok()
    });

    let today = calendar::today();
    let mut displayed = use_signal(|| (today.year(), today.month()));
    let (year, month) = displayed();

    let due_dates: Vec<Date> = match &*questions.read() {
        Some(Some(qs)) => qs.iter().filter_map(|q| calendar::parse_date(&q.next_review_date)).collect(),
        _ => Vec::new(),
    };

    let late_count = due_dates.iter().filter(|d| **d < today).count();
    let today_count = due_dates.iter().filter(|d| **d == today).count();
    let month_count = due_dates.iter().filter(|d| d.year() == year && d.month() == month).count();
    let active_count = due_dates.len();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 2rem; padding: 2rem; max-width: 640px; margin: 0 auto;",

            div {
                style: "display: flex; gap: 1.5rem; align-self: flex-end;",

                // Settings (icon person)
                button {
                    style: "background: none; border: none; cursor: pointer;",
                    onclick: move |_| { nav.push(Route::Settings {}); },
                    svg {
                        width: "28",
                        height: "28",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        circle { cx: "12", cy: "8", r: "4" }
                        path { d: "M4 20c0-4 3.5-7 8-7s8 3 8 7" }
                    }
                }

                // Contact (icon envelope)
                button {
                    style: "background: none; border: none; cursor: pointer;",
                    onclick: move |_| { nav.push(Route::Contact {}); },
                    svg {
                        width: "28",
                        height: "28",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        stroke_width: "2",
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        rect { x: "3", y: "5", width: "18", height: "14", rx: "2" }
                        path { d: "M3 7l9 6 9-6" }
                    }
                }
            }

            div {
                style: "display: flex; gap: 1rem; flex-wrap: wrap;",
                StatTile { label: "Late", value: late_count }
                StatTile { label: "Due today", value: today_count }
                StatTile { label: "This month", value: month_count }
                StatTile { label: "Active", value: active_count }
            }

            div {
                style: "display: flex; align-items: center; justify-content: center; gap: 1.5rem;",
                button {
                    style: "background: none; border: none; cursor: pointer; font-size: 1.25rem;",
                    onclick: move |_| { let (y, m) = displayed(); displayed.set(calendar::previous_month(y, m)); },
                    "‹"
                }
                h2 { style: "margin: 0; min-width: 12rem; text-align: center;", "{month} {year}" }
                button {
                    style: "background: none; border: none; cursor: pointer; font-size: 1.25rem;",
                    onclick: move |_| { let (y, m) = displayed(); displayed.set(calendar::next_month(y, m)); },
                    "›"
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.25rem; font-size: 0.75rem; color: #888; text-align: center;",
                for label in WEEKDAY_LABELS {
                    div { "{label}" }
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(7, 1fr); gap: 0.25rem;",
                for cell in calendar::month_grid(year, month) {
                    DayTile { cell, count: due_dates.iter().filter(|d| **d == cell.date).count(), is_today: cell.date == today }
                }
            }
        }
    }
}

#[component]
fn StatTile(label: &'static str, value: usize) -> Element {
    rsx! {
        div {
            style: "flex: 1; min-width: 6rem; border: 1px solid #ddd; border-radius: 8px; padding: 0.75rem; text-align: center;",
            div { style: "font-size: 1.5rem; font-weight: bold;", "{value}" }
            div { style: "font-size: 0.75rem; color: #888;", "{label}" }
        }
    }
}

#[component]
fn DayTile(cell: DayCell, count: usize, is_today: bool) -> Element {
    let border = if is_today { "var(--secondary-color-5)" } else { "#ddd" };
    let opacity = if cell.in_month { "1" } else { "0.35" };

    rsx! {
        div {
            style: "border: 1px solid {border}; border-radius: 6px; padding: 0.4rem; min-height: 3.25rem; display: flex; flex-direction: column; justify-content: space-between; opacity: {opacity};",
            span { style: if is_today { "font-weight: bold;" } else { "" }, "{cell.date.day()}" }
            if count > 0 {
                span { style: "font-size: 0.75rem; color: #888; align-self: flex-end;", "{count}" }
            }
        }
    }
}
