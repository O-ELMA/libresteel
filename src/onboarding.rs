use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{
        LdCake, LdCheck, LdCircleUser, LdRuler, LdScale, LdTimer,
    },
    Icon,
};

use crate::{
    body::{self, DobSheet},
    db,
    i18n::use_i18n,
    ui::{parse_weight, trim, use_exit, weight_field, weight_unit, BackIcon, ForwardIcon, Sheet},
};

const STEPS: usize = 6;
const RESTS: [i64; 6] = [30, 45, 60, 90, 120, 180];
const EYEBROW: &str = "text-[11px] font-semibold tracking-[0.22em] text-muted uppercase";
const TITLE: &str = "mt-3 text-[34px] font-bold leading-[1.1] tracking-tight";
const BODY: &str = "mt-3 text-[15px] leading-6 text-muted";

#[component]
pub fn Onboarding(ondone: EventHandler<()>) -> Element {
    let i18n = use_i18n();
    let mut language = i18n.language;
    let mut step = use_signal(|| 0usize);
    let mut name = use_signal(String::new);
    let mut unit = use_signal(db::unit_lb);
    let gender = use_signal(db::gender);
    let mut dob = use_signal(db::dob);
    let height = use_signal(|| body_latest("height"));
    let weight = use_signal(|| body_latest("weight"));
    let mut week_start = use_signal(db::first_week_day);
    let mut rest = use_signal(db::default_rest);
    let mut dob_open = use_signal(|| false);
    let (dob_closing, close_dob) = use_exit(dob_open, false, |value| *value);

    let advance = EventHandler::new(move |()| {
        let current = step.cloned();
        if current == 1 {
            save_name(&name.cloned());
        }
        if current == 3 {
            save_body(weight.cloned(), height.cloned());
        }
        step.set(current.saturating_add(1).min(STEPS - 1));
    });
    let back = EventHandler::new(move |()| step.set(step.cloned().saturating_sub(1)));
    let finish = EventHandler::new(move |()| {
        save_name(&name.cloned());
        if step.cloned() == 3 {
            save_body(weight.cloned(), height.cloned());
        }
        if let Err(err) = db::finish_onboarding() {
            dioxus::logger::tracing::error!("finish onboarding failed: {err:#}");
        }
        ondone.call(());
    });

    let current = step.cloned();
    let number = current.saturating_add(1);
    let label = i18n.ta(
        "onboarding.step",
        &[("current", &number.to_string()), ("total", &STEPS.to_string())],
    );
    let ready_title = if name.read().trim().is_empty() {
        i18n.t("onboarding.ready.title")
    } else {
        i18n.ta("onboarding.ready.titleNamed", &[("name", name.read().trim())])
    };
    let samples = [
        i18n.t("onboarding.units.kgSample"),
        i18n.t("onboarding.units.lbSample"),
    ];
    let week_label = |value: u32| {
        i18n.t(if value == 6 {
            "settings.sunday"
        } else {
            "settings.monday"
        })
    };

    rsx! {
        div {
            class: "fixed inset-0 z-30 overflow-y-auto bg-surface text-lead",
            div { class: "mx-auto flex min-h-dvh w-full max-w-md flex-col px-5 pt-[calc(env(safe-area-inset-top)+1.25rem)] pb-[calc(env(safe-area-inset-bottom)+1rem)]",
                header { class: "flex items-center gap-1.5",
                    div { class: "flex flex-1 gap-1.5",
                        for index in 0..STEPS {
                            div {
                                key: "{index}",
                                class: if index <= current {
                                    "h-1 flex-1 rounded-full bg-lilac"
                                } else {
                                    "h-1 flex-1 rounded-full bg-line"
                                },
                            }
                        }
                    }
                    if number < STEPS {
                        button {
                            class: "ms-3 h-10 px-1 text-sm text-muted",
                            onclick: move |_| finish.call(()),
                            "{i18n.t(\"onboarding.skip\")}"
                        }
                    }
                }

                main { class: "flex flex-1 flex-col justify-center py-10",
                    div { key: "{current}", class: "reveal-enter",
                        if current == 0 {
                            Heading {
                                label,
                                title: i18n.t("onboarding.language.title"),
                                body: i18n.t("onboarding.language.body"),
                            }
                            div { class: "mt-8 overflow-hidden rounded-2xl bg-tile",
                                for (code, title) in [("en", "English"), ("ar", "العربية"), ("fr", "Français")] {
                                    button {
                                        key: "{code}",
                                        class: "flex h-16 w-full items-center justify-between border-b border-line px-4 text-start last:border-0",
                                        onclick: move |_| {
                                            if let Err(err) = db::set_language(code) {
                                                dioxus::logger::tracing::error!(
                                                    "set language failed: {err:#}"
                                                );
                                            } else {
                                                language.set(code.to_string());
                                            }
                                        },
                                        span {
                                            class: if i18n.lang() == code {
                                                "text-[17px] font-semibold text-accent-ink"
                                            } else {
                                                "text-[17px] font-semibold"
                                            },
                                            "{title}"
                                        }
                                        if i18n.lang() == code {
                                            Icon {
                                                icon: LdCheck,
                                                width: 20,
                                                height: 20,
                                                class: "text-accent-ink",
                                            }
                                        }
                                    }
                                }
                            }
                        } else if current == 1 {
                            Heading {
                                label,
                                title: i18n.t("onboarding.name.title"),
                                body: i18n.t("onboarding.name.body"),
                            }
                            div { class: "mt-8 flex h-16 items-center gap-3 rounded-2xl bg-tile px-5",
                                Icon {
                                    icon: LdCircleUser,
                                    width: 22,
                                    height: 22,
                                    class: "shrink-0 text-muted",
                                }
                                input {
                                    class: "h-full min-w-0 flex-1 bg-transparent text-[17px] text-lead outline-none placeholder:text-muted",
                                    r#type: "text",
                                    autocomplete: "off",
                                    placeholder: "{i18n.t(\"onboarding.name.placeholder\")}",
                                    value: "{name}",
                                    oninput: move |event| name.set(event.value()),
                                    onchange: move |event| save_name(&event.value()),
                                }
                            }
                        } else if current == 2 {
                            Heading {
                                label,
                                title: i18n.t("onboarding.units.title"),
                                body: i18n.t("onboarding.units.body"),
                            }
                            div { class: "mt-8 flex gap-3",
                                for (index, (pounds, code)) in [(false, "kg"), (true, "lb")].into_iter().enumerate() {
                                    button {
                                        key: "{code}",
                                        class: if unit.cloned() == pounds {
                                            "relative h-40 flex-1 rounded-2xl border-2 border-mango bg-tile p-4 text-start"
                                        } else {
                                            "relative h-40 flex-1 rounded-2xl border-2 border-transparent bg-tile p-4 text-start"
                                        },
                                        onclick: move |_| {
                                            if let Err(err) = db::set_unit(pounds) {
                                                dioxus::logger::tracing::error!(
                                                    "set unit failed: {err:#}"
                                                );
                                            } else {
                                                unit.set(pounds);
                                            }
                                        },
                                        if unit.cloned() == pounds {
                                            span { class: "absolute top-3 end-3 grid size-5 place-items-center rounded-full bg-mango text-black",
                                                Icon { icon: LdCheck, width: 13, height: 13 }
                                            }
                                        }
                                        span { class: "block text-[40px] font-bold leading-none", "{code}" }
                                        span { class: "mt-2 block text-[13px] leading-5 text-muted",
                                            "{samples[index]}"
                                        }
                                    }
                                }
                            }
                        } else if current == 3 {
                            Heading {
                                label,
                                title: i18n.t("onboarding.body.title"),
                                body: i18n.t("onboarding.body.body"),
                            }
                            StepBody {
                                gender,
                                dob,
                                height,
                                weight,
                                ondob: move |()| dob_open.set(true),
                            }
                        } else if current == 4 {
                            Heading {
                                label,
                                title: i18n.t("onboarding.week.title"),
                                body: i18n.t("onboarding.week.body"),
                            }
                            div { class: "mt-8 flex rounded-2xl bg-tile p-1",
                                for value in [0u32, 6] {
                                    button {
                                        key: "{value}",
                                        class: if week_start.cloned() == value {
                                            "h-12 flex-1 rounded-xl bg-lilac text-sm font-semibold text-black"
                                        } else {
                                            "h-12 flex-1 rounded-xl text-sm font-medium text-muted"
                                        },
                                        onclick: move |_| {
                                            if let Err(err) = db::set_first_week_day(value) {
                                                dioxus::logger::tracing::error!(
                                                    "set week start failed: {err:#}"
                                                );
                                            } else {
                                                week_start.set(value);
                                            }
                                        },
                                        "{week_label(value)}"
                                    }
                                }
                            }
                            h2 { class: "mt-8 text-sm font-semibold text-muted",
                                "{i18n.t(\"settings.defaultRest\")}"
                            }
                            div { class: "mt-2 grid grid-cols-3 gap-2",
                                for seconds in RESTS {
                                    button {
                                        key: "{seconds}",
                                        class: if rest.cloned() == seconds {
                                            "h-12 rounded-xl bg-sky text-sm font-semibold text-black"
                                        } else {
                                            "h-12 rounded-xl bg-tile text-sm font-medium text-muted"
                                        },
                                        onclick: move |_| {
                                            if let Err(err) = db::set_default_rest(seconds) {
                                                dioxus::logger::tracing::error!(
                                                    "set rest failed: {err:#}"
                                                );
                                            } else {
                                                rest.set(seconds);
                                            }
                                        },
                                        "{seconds} s"
                                    }
                                }
                            }
                            p { class: "mt-2 text-sm leading-6 text-muted",
                                "{i18n.t(\"settings.restNote\")}"
                            }
                        } else {
                            Heading {
                                label,
                                title: ready_title,
                                body: i18n.t("onboarding.ready.body"),
                            }
                            div { class: "mt-6 rounded-2xl bg-tile px-4",
                                SummaryRow {
                                    label: i18n.t("onboarding.ready.language"),
                                    value: i18n.t(match i18n.lang().as_str() {
                                        "ar" => "onboarding.ready.arabic",
                                        "fr" => "onboarding.ready.french",
                                        _ => "onboarding.ready.english",
                                    }),
                                }
                                SummaryRow {
                                    label: i18n.t("onboarding.ready.units"),
                                    value: i18n.t(if unit.cloned() {
                                        "settings.pounds"
                                    } else {
                                        "settings.kilograms"
                                    }),
                                }
                                SummaryRow {
                                    label: i18n.t("onboarding.ready.week"),
                                    value: week_label(week_start.cloned()),
                                }
                                SummaryRow {
                                    label: i18n.t("onboarding.ready.rest"),
                                    value: format!("{} s", rest.cloned()),
                                }
                            }
                        }
                    }
                }

                footer { class: "sticky bottom-0 flex items-center gap-3 bg-surface pt-2",
                    if current > 0 {
                        button {
                            class: "grid size-14 shrink-0 place-items-center rounded-full bg-tile text-lead",
                            "aria-label": i18n.t("onboarding.back"),
                            onclick: move |_| back.call(()),
                            BackIcon { size: 22 }
                        }
                    }
                    button {
                        class: "h-14 flex-1 rounded-full bg-lilac text-[15px] font-semibold tracking-[0.08em] text-black uppercase",
                        onclick: move |_| {
                            if number == STEPS {
                                finish.call(());
                            } else {
                                advance.call(());
                            }
                        },
                        if number == STEPS {
                            "{i18n.t(\"onboarding.start\")}"
                        } else {
                            "{i18n.t(\"onboarding.next\")}"
                        }
                    }
                }
            }
        }
        if dob_open() {
            Sheet {
                closing: dob_closing(),
                onclose: close_dob,
                DobSheet {
                    initial: body::dob_text(dob.cloned()),
                    onsave: move |value: String| {
                        let at = body::dob_timestamp(&value);
                        if let Err(err) = db::set_dob(at) {
                            dioxus::logger::tracing::error!("set birth date failed: {err:#}");
                        }
                        dob.set(at);
                        close_dob.call(());
                    },
                }
            }
        }
    }
}

#[component]
fn Heading(label: String, title: String, body: String) -> Element {
    rsx! {
        p { class: EYEBROW, "{label}" }
        h1 { class: TITLE, "{title}" }
        p { class: BODY, "{body}" }
    }
}

#[component]
fn SummaryRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex h-14 items-center justify-between border-b border-line last:border-0",
            span { class: "text-[15px] text-muted", "{label}" }
            span { class: "text-[15px] font-semibold", "{value}" }
        }
    }
}

#[component]
fn StepBody(
    mut gender: Signal<Option<String>>,
    dob: Signal<Option<i64>>,
    mut height: Signal<Option<f64>>,
    mut weight: Signal<Option<f64>>,
    ondob: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let lb = db::unit_lb();
    let mut height_text = use_signal(|| height.cloned().map_or_else(String::new, trim));
    let mut weight_text = use_signal(|| weight_field(weight.cloned(), lb));
    let age = body::age(dob.cloned());
    let selected = gender.cloned();

    rsx! {
        div { class: "mt-6 overflow-hidden rounded-2xl bg-tile",
            div { class: "flex h-16 items-center gap-3 border-b border-line px-4",
                Icon { icon: LdCircleUser, width: 20, height: 20, class: "shrink-0 text-muted" }
                span { class: "text-[15px] font-medium", "{i18n.t(\"onboarding.body.sex\")}" }
                div { class: "ms-auto flex rounded-full bg-surface p-1",
                    for (value, label) in [("male", i18n.t("settings.male")), ("female", i18n.t("settings.female"))] {
                        button {
                            key: "{value}",
                            class: if selected.as_deref() == Some(value) {
                                "h-9 rounded-full bg-lime px-4 text-[13px] font-semibold text-black"
                            } else {
                                "h-9 rounded-full px-4 text-[13px] font-medium text-muted"
                            },
                            onclick: move |_| {
                                if let Err(err) = db::set_gender(value) {
                                    dioxus::logger::tracing::error!(
                                        "set gender failed: {err:#}"
                                    );
                                } else {
                                    gender.set(Some(value.to_string()));
                                }
                            },
                            "{label}"
                        }
                    }
                }
            }
            button {
                class: "flex h-16 w-full items-center gap-3 border-b border-line px-4 text-start",
                onclick: move |_| ondob.call(()),
                Icon { icon: LdCake, width: 20, height: 20, class: "shrink-0 text-muted" }
                span { class: "text-[15px] font-medium", "{i18n.t(\"body.age\")}" }
                span { class: "ms-auto text-[15px] font-semibold",
                    "{age.map_or_else(|| \"--\".to_string(), |years| years.to_string())}"
                }
                ForwardIcon { size: 16 }
            }
            div { class: "flex h-16 items-center gap-3 border-b border-line px-4",
                Icon { icon: LdRuler, width: 20, height: 20, class: "shrink-0 text-muted" }
                span { class: "text-[15px] font-medium", "{i18n.t(\"body.height\")}" }
                input {
                    class: "ms-auto w-20 bg-transparent text-end text-[15px] font-semibold text-lead outline-none",
                    r#type: "number",
                    inputmode: "decimal",
                    step: "1",
                    value: "{height_text}",
                    oninput: move |event| {
                        let raw = event.value();
                        height_text.set(raw.clone());
                        height.set(
                            raw.trim().parse::<f64>().ok().filter(|value| *value > 0.0),
                        );
                    },
                }
                span { class: "w-7 text-[13px] text-muted", "cm" }
            }
            div { class: "flex h-16 items-center gap-3 px-4",
                Icon { icon: LdScale, width: 20, height: 20, class: "shrink-0 text-muted" }
                span { class: "text-[15px] font-medium", "{i18n.t(\"body.weight\")}" }
                input {
                    class: "ms-auto w-20 bg-transparent text-end text-[15px] font-semibold text-lead outline-none",
                    r#type: "number",
                    inputmode: "decimal",
                    step: "0.1",
                    value: "{weight_text}",
                    oninput: move |event| {
                        let raw = event.value();
                        weight_text.set(raw.clone());
                        weight.set(parse_weight(&raw, lb).filter(|value| *value > 0.0));
                    },
                }
                span { class: "w-7 text-[13px] text-muted", "{weight_unit(lb)}" }
            }
        }
        p { class: "mt-3 flex items-center gap-2 text-sm text-muted",
            Icon { icon: LdTimer, width: 16, height: 16 }
            "{i18n.t(\"onboarding.body.note\")}"
        }
    }
}

fn body_latest(field: &str) -> Option<f64> {
    db::body_latest(field).ok().flatten().map(|(_, value)| value)
}

fn save_name(value: &str) {
    if let Err(err) = db::set_display_name(value) {
        dioxus::logger::tracing::error!("set display name failed: {err:#}");
    }
}

fn save_body(weight: Option<f64>, height: Option<f64>) {
    if weight.is_none() && height.is_none() {
        return;
    }
    if let Err(err) = db::save_measurement(weight, None, height) {
        dioxus::logger::tracing::error!("save body stats failed: {err:#}");
    }
}
