use dioxus::prelude::*;

#[component]
pub fn Card(#[props(default)] class: String, children: Element) -> Element {
    rsx! {
        div { class: "rounded-2xl bg-tile {class}",
            {children}
        }
    }
}
