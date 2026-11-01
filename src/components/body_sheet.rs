use chrono::{Datelike, Local, NaiveDate, TimeZone};
use dioxus::prelude::*;

use crate::{
    components::ui::{
        Button, ButtonSize, Field, parse_weight, quiet, trim, use_exit, weight_field, weight_unit,
        Sheet,
    },
    db,
    i18n::{use_i18n, I18n},
    pages::calendar::long_date,
};

#[derive(Clone, PartialEq)]
struct Form {
    gender: Option<String>,
    dob: String,
    height: String,
    weight: String,
    fat: String,
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
                    Field {
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
                    Field {
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
                    Field {
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
                    Button {
                        size: ButtonSize::Wide,
                        class: "mt-4",
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
            Button {
                size: ButtonSize::Wide,
                class: "mt-4",
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
    NaiveDate::parse_from_str(raw, "%Y-%m-%d")
        .map_or_else(|_| "--".to_string(), |date| long_date(i18n, date))
}
