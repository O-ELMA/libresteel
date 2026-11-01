use dioxus::prelude::*;

#[component]
pub fn Field(
    label: String,
    #[props(default)] unit: &'static str,
    value: String,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "mt-2 flex h-12 items-center justify-between rounded-xl bg-tile px-4",
            span { class: "text-sm font-medium", "{label}" }
            div { class: "flex items-center gap-2",
                input {
                    class: "w-24 bg-transparent text-end text-sm text-lead outline-none",
                    r#type: "number",
                    inputmode: "decimal",
                    step: "0.01",
                    value: "{value}",
                    oninput: move |event| oninput.call(event.value()),
                }
                span { class: "w-8 text-sm text-muted", "{unit}" }
            }
        }
    }
}

#[component]
pub fn TextField(
    label: String,
    value: String,
    #[props(default)] placeholder: String,
    #[props(default)] label_class: &'static str,
    oninput: EventHandler<String>,
    children: Element,
) -> Element {
    rsx! {
        label { class: "block text-sm text-muted {label_class}", "{label}" }
        input {
            class: "mt-2 h-12 w-full rounded-2xl bg-tile px-4 text-base text-lead outline-none",
            placeholder: "{placeholder}",
            value: "{value}",
            oninput: move |event| oninput.call(event.value()),
        }
        {children}
    }
}
