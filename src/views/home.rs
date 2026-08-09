use dioxus::prelude::*;
use crate::routes::Route;

#[component]
pub fn Home() -> Element {
    let nav = use_navigator();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; align-items: center; gap: 2rem; padding-top: 3rem;",

            div {
                style: "display: flex; gap: 1.5rem; align-self: flex-end; margin-right: 2rem;",

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

            // TODO: re-enable the interactive Calendar once the upstream
            // dioxus-primitives context panic on click is fixed (commit bf007c1).
            div {
                style: "padding: 2rem; border: 1px dashed #ccc; border-radius: 8px; color: #888;",
                "Calendar coming soon"
            }
        }
    }
}