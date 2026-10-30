use chrono::{Datelike, Local, NaiveDate, TimeZone};
use dioxus::prelude::*;
use dioxus_free_icons::{icons::ld_icons::LdPencil, Icon};

use crate::{
    db,
    i18n::{use_i18n, I18n},
    overview::{quiet, OverviewTabs},
    shell::Route,
    ui::{
        display_weight, parse_weight, trim, use_exit, weight_field, weight_text, weight_unit,
        ForwardIcon, Sheet,
    },
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

const BODY_FRONT: &str = include_str!("../assets/muscles/body_front.svg");

#[derive(Clone, PartialEq)]
struct Form {
    gender: Option<String>,
    dob: String,
    height: String,
    weight: String,
    fat: String,
}

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
            div { class: "mt-5 rounded-2xl bg-tile p-4",
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
                                      input_value(key, value, lb)
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
                    button {
                        class: "mt-4 h-12 w-full rounded-xl bg-lilac text-sm font-semibold text-black",
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
pub fn BodyFormSheet(
    closing: bool,
    onclose: EventHandler<()>,
    onchange: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let lb = db::unit_lb();
    let weight = quiet(db::body_latest("weight"));
    let fat = quiet(db::body_latest("fat"));
    let height = quiet(db::body_latest("height"));
    let gender = db::gender();
    let dob = db::dob();
    let mut form = use_signal(|| {
        Some(Form {
            gender,
            dob: dob_text(dob),
            height: height.map_or_else(String::new, |(_, value)| trim(value)),
            weight: weight_field(weight.map(|(_, value)| value), lb),
            fat: fat.map_or_else(String::new, |(_, value)| trim(value)),
        })
    });
    let mut dob_picker = use_signal(|| false);
    let (dob_closing, close_dob) = use_exit(dob_picker, false, |value| *value);

    rsx! {
        if let Some(state) = form.cloned() {
            Sheet {
                closing: closing,
                onclose: onclose,
                div { class: "px-3 pb-2",
                    h2 { class: "text-lg font-semibold", "{i18n.t(\"body.stats\")}" }
                    div { class: "mt-4 flex rounded-xl bg-tile p-1",
                        for (value, label) in [("male", i18n.t("settings.male")), ("female", i18n.t("settings.female"))] {
                            button {
                                key: "{value}",
                                class: if state.gender.as_deref() == Some(value) {
                                    "h-11 flex-1 rounded-lg bg-accent text-sm font-semibold text-black"
                                } else {
                                    "h-11 flex-1 rounded-lg text-sm font-medium text-muted"
                                },
                                onclick: move |_| {
                                    form.with_mut(|state| {
                                        if let Some(state) = state {
                                            state.gender = Some(value.to_string());
                                        }
                                    });
                                },
                                "{label}"
                            }
                        }
                    }
                    button {
                        class: "mt-2 flex h-12 w-full items-center justify-between rounded-xl bg-tile px-4",
                        onclick: move |_| dob_picker.set(true),
                        span { class: "text-sm font-medium", "{i18n.t(\"settings.birthDate\")}" }
                        span { class: "text-sm text-muted", "{dob_label(&i18n, &state.dob)}" }
                    }
                    FormField {
                        label: i18n.t("body.height"),
                        unit: "cm",
                        value: state.height,
                        oninput: move |value| {
                            form.with_mut(|state| {
                                if let Some(state) = state {
                                    state.height = value;
                                }
                            });
                        },
                    }
                    FormField {
                        label: i18n.t("body.weight"),
                        unit: weight_unit(lb),
                        value: state.weight,
                        oninput: move |value| {
                            form.with_mut(|state| {
                                if let Some(state) = state {
                                    state.weight = value;
                                }
                            });
                        },
                    }
                    FormField {
                        label: i18n.t("body.bodyFat"),
                        unit: "%",
                        value: state.fat,
                        oninput: move |value| {
                            form.with_mut(|state| {
                                if let Some(state) = state {
                                    state.fat = value;
                                }
                            });
                        },
                    }
                    button {
                        class: "mt-4 h-12 w-full rounded-xl bg-lilac text-sm font-semibold text-black",
                        onclick: move |_| {
                            let Some(state) = form.cloned() else {
                                return;
                            };
                            if let Some(gender) = state.gender.as_deref() {
                                if let Err(err) = db::set_gender(gender) {
                                    dioxus::logger::tracing::error!("set gender failed: {err:#}");
                                }
                            }
                            let dob = dob_timestamp(&state.dob);
                            if let Err(err) = db::set_dob(dob) {
                                dioxus::logger::tracing::error!("set birth date failed: {err:#}");
                            }
                            let height = state
                                .height
                                .trim()
                                .parse::<f64>()
                                .ok()
                                .filter(|value| *value > 0.0);
                            let weight = parse_weight(&state.weight, lb)
                                .filter(|value| *value > 0.0);
                            let fat = state
                                .fat
                                .trim()
                                .parse::<f64>()
                                .ok()
                                .filter(|value| *value > 0.0 && *value <= 100.0);
                            match db::save_measurement(weight, fat, height) {
                                Ok(()) => {
                                    onchange.call(());
                                    onclose.call(());
                                }
                                Err(err) => {
                                    dioxus::logger::tracing::error!(
                                        "save body stats failed: {err:#}"
                                    );
                                }
                            }
                        },
                        "{i18n.t(\"common.actions.save\")}"
                    }
                }
            }
        }
        if let Some(state) = form.cloned() {
            if dob_picker() {
                Sheet {
                    closing: dob_closing(),
                    onclose: close_dob,
                    DobSheet {
                        initial: state.dob,
                        onsave: move |value: String| {
                            form.with_mut(|state| {
                                if let Some(state) = state {
                                    state.dob = value;
                                }
                            });
                            close_dob.call(());
                        },
                    }
                }
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

#[component]
fn FormField(
    label: String,
    unit: &'static str,
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

pub fn dob_text(at: Option<i64>) -> String {
    at.and_then(|at| Local.timestamp_opt(at, 0).single())
        .map_or_else(String::new, |moment| moment.format("%Y-%m-%d").to_string())
}

pub fn dob_timestamp(raw: &str) -> Option<i64> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .ok()
        .and_then(|day| day.and_hms_opt(0, 0, 0))
        .and_then(|naive| Local.from_local_datetime(&naive).earliest())
        .map(|moment| moment.timestamp())
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

fn input_value(field: &str, value: f64, lb: bool) -> String {
    if field == "weight" {
        weight_field(Some(value), lb)
    } else {
        trim(value)
    }
}

fn label_key(field: &str) -> &'static str {
    FIELDS
        .iter()
        .find(|(key, _, _, _)| *key == field)
        .map_or("body.stats", |(_, label, _, _)| *label)
}

#[component]
pub fn DobSheet(initial: String, onsave: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let today = Local::now().date_naive();
    let fallback = NaiveDate::from_ymd_opt(1990, 1, 1).unwrap_or(today);
    let start = NaiveDate::parse_from_str(&initial, "%Y-%m-%d")
        .ok()
        .filter(|date| date.year() >= 1900 && *date <= today)
        .unwrap_or(fallback);
    let day_index = start.day().saturating_sub(1);
    let month_index = start.month().saturating_sub(1);
    let year_index = u32::try_from(start.year().saturating_sub(1900)).unwrap_or(0);

    let days = (1..=31).map(|day| day.to_string()).collect::<Vec<_>>();
    let months = (1..=12)
        .map(|month| i18n.month_long(month))
        .collect::<Vec<_>>();
    let years = (1900..=today.year())
        .map(|year| year.to_string())
        .collect::<Vec<_>>();

    use_effect(move || {
        let _ = document::eval(&format!(
            "const set = (id, row) => {{ const el = document.getElementById(id); if (el) el.scrollTop = row * 40; }}; set('dob-day', {day_index}); set('dob-month', {month_index}); set('dob-year', {year_index});"
        ));
    });

    rsx! {
        div { class: "px-3 pb-2",
            h2 { class: "text-lg font-semibold", "{i18n.t(\"settings.birthDate\")}" }
            div { class: "relative mt-3",
                div { class: "pointer-events-none absolute inset-x-0 top-20 h-10 rounded-lg bg-white/5" }
                div { class: "relative grid grid-cols-3 gap-2",
                    Wheel { id: "dob-day".to_string(), items: days }
                    Wheel { id: "dob-month".to_string(), items: months }
                    Wheel { id: "dob-year".to_string(), items: years }
                }
            }
            button {
                class: "mt-4 h-12 w-full rounded-xl bg-lilac text-sm font-semibold text-black",
                onclick: move |_| read_wheels(onsave),
                "{i18n.t(\"common.actions.save\")}"
            }
        }
    }
}

#[component]
fn Wheel(id: String, items: Vec<String>) -> Element {
    rsx! {
        div {
            id: "{id}",
            class: "dob-wheel h-[200px] snap-y snap-mandatory overflow-y-auto overscroll-contain",
            style: "scroll-snap-type: y mandatory; scrollbar-width: none; -webkit-mask-image: linear-gradient(to bottom, transparent, black 25%, black 75%, transparent); mask-image: linear-gradient(to bottom, transparent, black 25%, black 75%, transparent);",
            div { class: "h-20 shrink-0" }
            for (index, item) in items.iter().enumerate() {
                div {
                    key: "{index}",
                    class: "flex h-10 snap-center items-center justify-center text-[17px]",
                    "{item}"
                }
            }
            div { class: "h-20 shrink-0" }
        }
    }
}

fn read_wheels(onsave: EventHandler<String>) {
    spawn(async move {
        let mut eval = document::eval(
            "const read = (id) => { const el = document.getElementById(id); return el ? Math.round(el.scrollTop / 40) : -1; }; dioxus.send([read('dob-day'), read('dob-month'), read('dob-year')].join(','));",
        );
        let Ok(raw) = eval.recv::<String>().await else {
            return;
        };
        let mut parts = raw.split(',').map(|part| part.trim().parse::<i32>().ok());
        let (Some(Some(day)), Some(Some(month)), Some(Some(year))) =
            (parts.next(), parts.next(), parts.next())
        else {
            return;
        };
        if day < 0 || month < 0 || year < 0 {
            return;
        }
        let year = year.saturating_add(1900);
        let month = u32::try_from(month.saturating_add(1))
            .unwrap_or(1)
            .clamp(1, 12);
        let day = u32::try_from(day.saturating_add(1)).unwrap_or(1);
        let day = day.clamp(1, days_in_month(year, month));
        if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
            onsave.call(date.format("%Y-%m-%d").to_string());
        }
    });
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year.saturating_add(1), 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month.saturating_add(1), 1)
    };
    next.and_then(|date| date.pred_opt())
        .map_or(31, |date| date.day())
}

fn dob_label(i18n: &I18n, raw: &str) -> String {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_or_else(
        |_| "--".to_string(),
        |date| {
            format!(
                "{} {}, {}",
                i18n.month_short(date.month()),
                date.day(),
                date.year()
            )
        },
    )
}
