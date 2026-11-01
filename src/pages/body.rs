use chrono::{Datelike, Local, TimeZone};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdPencil, Icon};

use crate::{
    components::body_sheet::BodyFormSheet,
    components::shell::Route,
    components::ui::{
        display_weight, parse_weight, quiet, trim, use_exit, weight_text, weight_unit, Button,
        ButtonSize, Card, ForwardIcon, Sheet,
    },
    db,
    i18n::use_i18n,
    pages::overview::OverviewTabs,
};

const FIELDS: [(&str, &str, bool, &str); 9] = [
    ("neck", "body.neck", false, "12%"),
    ("shoulders", "body.shoulders", true, "19%"),
    ("chest", "body.chest", false, "25%"),
    ("arms", "body.arms", true, "31%"),
    ("waist", "body.waist", false, "40%"),
    ("forearms", "body.forearms", true, "43%"),
    ("hips", "body.hips", true, "53%"),
    ("upper_leg", "body.upperLeg", false, "60%"),
    ("lower_leg", "body.lowerLeg", false, "85%"),
];

const BODY_FRONT: &str = include_str!("../../assets/muscles/body_front.svg");

#[component]
pub fn Body() -> Element {
    let i18n = use_i18n();
    let lb = db::unit_lb();
    let mut revision = use_signal(|| 0u32);
    let _ = revision.cloned();
    let mut open = use_signal(|| None::<&'static str>);
    let mut text = use_signal(String::new);
    let (closing, close) = use_exit(open, None, Option::is_some);
    let mut form_open = use_signal(|| false);
    let (form_closing, close_form) = use_exit(form_open, false, |value| *value);

    let rows = FIELDS
        .iter()
        .map(|(key, label, right, top)| (*key, *label, *right, *top, quiet(db::body_latest(key))))
        .collect::<Vec<_>>();
    let weight = quiet(db::body_latest("weight"));
    let fat = quiet(db::body_latest("fat"));
    let height = quiet(db::body_latest("height"));
    let gender = db::gender();
    let gender_label = gender_text(&i18n, gender.as_deref());
    let dob = db::dob();
    let age_label = age(dob).map_or_else(|| "--".to_string(), |years| years.to_string());

    rsx! {
        div { class: "px-5",
            OverviewTabs {}
            Card { class: "mt-5 p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"body.stats\")}" }
                p { class: "mt-1 text-sm text-muted", "{i18n.t(\"body.tapToEdit\")}" }
                div { class: "relative mt-2 h-[28rem] w-full",
                    div {
                        class: "absolute inset-y-0 left-[54%] -translate-x-1/2 [&_svg]:h-full [&_svg]:w-auto",
                        dangerous_inner_html: front(),
                    }
                    for (key, label, right, top, latest) in rows.iter() {
                        button {
                            key: "{key}",
                            class: if *right {
                                "absolute end-0 w-[18%] -translate-y-1/2 text-start"
                            } else {
                                "absolute start-0 w-[18%] -translate-y-1/2 text-end"
                            },
                            style: "top: {top}",
                            onclick: {
                                let key = *key;
                                let latest = *latest;
                                move |_| {
                                    text.set(latest.map_or_else(String::new, |(_, value)| {
                                      number(key, value, lb)
                                    }));
                                    open.set(Some(key));
                                }
                            },
                            div { class: "text-xs text-muted", "{i18n.t(label)}" }
                            div {
                                class: if latest.is_some() {
                                    "text-[15px] font-semibold"
                                } else {
                                    "text-[15px] text-muted"
                                },
                                "{latest.map_or_else(|| \"--\".to_string(), |(_, value)| number(key, value, lb))}"
                                span { class: "text-xs text-muted", " {unit(key, lb)}" }
                            }
                        }
                    }
                }
            }
            button {
                class: "relative mt-3 block w-full rounded-2xl bg-tile p-4 text-start",
                onclick: move |_| form_open.set(true),
                Icon {
                    icon: LdPencil,
                    width: 18,
                    height: 18,
                    class: "absolute top-3 end-3 text-muted",
                }
                div { class: "grid grid-cols-2 gap-3 pe-6",
                    Composition {
                        label: i18n.t("settings.gender"),
                        value: gender_label,
                    }
                    Composition { label: i18n.t("body.age"), value: age_label }
                }
                div { class: "mt-4 grid grid-cols-2 gap-3",
                    Composition {
                        label: i18n.t("body.height"),
                        value: height.map_or_else(|| "-- cm".to_string(), |(_, value)| {
                            format!("{} cm", trim(value))
                        }),
                    }
                    Composition {
                        label: i18n.t("body.weight"),
                        value: weight.map_or_else(
                            || "--".to_string(),
                            |(_, value)| weight_text(value, lb),
                        ),
                    }
                }
                div { class: "mt-4 grid grid-cols-3 gap-3",
                    Composition {
                        label: i18n.t("body.bodyFat"),
                        value: fat.map_or_else(
                            || "--".to_string(),
                            |(_, value)| format!("{}%", trim(value)),
                        ),
                    }
                    Composition { label: i18n.t("body.bmi"), value: bmi(weight, height) }
                    Composition {
                        label: i18n.t("body.leanMass"),
                        value: lean_mass(weight, fat, lb),
                    }
                }
            }
            Link {
                to: Route::BodyEnergy {},
                class: "mt-3 flex items-center justify-between rounded-2xl bg-tile p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"energy.title\")}" }
                ForwardIcon { size: 18 }
            }
            div { class: "h-8" }
        }
        if let Some(key) = open.cloned() {
            Sheet {
                closing: closing(),
                onclose: close,
                div { class: "px-3 pb-2",
                    h2 { class: "text-lg font-semibold", "{i18n.t(label_key(key))}" }
                    div { class: "mt-4 flex items-center gap-3 rounded-xl bg-tile px-4",
                        input {
                            class: "h-14 w-full bg-transparent text-[20px] font-semibold text-lead outline-none",
                            r#type: "number",
                            inputmode: "decimal",
                            step: "0.01",
                            value: "{text}",
                            oninput: move |event| text.set(event.value()),
                        }
                        span { class: "text-sm text-muted", "{unit(key, lb)}" }
                    }
                    Button {
                        size: ButtonSize::Wide,
                        class: "mt-4",
                        onclick: move |_| {
                            let raw = text.cloned();
                            let parsed = if key == "weight" {
                                parse_weight(&raw, lb)
                            } else {
                                raw.trim().parse::<f64>().ok()
                            };
                            let Some(value) = parsed
                                .filter(|value| *value > 0.0 && (key != "fat" || *value <= 100.0))
                            else {
                                return;
                            };
                            match db::save_body(key, value) {
                                Ok(()) => {
                                    revision.set(revision.cloned().wrapping_add(1));
                                    close.call(());
                                }
                                Err(err) => {
                                    dioxus::logger::tracing::error!("save body failed: {err:#}");
                                }
                            }
                        },
                        "{i18n.t(\"common.actions.save\")}"
                    }
                }
            }
        }
        if form_open() {
            BodyFormSheet {
                closing: form_closing(),
                onclose: close_form,
                onchange: move |()| revision.set(revision.cloned().wrapping_add(1)),
            }
        }
    }
}

#[component]
fn Composition(label: String, value: String) -> Element {
    rsx! {
        div { class: "min-w-0",
            div { class: "text-[21px] font-bold leading-7 text-accent-ink", "{value}" }
            div { class: "mt-1 text-sm text-muted", "{label}" }
        }
    }
}

fn gender_text(i18n: &crate::i18n::I18n, gender: Option<&str>) -> String {
    match gender {
        Some("male") => i18n.t("settings.male"),
        Some("female") => i18n.t("settings.female"),
        _ => "--".to_string(),
    }
}

pub fn age(dob: Option<i64>) -> Option<i64> {
    let day = Local.timestamp_opt(dob?, 0).single()?.date_naive();
    let today = Local::now().date_naive();
    let years = today
        .year()
        .saturating_sub(day.year())
        .saturating_sub(i32::from(
            (today.month(), today.day()) < (day.month(), day.day()),
        ));
    (years >= 0).then(|| i64::from(years))
}

fn front() -> String {
    BODY_FRONT.replace("<title>Body, front</title>", "")
}

fn bmi(weight: Option<(i64, f64)>, height: Option<(i64, f64)>) -> String {
    let (Some((_, weight)), Some((_, height))) = (weight, height) else {
        return "--".to_string();
    };
    if height <= 0.0 {
        return "--".to_string();
    }
    let meters = height / 100.0;
    format!("{:.2}", weight / (meters * meters))
}

fn lean_mass(weight: Option<(i64, f64)>, fat: Option<(i64, f64)>, lb: bool) -> String {
    let (Some((_, weight)), Some((_, fat))) = (weight, fat) else {
        return "--".to_string();
    };
    let value = display_weight(weight * (1.0 - fat / 100.0), lb);
    format!("{} {}", trim(value), weight_unit(lb))
}

fn number(field: &str, value: f64, lb: bool) -> String {
    if field == "weight" {
        trim(display_weight(value, lb))
    } else {
        trim(value)
    }
}

fn unit(field: &str, lb: bool) -> &'static str {
    match field {
        "weight" => weight_unit(lb),
        "fat" => "%",
        _ => "cm",
    }
}

fn label_key(field: &str) -> &'static str {
    FIELDS
        .iter()
        .find(|(key, _, _, _)| *key == field)
        .map_or("body.stats", |(_, label, _, _)| *label)
}
