use dioxus::prelude::*;

use crate::{
    db,
    i18n::use_i18n,
    shell::Route,
    ui::ForwardIcon,
};

#[component]
pub fn Settings() -> Element {
    let i18n = use_i18n();
    let mut lb = use_signal(db::unit_lb);
    let mut first = use_signal(|| db::first_week_day() == 6);
    let mut rest = use_signal(|| db::default_rest().to_string());
    let current = i18n.lang();

    rsx! {
        h1 { class: "px-5 pt-5 text-[26px] font-bold tracking-tight", "{i18n.t(\"settings.title\")}" }
        div { class: "px-5 pb-8",
            h2 { class: "mt-5 text-sm font-semibold text-muted", "{i18n.t(\"settings.unitSystem\")}" }
            div { class: "mt-2 flex rounded-xl bg-tile p-1",
                for (value, label) in [(false, i18n.t("settings.kilograms")), (true, i18n.t("settings.pounds"))] {
                    button {
                        key: "{label}",
                        class: if lb() == value {
                            "h-11 flex-1 rounded-lg bg-mango text-sm font-semibold text-black"
                        } else {
                            "h-11 flex-1 rounded-lg text-sm font-medium text-muted"
                        },
                        onclick: move |_| match db::set_unit(value) {
                            Ok(()) => lb.set(value),
                            Err(err) => dioxus::logger::tracing::error!("set unit failed: {err:#}"),
                        },
                        "{label}"
                    }
                }
            }
            p { class: "mt-2 text-sm leading-6 text-muted",
                "{i18n.t(\"settings.unitNote\")}"
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"settings.firstWeekDay\")}" }
            div { class: "mt-2 flex rounded-xl bg-tile p-1",
                for (sunday, label) in [(false, i18n.t("settings.monday")), (true, i18n.t("settings.sunday"))] {
                    button {
                        key: "{label}",
                        class: if first() == sunday {
                            "h-11 flex-1 rounded-lg bg-lilac text-sm font-semibold text-black"
                        } else {
                            "h-11 flex-1 rounded-lg text-sm font-medium text-muted"
                        },
                        onclick: move |_| {
                            let value = if sunday { 6 } else { 0 };
                            match db::set_first_week_day(value) {
                                Ok(()) => first.set(sunday),
                                Err(err) => dioxus::logger::tracing::error!(
                                    "set week start failed: {err:#}"
                                ),
                            }
                        },
                        "{label}"
                    }
                }
            }
            p { class: "mt-2 text-sm leading-6 text-muted",
                "{i18n.t(\"settings.weekNote\")}"
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"settings.restTimer\")}" }
            div { class: "mt-2 flex items-center justify-between rounded-xl bg-tile px-4 py-3",
                span { class: "text-sm font-medium", "{i18n.t(\"settings.defaultRest\")}" }
                div { class: "flex items-center gap-2",
                    input {
                        class: "h-10 w-20 rounded-lg bg-surface px-2 text-end text-sm text-lead outline-none",
                        r#type: "number",
                        inputmode: "numeric",
                        value: "{rest}",
                        onchange: move |event| {
                            let raw = event.value();
                            let Ok(seconds) = raw.trim().parse::<i64>() else {
                                return;
                            };
                            match db::set_default_rest(seconds) {
                                Ok(()) => rest.set(seconds.max(0).to_string()),
                                Err(err) => dioxus::logger::tracing::error!(
                                    "set rest failed: {err:#}"
                                ),
                            }
                        },
                    }
                    span { class: "text-sm text-muted", "s" }
                }
            }
            p { class: "mt-2 text-sm leading-6 text-muted",
                "{i18n.t(\"settings.restNote\")}"
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"settings.language\")}" }
            div { class: "mt-2 flex rounded-xl bg-tile p-1",
                for (code, label) in [("en", "English"), ("ar", "العربية"), ("fr", "Français")] {
                    button {
                        key: "{code}",
                        class: if current == code {
                            "h-11 flex-1 rounded-lg bg-lime text-sm font-semibold text-black"
                        } else {
                            "h-11 flex-1 rounded-lg text-sm font-medium text-muted"
                        },
                        onclick: move |_| match db::set_language(code) {
                            Ok(()) => {
                                let mut language = i18n.language;
                                language.set(code.to_string());
                            }
                            Err(err) => dioxus::logger::tracing::error!(
                                "set language failed: {err:#}"
                            ),
                        },
                        "{label}"
                    }
                }
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"data.title\")}" }
            Link {
                to: Route::DataTransfer {},
                class: "mt-2 flex items-center justify-between rounded-2xl bg-tile p-4",
                div { class: "min-w-0",
                    div { class: "text-base font-semibold", "{i18n.t(\"data.rowTitle\")}" }
                    p { class: "mt-1 text-sm leading-6 text-muted", "{i18n.t(\"data.rowNote\")}" }
                }
                ForwardIcon { size: 20 }
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"settings.about\")}" }
            div { class: "mt-2 rounded-2xl bg-tile p-4",
                div { class: "flex items-center justify-between",
                    span { class: "text-base font-semibold", "LibreSteel" }
                    span { class: "text-sm text-muted", "v{env!(\"CARGO_PKG_VERSION\")}" }
                }
                p { class: "mt-2 text-sm leading-6 text-muted",
                    "{i18n.t(\"settings.tagline\")}"
                }
                p { class: "mt-3 text-sm leading-6 text-muted",
                    "{i18n.t(\"settings.credit\")}"
                }
            }
        }
    }
}
