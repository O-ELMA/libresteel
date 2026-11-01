use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCopy, LdPlus},
    Icon,
};

use crate::{
    components::routine_form::RoutineForm,
    components::ui::{use_exit, BackIcon, IconButton, RowActions},
    db,
    i18n::{use_i18n, I18n},
};

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
                                IconButton {
                                    label: i18n.t("routines.clone"),
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
