use dioxus::prelude::*;

#[component]
pub fn Chip(
    label: String,
    selected: bool,
    #[props(default)] class: String,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: if selected {
                "h-10 rounded-xl bg-lilac px-4 text-sm font-medium text-black {class}"
            } else {
                "h-10 rounded-xl border border-line px-4 text-sm font-medium text-lead {class}"
            },
            onclick,
            "{label}"
        }
    }
}
