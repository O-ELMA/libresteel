use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCopy, LdPlus},
    Icon,
};

use crate::{
    db,
    i18n::{use_i18n, I18n},
    shell::Route,
    ui::{use_exit, BackIcon, RowActions},
};

const LEVELS: [&str; 3] = ["beginner", "intermediate", "advanced"];
const GOALS: [&str; 4] = ["cutting", "maintaining", "bulking", "building_strength"];
const SPLITS: [(&str, &str); 2] = [
    ("numbered", "routines.numbered"),
    ("weekday", "routines.weekday"),
];

#[derive(Clone, Copy, PartialEq)]
enum FormMode {
    Create,
    Edit(i64),
}

#[component]
pub fn Routines() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let mut revision = use_signal(|| 0u32);
    let _ = revision.cloned();
    let rows = db::routines().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("routines failed: {err:#}");
        Vec::new()
    });
    let mut form = use_signal(|| rows.is_empty().then_some(FormMode::Create));
    let (form_closing, close_form) = use_exit(form, None, Option::is_some);

    let editing = form.cloned().and_then(|mode| match mode {
        FormMode::Create => None,
        FormMode::Edit(id) => rows.iter().find(|routine| routine.id == id).cloned(),
    });
    let editing_key = match form.cloned() {
        Some(FormMode::Edit(id)) => id,
        _ => 0,
    };

    rsx! {
        div { class: "flex h-[calc(100dvh-6rem)] flex-col overflow-hidden",
            header { class: "flex items-center gap-3 px-5 pt-5",
                button {
                    class: "text-lead",
                    onclick: move |_| navigator.go_back(),
                    BackIcon { size: 24 }
                }
                h1 { class: "text-[22px] font-bold tracking-tight", "{i18n.t(\"routines.title\")}" }
                button {
                    class: "ms-auto flex h-11 items-center gap-1 px-2 text-sm font-semibold text-accent-ink",
                    onclick: move |_| form.set(Some(FormMode::Create)),
                    Icon { icon: LdPlus, width: 18, height: 18 }
                    "{i18n.t(\"routines.create\")}"
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto pb-12",
                if rows.is_empty() {
                    p { class: "px-5 pt-10 text-center text-base text-muted",
                        "{i18n.t(\"routines.empty\")}"
                    }
                }
                div { class: "mt-5 flex flex-col gap-4 px-5",
                    for routine in rows.iter() {
                    div {
                        key: "{routine.id}",
                        class: "rounded-2xl bg-linear-to-br from-accent/25 to-tile p-4",
                        div { class: "flex items-start justify-between gap-3",
                            if routine.is_active {
                                span { class: "rounded-md bg-accent px-2.5 py-1 text-[11px] font-bold tracking-wider",
                                    "{i18n.t(\"routines.current\")}"
                                }
                            } else {
                                span { class: "h-6" }
                            }
                            div { class: "flex items-start",
                                button {
                                    class: "grid size-10 place-items-center text-muted",
                                    "aria-label": i18n.t("routines.clone"),
                                    onclick: {
                                        let id = routine.id;
                                        let name = i18n.ta("routines.copyName", &[("name", &routine.name)]);
                                        move |_| match db::clone_routine(id, &name) {
                                            Ok(_) => *revision.write() += 1,
                                            Err(err) => {
                                                dioxus::logger::tracing::error!("clone routine {id} failed: {err:#}");
                                            }
                                        }
                                    },
                                    Icon { icon: LdCopy, width: 18, height: 18 }
                                }
                                RowActions {
                                    vertical: true,
                                    onedit: {
                                        let id = routine.id;
                                        move |()| form.set(Some(FormMode::Edit(id)))
                                    },
                                    ondelete: {
                                        let id = routine.id;
                                        move |()| match db::delete_routine(id) {
                                            Ok(()) => *revision.write() += 1,
                                            Err(err) => {
                                                dioxus::logger::tracing::error!("delete routine {id} failed: {err:#}");
                                            }
                                        }
                                    },
                                }
                            }
                        }
                        h2 { class: "mt-10 text-xl font-bold leading-tight", "{routine.name}" }
                        p { class: "mt-1 text-[12px] font-semibold tracking-wide text-lead/60",
                            "{meta(&i18n, routine)}"
                        }
                        if !routine.is_active {
                            div { class: "mt-4 flex justify-end",
                                button {
                                    class: "h-10 rounded-xl bg-white/10 px-5 text-sm font-semibold",
                                    onclick: {
                                        let id = routine.id;
                                        move |_| match db::select_routine(id) {
                                            Ok(()) => {
                                                *revision.write() += 1;
                                            }
                                            Err(err) => {
                                                dioxus::logger::tracing::error!("select routine {id} failed: {err:#}");
                                            }
                                        }
                                    },
                                    "{i18n.t(\"routines.select\")}"
                                }
                            }
                        }
                    }
                }
            }
        }
        }
        if form.cloned().is_some() {
            RoutineForm {
                key: "{editing_key}",
                closing: form_closing(),
                routine: editing,
                onclose: close_form,
                onsave: move |()| {
                    *revision.write() += 1;
                },
            }
        }
    }
}

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
                label { class: "block text-sm text-muted", "{i18n.t(\"routines.name\")}" }
                input {
                    class: "mt-2 h-12 w-full rounded-2xl bg-tile px-4 text-base text-lead outline-none",
                    placeholder: i18n.t("routines.namePlaceholder"),
                    value: "{name}",
                    oninput: move |event| name.set(event.value()),
                }
                if name.read().trim().is_empty() {
                    p { class: "mt-1.5 text-[13px] text-muted", "{i18n.t(\"routines.nameRequired\")}" }
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.level\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for key in LEVELS {
                        OptionChip {
                            label: i18n.value("exercises.levels", key),
                            selected: level.cloned().as_deref() == Some(key),
                            onclick: move |_| toggle_option(level, key),
                        }
                    }
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"routines.goal\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for key in GOALS {
                        OptionChip {
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
                        OptionChip {
                            label,
                            selected: block_on.cloned() == enabled,
                            onclick: move |_| block_on.set(enabled),
                        }
                    }
                }
                if block_on() {
                    div { class: "mt-2 flex flex-wrap gap-2",
                        for weeks in [4i64, 5, 6] {
                            OptionChip {
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
                        OptionChip {
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
                button {
                    class: if can_save {
                        "mt-6 h-12 w-full rounded-2xl bg-lilac text-base font-semibold text-black"
                    } else {
                        "mt-6 h-12 w-full rounded-2xl bg-tile text-base font-semibold text-muted"
                    },
                    disabled: !can_save,
                    onclick: save,
                    "{i18n.t(\"common.actions.save\")}"
                }
            }
        }
    }
}

fn meta(i18n: &I18n, routine: &db::Routine) -> String {
    let mut parts = Vec::new();
    if let Some(level) = &routine.level {
        parts.push(i18n.value("exercises.levels", level).to_uppercase());
    }
    if let Some(goal) = &routine.goal {
        parts.push(i18n.value("routines.goals", goal).to_uppercase());
    }
    parts.push(i18n.tn("routines.dayCount", routine.day_count));
    parts.join(" • ")
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

#[component]
fn OptionChip(label: String, selected: bool, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: if selected {
                "h-10 rounded-xl bg-lilac px-4 text-sm font-medium text-black"
            } else {
                "h-10 rounded-xl border border-line px-4 text-sm font-medium text-lead"
            },
            onclick,
            "{label}"
        }
    }
}
