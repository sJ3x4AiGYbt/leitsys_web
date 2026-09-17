use dioxus::prelude::*;

const CONTACT_EMAIL: &str = "leomehdibelarbi@gmail.com";

#[component]
pub fn Contact() -> Element {
    rsx! {
        div {
            style: "max-width: 480px; margin: 3rem auto; display: flex; flex-direction: column; gap: 1rem;",
            h1 { "Contact" }
            p { style: "color: #888;", "Found a bug, or have a suggestion for leitsys? Reach out directly." }
            a {
                href: "mailto:{CONTACT_EMAIL}",
                style: "font-weight: 600;",
                "{CONTACT_EMAIL}"
            }
        }
    }
}
