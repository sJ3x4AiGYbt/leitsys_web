use dioxus::prelude::*;
use crate::routes::Route;

/// Sticky, translucent top bar shared by every logged-in page.
#[component]
pub fn AppShell(children: Element) -> Element {
    rsx! {
        header { class: "app-nav",
            nav { class: "app-nav-inner",
                Link { class: "app-brand", to: Route::Home {}, "leitsys" }
                div { class: "app-links",
                    Link { class: "app-link", active_class: "active", to: Route::Home {}, "Home" }
                    Link { class: "app-link", active_class: "active", to: Route::Review {}, "Review" }
                    Link { class: "app-link", active_class: "active", to: Route::Questions {}, "Questions" }
                    Link { class: "app-link", active_class: "active", to: Route::Categories {}, "Categories" }
                    Link { class: "app-link", active_class: "active", to: Route::Steps {}, "Steps" }
                    Link { class: "app-link", active_class: "active", to: Route::Quiz {}, "Quiz" }
                }
                div { class: "app-actions",
                    Link { class: "icon-button", active_class: "active", to: Route::Contact {}, title: "Contact",
                        svg {
                            width: "18",
                            height: "18",
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
                    Link { class: "icon-button", active_class: "active", to: Route::Settings {}, title: "Settings",
                        svg {
                            width: "18",
                            height: "18",
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
                }
            }
        }
        main { {children} }
    }
}
