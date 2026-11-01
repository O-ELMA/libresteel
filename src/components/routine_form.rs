use dioxus::prelude::*;

use crate::{
    components::shell::Route,
    components::ui::{Button, Chip, TextField},
    db,
    i18n::use_i18n,
};

const LEVELS: [&str; 3] = ["beginner", "intermediate", "advanced"];
const GOALS: [&str; 4] = ["cutting", "maintaining", "bulking", "building_strength"];
const SPLITS: [(&str, &str); 2] = [
    ("numbered", "routines.numbered"),
    ("weekday", "routines.weekday"),
];

#[component]
pub fn RoutineForm(
    closing: bool,
    routine: Option<db::Routine>,
    onclose: EventHandler<()>,
    onsave: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let existing = routine.clone();
    let mut name = use_signal(|| {
        existing
            .as_ref()
            .map_or(String::new(), |routine| routine.name.clone())
    });
    let level = use_signal(|| existing.as_ref().and_then(|routine| routine.level.clone()));
    let goal = use_signal(|| existing.as_ref().and_then(|routine| routine.goal.clone()));
    let mut block = use_signal(|| existing.as_ref().map_or(4, |routine| routine.block_weeks));
    let mut block_on = use_signal(|| {
        existing
            .as_ref()
            .is_none_or(|routine| routine.block_enabled)
    });
    let mut block_help = use_signal(|| false);
    let mut split = use_signal(|| {
        existing
            .as_ref()
            .map_or_else(|| "numbered".to_string(), |routine| routine.split.clone())
    });
    let weekdays = use_signal(|| vec![0i64, 1, 2, 3, 4, 5, 6]);
    let mut days = use_signal(|| 3i64);
    let creating = routine.is_none();
    let can_save = !name.read().trim().is_empty()
        && (!creating || split.cloned() != "weekday" || !weekdays.cloned().is_empty());

    let save = move |_| {
        let trimmed = name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let split_value = split.cloned();
        let enabled = block_on.cloned();
        let mut created = None;
        let result = routine.as_ref().map_or_else(
            || {
                let seeds = seeds_for(&split_value, days.cloned(), &weekdays.cloned());
                db::create_routine(
                    &trimmed,
                    level.cloned().as_deref(),
                    goal.cloned().as_deref(),
                    &split_value,
                    block.cloned(),
                    enabled,
                    &seeds,
                )
                .and_then(|id| {
                    db::select_routine(id)?;
                    created = Some(id);
                    Ok(())
                })
            },
            |routine| {
                db::update_routine(
                    routine.id,
                    &trimmed,
                    level.cloned().as_deref(),
                    goal.cloned().as_deref(),
                    &split_value,
                    block.cloned(),
                    enabled,
                )
                .and_then(|()| {
                    if routine.split == split_value {
                        Ok(())
                    } else {
                        db::renumber_days(routine.id, &routine.split, &split_value)
                    }
                })
            },
        );
        match result {
            Ok(()) => {
                onsave.call(());
                if created.is_some() {
                    navigator.push(Route::Workout {});
                } else {
                    onclose.call(());
                }
            }
            Err(err) => dioxus::logger::tracing::error!("save routine failed: {err:#}"),
        }
    };
    let state = if closing { " is-closing" } else { "" };

    rsx! {
        div { class: "fixed inset-0 z-40 flex items-end justify-center",
            div {
                class: "absolute inset-0 bg-black/60 sheet-backdrop{state}",
                onclick: move |_| onclose.call(()),
            }
            div { class: "relative max-h-[92dvh] w-full max-w-md overflow-y-auto rounded-t-3xl bg-surface p-5 pb-[calc(env(safe-area-inset-bottom)+1.25rem)] sheet-panel{state}",
                TextField {
                    label: i18n.t("routines.name"),
                    placeholder: i18n.t("routines.namePlaceholder"),
                    value: name.cloned(),
                    oninput: move |value| name.set(value),
                    if name.read().trim().is_empty() {
                        p { class: "mt-1.5 text-[13px] text-muted", "{i18n.t(\"routines.nameRequired\")}" }
                    }
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.level\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for key in LEVELS {
                        Chip {
                            label: i18n.value("exercises.levels", key),
                            selected: level.cloned().as_deref() == Some(key),
                            onclick: move |_| toggle_option(level, key),
                        }
                    }
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.goal\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for key in GOALS {
                        Chip {
                            label: i18n.value("routines.goals", key),
                            selected: goal.cloned().as_deref() == Some(key),
                            onclick: move |_| toggle_option(goal, key),
                        }
                    }
                }
                div { class: "mt-4 flex items-center gap-2",
                    span { class: "text-sm text-muted", "{i18n.t(\"routines.mesocycle\")}" }
                    button {
                        class: "grid h-6 w-6 place-items-center rounded-full bg-tile text-[13px] font-semibold text-muted",
                        "aria-label": i18n.t("routines.mesocycleHelp"),
                        onclick: move |_| block_help.set(!block_help()),
                        "?"
                    }
                }
                if block_help() {
                    p { class: "mt-1.5 text-[13px] leading-6 text-muted",
                        "{i18n.t(\"routines.mesocycleHelp\")}"
                    }
                }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for (enabled, label) in [(true, i18n.t("routines.mesocycleOn")), (false, i18n.t("routines.mesocycleOff"))] {
                        Chip {
                            label,
                            selected: block_on.cloned() == enabled,
                            onclick: move |_| block_on.set(enabled),
                        }
                    }
                }
                if block_on() {
                    div { class: "mt-2 flex flex-wrap gap-2",
                        for weeks in [4i64, 5, 6] {
                            Chip {
                                label: i18n.tn("routines.mesocycleWeeks", weeks),
                                selected: block.cloned() == weeks,
                                onclick: move |_| block.set(weeks),
                            }
                        }
                    }
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.split\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for (key, label) in SPLITS {
                        Chip {
                            label: i18n.t(label),
                            selected: split.cloned() == key,
                            onclick: move |_| split.set(key.to_string()),
                        }
                    }
                }
                if creating {
                    if split.cloned() == "weekday" {
                        div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.trainingDays\")}" }
                        div { class: "mt-2 grid grid-cols-7 gap-1.5",
                            for (index, label) in i18n.weekdays_short().iter().enumerate() {
                                button {
                                    key: "{index}",
                                    class: if weekdays.read().contains(&i64::try_from(index).unwrap_or(i64::MAX)) {
                                        "h-11 rounded-xl bg-lilac text-[13px] font-medium text-black"
                                    } else {
                                        "h-11 rounded-xl border border-line text-[13px] font-medium text-lead"
                                    },
                                    onclick: {
                                        let value = i64::try_from(index).unwrap_or(i64::MAX);
                                        move |_| toggle_weekday(weekdays, value)
                                    },
                                    "{label}"
                                }
                            }
                        }
                    } else {
                        div { class: "mt-4 flex items-center justify-between",
                            span { class: "text-sm text-muted", "{i18n.t(\"routines.days\")}" }
                            div { class: "flex items-center gap-3",
                                button {
                                    class: "grid size-10 place-items-center rounded-xl bg-tile text-xl",
                                    onclick: move |_| days.set(days.cloned().saturating_sub(1).max(1)),
                                    "−"
                                }
                                span { class: "w-6 text-center text-base font-semibold", "{days}" }
                                button {
                                    class: "grid size-10 place-items-center rounded-xl bg-tile text-xl",
                                    onclick: move |_| days.set(days.cloned().saturating_add(1).min(14)),
                                    "+"
                                }
                            }
                        }
                    }
                }
                Button {
                    disabled: !can_save,
                    class: "mt-6",
                    onclick: save,
                    "{i18n.t(\"common.actions.save\")}"
                }
            }
        }
    }
}

fn seeds_for(split: &str, days: i64, weekdays: &[i64]) -> Vec<db::DaySeed> {
    if split == "weekday" {
        db::WEEKDAY_NAMES
            .iter()
            .enumerate()
            .filter_map(|(index, name)| {
                let weekday = i64::try_from(index).ok()?;
                weekdays.contains(&weekday).then(|| db::DaySeed {
                    name: (*name).to_string(),
                    weekday: Some(weekday),
                })
            })
            .collect()
    } else {
        (0..days)
            .map(|index| db::DaySeed {
                name: format!("Day {}", index.saturating_add(1)),
                weekday: None,
            })
            .collect()
    }
}

fn toggle_weekday(mut weekdays: Signal<Vec<i64>>, value: i64) {
    weekdays.with_mut(|list| {
        if let Some(position) = list.iter().position(|item| *item == value) {
            list.remove(position);
        } else {
            list.push(value);
            list.sort_unstable();
        }
    });
}

fn toggle_option(mut signal: Signal<Option<String>>, key: &str) {
    let next = if signal.cloned().as_deref() == Some(key) {
        None
    } else {
        Some(key.to_string())
    };
    signal.set(next);
}
