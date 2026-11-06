use dioxus::prelude::*;

const CONTACT_EMAIL: &str = "sj3x4aigybt.deuce273@passinbox.com";

#[component]
pub fn Contact() -> Element {
    rsx! {
        div {
            class: "page",
            h1 { "Contact" }
            p { class: "muted", "Found a bug, or have a suggestion for leitsys? Reach out directly." }
            a {
                href: "mailto:{CONTACT_EMAIL}",
                style: "font-weight: 600;",
                "{CONTACT_EMAIL}"
            }
        }
    }
}
