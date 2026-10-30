use dioxus::prelude::*;

use crate::{
    body::{age, BodyFormSheet},
    calendar::volume_text,
    db,
    i18n::{use_i18n, I18n},
    overview::quiet,
    ui::{
        display_weight, stored_weight, trim, use_exit, weight_unit, BackIcon, ForwardIcon, Sheet,
        LB_PER_KG,
    },
};

const ACTIVITIES: [(&str, f64); 5] = [
    ("sedentary", 1.2),
    ("light", 1.375),
    ("moderate", 1.55),
    ("very", 1.725),
    ("extra", 1.9),
];

const PERIODS: [(&str, f64); 4] = [
    ("daily", 1.0),
    ("weekly", 7.0),
    ("monthly", 30.42),
    ("yearly", 365.0),
];

const KCAL_PER_KG: f64 = 3500.0 * LB_PER_KG;

const FORMULAS: [&str; 3] = ["mifflin", "katch", "cunningham"];

#[component]
pub fn BodyEnergy() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let lb = db::unit_lb();
    let mut revision = use_signal(|| 0u32);
    let _ = revision.cloned();
    let mut form_open = use_signal(|| false);
    let (form_closing, close_form) = use_exit(form_open, false, |value| *value);
    let gender = db::gender();
    let age = age(db::dob());
    let weight = quiet(db::body_latest("weight"));
    let height = quiet(db::body_latest("height"));
    let fat = quiet(db::body_latest("fat"));

    let mut activity = use_signal(|| 2usize);
    let mut formula = use_signal(|| 0usize);
    let mut direction = use_signal(|| 0i8);
    let mut mode = use_signal(|| 0i8);
    let mut target_in = use_signal(String::new);
    let mut rates = use_signal(|| std::array::from_fn::<String, 4, _>(|_| "0".to_string()));
    let mut picking = use_signal(|| false);
    let (picking_closing, close_picking) = use_exit(picking, false, |value| *value);
    let mut picking_formula = use_signal(|| false);
    let (formula_closing, close_formula) = use_exit(picking_formula, false, |value| *value);

    let mut missing = Vec::new();
    if formula() == 0 {
        if !matches!(gender.as_deref(), Some("male" | "female")) {
            missing.push(i18n.t("settings.gender"));
        }
        if age.is_none() {
            missing.push(i18n.t("body.age"));
        }
        if height.is_none() {
            missing.push(i18n.t("body.height"));
        }
    }
    if weight.is_none() {
        missing.push(i18n.t("body.weight"));
    }
    if formula() != 0 && fat.is_none() {
        missing.push(i18n.t("body.bodyFat"));
    }

    let estimate = {
        let factor = ACTIVITIES
            .get(activity())
            .map_or(1.55, |(_, factor)| *factor);
        let yearly = rates
            .cloned()
            .get(3)
            .and_then(|value| value.trim().parse::<f64>().ok())
            .unwrap_or(0.0)
            .max(0.0);
        let deficit = stored_weight(yearly / 365.0, lb) * KCAL_PER_KG;
        let signed = match direction() {
            -1 => -deficit,
            1 => deficit,
            _ => 0.0,
        };
        let bmr = match formula() {
            0 => match (gender.as_deref(), age, weight, height) {
                (Some("male"), Some(age), Some((_, weight)), Some((_, height))) => {
                    Some(mifflin(weight, height, age, true))
                }
                (Some("female"), Some(age), Some((_, weight)), Some((_, height))) => {
                    Some(mifflin(weight, height, age, false))
                }
                _ => None,
            },
            index => match (weight, fat) {
                (Some((_, weight)), Some((_, fat))) => {
                    let lean = weight * (1.0 - fat / 100.0);
                    Some(if index == 1 {
                        21.6f64.mul_add(lean, 370.0)
                    } else {
                        22.0f64.mul_add(lean, 500.0)
                    })
                }
                _ => None,
            },
        };
        bmr.map(|bmr| {
            let tdee = bmr * factor;
            let target = (tdee + signed).max(0.0);
            let entered = target_in
                .cloned()
                .trim()
                .parse::<f64>()
                .ok()
                .unwrap_or(target);
            (bmr, tdee, target, (entered - tdee) / KCAL_PER_KG)
        })
    };

    rsx! {
        header { class: "flex items-center gap-3 px-5 pt-5",
            button {
                class: "text-lead",
                "aria-label": i18n.t("common.labels.back"),
                onclick: move |_| navigator.go_back(),
                BackIcon { size: 24 }
            }
            h1 { class: "flex-1 text-center text-[20px] font-bold tracking-tight",
                "{i18n.t(\"energy.title\")}"
            }
            span { class: "size-6" }
        }
        div { class: "px-5",
            div { class: "mt-5 rounded-2xl bg-tile p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"energy.formula\")}" }
                button {
                    class: "mt-3 flex h-12 w-full items-center justify-between rounded-xl bg-surface px-4 text-start",
                    onclick: move |_| picking_formula.set(true),
                    span { class: "min-w-0 flex-1 truncate text-sm font-medium",
                        "{formula_label(&i18n, FORMULAS.get(formula()).copied().unwrap_or(\"mifflin\"))}"
                    }
                    ForwardIcon { size: 18 }
                }
            }
            if let Some((bmr, tdee, target, daily_kg)) = estimate {
                div { class: "mt-3 rounded-2xl bg-tile p-4",
                    h2 { class: "text-base font-semibold", "{i18n.t(\"energy.activity\")}" }
                    button {
                        class: "mt-3 flex min-h-12 w-full items-center justify-between gap-3 rounded-xl bg-surface px-4 py-2 text-start",
                        onclick: move |_| picking.set(true),
                        span { class: "min-w-0 flex-1 text-sm font-medium",
                            "{level_label(&i18n, ACTIVITIES.get(activity()).map_or(\"moderate\", |(key, _)| *key))}"
                        }
                        ForwardIcon { size: 18 }
                    }
                }
                div { class: "mt-3 rounded-2xl bg-tile p-4",
                    div { class: "flex rounded-xl bg-surface p-1",
                        for (value, key) in [(0i8, "weight"), (1, "calories")] {
                            button {
                                key: "{key}",
                                class: if mode() == value {
                                    "h-10 flex-1 rounded-lg bg-accent text-sm font-semibold text-black"
                                } else {
                                    "h-10 flex-1 rounded-lg text-sm font-medium text-muted"
                                },
                                onclick: move |_| {
                                    if value == 1 {
                                        target_in.set(format!("{target:.0}"));
                                    }
                                    mode.set(value);
                                },
                                "{mode_label(&i18n, key)}"
                            }
                        }
                    }
                    if mode() == 0 {
                        h2 { class: "mt-4 text-base font-semibold", "{i18n.t(\"energy.rate\")}" }
                        div { class: "mt-3 flex rounded-xl bg-surface p-1",
                            for (value, key) in [(-1i8, "lose"), (0, "maintain"), (1, "gain")] {
                                button {
                                    key: "{key}",
                                    class: if direction() == value {
                                        "h-10 flex-1 rounded-lg bg-accent text-sm font-semibold text-black"
                                    } else {
                                        "h-10 flex-1 rounded-lg text-sm font-medium text-muted"
                                    },
                                    onclick: move |_| direction.set(value),
                                    "{period_label(&i18n, key)}"
                                }
                            }
                        }
                        div { class: "mt-3 grid grid-cols-2 gap-3",
                            for (index, (key, _)) in PERIODS.iter().enumerate() {
                                RateField {
                                    key: "{key}",
                                    label: period_label(&i18n, key),
                                    unit: weight_unit(lb).to_string(),
                                    disabled: direction() == 0,
                                    value: rates.cloned().get(index).cloned().unwrap_or_default(),
                                    oninput: move |value: String| {
                                        rates.with_mut(|rates| sync(rates, index, &value));
                                    },
                                }
                            }
                        }
                    } else {
                        div { class: "mt-3",
                            RateField {
                                label: i18n.t("energy.target"),
                                unit: i18n.t("energy.kcal"),
                                disabled: false,
                                value: target_in.cloned(),
                                oninput: move |value: String| target_in.set(value),
                            }
                        }
                    }
                }
                div { class: "mt-3 rounded-2xl bg-tile p-4",
                    div { class: "flex items-center justify-between",
                        div {
                            div { class: "text-sm text-muted", "{i18n.t(\"energy.bmr\")}" }
                            div { class: "mt-1 text-[17px] font-semibold", "{volume_text(bmr)}" }
                        }
                        div { class: "text-end",
                            div { class: "text-sm text-muted", "{i18n.t(\"energy.tdee\")}" }
                            div { class: "mt-1 text-[17px] font-semibold", "{volume_text(tdee)}" }
                        }
                    }
                    if mode() == 0 {
                        div { class: "mt-4 text-sm text-muted", "{i18n.t(\"energy.target\")}" }
                        div { class: "mt-1 flex items-baseline gap-1.5",
                            span { class: "text-[28px] font-bold leading-9", "{volume_text(target)}" }
                            span { class: "text-sm text-muted", "{i18n.t(\"energy.kcal\")}" }
                        }
                    } else {
                        div { class: "mt-4 text-sm text-muted", "{i18n.t(\"energy.weightChange\")}" }
                        div { class: "mt-2 grid grid-cols-2 gap-3",
                            for (key, days) in PERIODS.iter() {
                                div { key: "{key}", class: "rounded-xl bg-surface p-3",
                                    div { class: "text-xs text-muted", "{period_label(&i18n, key)}" }
                                    div { class: "mt-1 text-[17px] font-semibold",
                                        "{signed(display_weight(daily_kg * days, lb))} {weight_unit(lb)}"
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "mt-3 rounded-2xl bg-tile p-4",
                    h2 { class: "text-base font-semibold", "{i18n.t(\"energy.missingTitle\")}" }
                    p { class: "mt-2 text-sm leading-6 text-muted",
                        "{i18n.ta(\"energy.missingBody\", &[(\"fields\", &missing.join(\", \"))])}"
                    }
                    button {
                        class: "mt-4 flex h-12 w-full items-center justify-center rounded-xl bg-lilac text-sm font-semibold text-black",
                        onclick: move |_| form_open.set(true),
                        "{i18n.t(\"energy.fillBody\")}"
                    }
                }
            }
            div { class: "h-8" }
        }
        if form_open() {
            BodyFormSheet {
                closing: form_closing(),
                onclose: close_form,
                onchange: move |()| revision.set(revision.cloned().wrapping_add(1)),
            }
        }
        if picking() {
            Sheet {
                closing: picking_closing(),
                onclose: close_picking,
                h3 { class: "px-3 pb-1 text-base font-semibold", "{i18n.t(\"energy.activity\")}" }
                div { class: "max-h-[65dvh] overflow-y-auto p-1.5",
                    for (index, (key, factor)) in ACTIVITIES.iter().enumerate() {
                        button {
                            key: "{key}",
                            class: if activity() == index {
                                "flex w-full items-center gap-3 rounded-xl bg-tile px-3 py-3 text-start"
                            } else {
                                "flex w-full items-center gap-3 rounded-xl px-3 py-3 text-start"
                            },
                            onclick: move |_| {
                                activity.set(index);
                                close_picking.call(());
                            },
                            div { class: "min-w-0 flex-1",
                                div { class: "text-[15px] font-medium",
                                    "{level_label(&i18n, key)}"
                                }
                            }
                            span { class: "shrink-0 text-sm text-muted", "{factor:.3}" }
                        }
                    }
                }
            }
        }
        if picking_formula() {
            Sheet {
                closing: formula_closing(),
                onclose: close_formula,
                h3 { class: "px-3 pb-1 text-base font-semibold", "{i18n.t(\"energy.formula\")}" }
                div { class: "max-h-[65dvh] overflow-y-auto p-1.5",
                    for (index, key) in FORMULAS.iter().enumerate() {
                        button {
                            key: "{key}",
                            class: if formula() == index {
                                "flex w-full items-center gap-3 rounded-xl bg-tile px-3 py-3 text-start"
                            } else {
                                "flex w-full items-center gap-3 rounded-xl px-3 py-3 text-start"
                            },
                            onclick: move |_| {
                                formula.set(index);
                                close_formula.call(());
                            },
                            div { class: "min-w-0 flex-1",
                                div { class: "text-[15px] font-medium",
                                    "{formula_label(&i18n, key)}"
                                }
                                div { class: "mt-0.5 text-xs text-muted",
                                    "{formula_note(&i18n, key)}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RateField(
    label: String,
    unit: String,
    value: String,
    disabled: bool,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        div {
            class: if disabled {
                "rounded-xl bg-surface p-3 opacity-50"
            } else {
                "rounded-xl bg-surface p-3"
            },
            div { class: "text-xs text-muted", "{label}" }
            div { class: "mt-1 flex items-baseline gap-1",
                input {
                    class: if disabled {
                        "w-full min-w-0 bg-transparent text-[17px] font-semibold text-muted outline-none"
                    } else {
                        "w-full min-w-0 bg-transparent text-[17px] font-semibold text-lead outline-none"
                    },
                    r#type: "number",
                    inputmode: "decimal",
                    step: "0.01",
                    disabled: disabled,
                    value: "{value}",
                    oninput: move |event| oninput.call(event.value()),
                }
                span { class: "shrink-0 text-xs text-muted", "{unit}" }
            }
        }
    }
}

fn formula_label(i18n: &I18n, key: &str) -> String {
    i18n.t(&format!("energy.formulas.{key}"))
}

fn mode_label(i18n: &I18n, key: &str) -> String {
    i18n.t(&format!("energy.modes.{key}"))
}

fn signed(value: f64) -> String {
    if value.abs() < 0.005 {
        "0.00".to_string()
    } else {
        format!("{value:+.2}")
    }
}

fn formula_note(i18n: &I18n, key: &str) -> String {
    i18n.t(&format!("energy.formulaNotes.{key}"))
}

fn level_label(i18n: &I18n, key: &str) -> String {
    i18n.t(&format!("energy.levels.{key}"))
}

fn period_label(i18n: &I18n, key: &str) -> String {
    i18n.t(&format!("energy.{key}"))
}

fn sync(rates: &mut [String; 4], edited: usize, raw: &str) {
    let Some((_, days)) = PERIODS.get(edited) else {
        return;
    };
    let Ok(value) = raw.trim().parse::<f64>() else {
        return;
    };
    if let Some(slot) = rates.get_mut(edited) {
        slot.clone_from(&raw.to_string());
    }
    let daily = value / days;
    for (index, (_, length)) in PERIODS.iter().enumerate() {
        if index != edited {
            if let Some(slot) = rates.get_mut(index) {
                *slot = trim(daily * length);
            }
        }
    }
}

fn mifflin(weight_kg: f64, height_cm: f64, age: i64, male: bool) -> f64 {
    let age = f64::from(i32::try_from(age).unwrap_or(i32::MAX));
    let base = 5.0f64.mul_add(-age, 10.0f64.mul_add(weight_kg, 6.25 * height_cm));
    if male {
        base + 5.0
    } else {
        base - 161.0
    }
}
