use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{
        LdArrowLeftRight, LdCheck, LdCopy, LdGripVertical, LdLink, LdLink2, LdPlus, LdRefreshCw,
        LdX,
    },
    Icon,
};

use crate::{
    bodymap::BodyMap,
    db,
    exercises::{
        art_for, equipment_options, matches_filters, summary, ExerciseFilters, ExerciseThumb, Thumb,
    },
    i18n::{use_i18n, I18n, WEEKDAYS},
    routines::RoutineForm,
    session::ActiveSessionPill,
    shell::Route,
    ui::{
        parse_weight, use_exit, weight_field, BackIcon, BodyIcon, DeleteButton, RowActions, Sheet,
    },
};

const EXERCISE_PITCH: f64 = 88.0;
const DAY_PITCH: f64 = 128.0;

const FOCUS_GROUPS: [&str; 10] = [
    "Abs",
    "Upper Legs",
    "Back",
    "Shoulders",
    "Chest",
    "Biceps",
    "Triceps",
    "Forearms",
    "Lower Legs",
    "Glutes",
];

#[derive(Clone, Copy, PartialEq)]
enum Focus {
    Routine,
    Day(i64),
}

#[derive(Clone, PartialEq)]
struct Reorder {
    id: i64,
    from: usize,
    to: usize,
    started: bool,
    start_y: f64,
    delta: f64,
}

#[component]
pub fn Workout() -> Element {
    workout(None)
}

#[component]
pub fn RoutineDay(day_id: i64) -> Element {
    workout(Some(day_id))
}

fn workout(route_day: Option<i64>) -> Element {
    rsx! {
        WorkoutScreen { route_day }
    }
}

#[component]
fn WorkoutScreen(route_day: Option<i64>) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let revision = use_signal(|| 0u32);
    let _ = revision.cloned();
    let mut show_day = use_signal(|| route_day.is_some());
    let mut selected = use_signal(|| route_day);
    let mut edit_routine = use_signal(|| false);
    let mut day_form = use_signal(|| None::<Option<i64>>);
    let mut clone_day = use_signal(|| None::<i64>);
    let mut focus = use_signal(|| None::<Focus>);
    let (edit_closing, close_edit) = use_exit(edit_routine, false, |value| *value);
    let (day_form_closing, close_day_form) = use_exit(day_form, None, Option::is_some);
    let (clone_closing, close_clone) = use_exit(clone_day, None, Option::is_some);
    let (focus_closing, close_focus) = use_exit(focus, None, Option::is_some);

    let routines = db::routines().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("routines failed: {err:#}");
        Vec::new()
    });
    let Some(routine) = routines.iter().find(|routine| routine.is_active).cloned() else {
        let label = if routines.is_empty() {
            i18n.t("workout.createRoutine")
        } else {
            i18n.t("workout.chooseRoutine")
        };
        return rsx! {
            div { class: "px-5 pt-5",
                h1 { class: "text-[26px] font-bold tracking-tight", "{i18n.t(\"workout.title\")}" }
                div { class: "mt-10 rounded-2xl border border-dashed border-line p-6 text-center",
                    p { class: "text-base text-muted", "{i18n.t(\"workout.noActiveRoutine\")}" }
                    Link {
                        to: Route::Routines {},
                        class: "mt-4 inline-flex h-11 items-center gap-1.5 rounded-xl bg-lilac px-5 text-sm font-semibold text-black",
                        Icon { icon: LdPlus, width: 18, height: 18 }
                        "{label}"
                    }
                }
            }
        };
    };

    let active = db::active_session().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("active session failed: {err:#}");
        None
    });

    let days = db::days(routine.id).unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("days failed: {err:#}");
        Vec::new()
    });
    let current_id = selected
        .cloned()
        .filter(|id| days.iter().any(|day| day.id == *id))
        .or_else(|| days.first().map(|day| day.id));
    let current = current_id.and_then(|id| days.iter().find(|day| day.id == id).cloned());
    let current_index = current_id.and_then(|id| days.iter().position(|day| day.id == id));
    let exercises = current
        .as_ref()
        .map(|day| {
            db::day_exercises(day.id).unwrap_or_else(|err| {
                dioxus::logger::tracing::error!("day exercises failed: {err:#}");
                Vec::new()
            })
        })
        .unwrap_or_default();
    let can_start = current_id.is_some() && !exercises.is_empty();
    let editing_day = day_form
        .cloned()
        .flatten()
        .and_then(|id| days.iter().find(|day| day.id == id).cloned());
    let cloning_day = clone_day
        .cloned()
        .and_then(|id| days.iter().find(|day| day.id == id).cloned());
    let focus_scope = focus.cloned();
    let focus_title = focus_scope.map_or_else(String::new, |scope| match scope {
        Focus::Routine => routine.name.clone(),
        Focus::Day(id) => days
            .iter()
            .find(|day| day.id == id)
            .map_or_else(String::new, |day| day.name.clone()),
    });

    rsx! {
        div { class: "flex h-[calc(100dvh-6rem)] flex-col overflow-hidden",
            header { class: "flex items-center gap-3 px-5 pt-5",
                h1 { class: "min-w-0 flex-1 truncate text-[22px] font-bold tracking-tight",
                    "{routine.name}"
                }
                button {
                    class: "grid size-11 shrink-0 place-items-center rounded-full border border-line text-lead",
                    "aria-label": i18n.t("workout.bodyMap"),
                    onclick: move |_| focus.set(Some(Focus::Routine)),
                    BodyIcon { size: 20 }
                }
                Link {
                    to: Route::Routines {},
                    class: "grid size-11 shrink-0 place-items-center rounded-full border border-line text-lead",
                    "aria-label": i18n.t("workout.switchRoutine"),
                    Icon { icon: LdArrowLeftRight, width: 20, height: 20 }
                }
                RowActions {
                    vertical: false,
                    onedit: move |()| edit_routine.set(true),
                    ondelete: move |()| match db::delete_routine(routine.id) {
                        Ok(()) => bump(revision),
                        Err(err) => {
                            dioxus::logger::tracing::error!("delete routine failed: {err:#}");
                        }
                    },
                }
            }
            div { class: "mt-4 flex border-b border-line",
                button {
                    class: if show_day.cloned() {
                        "flex-1 border-b-2 border-transparent pb-3 text-base font-medium text-muted"
                    } else {
                        "flex-1 border-b-2 border-white pb-3 text-base font-semibold text-lead"
                    },
                    onclick: move |_| show_day.set(false),
                    "{i18n.t(\"workout.daysTab\")}"
                }
                button {
                    class: if show_day.cloned() {
                        "flex-1 border-b-2 border-white pb-3 text-base font-semibold text-lead"
                    } else {
                        "flex-1 border-b-2 border-transparent pb-3 text-base font-medium text-muted"
                    },
                    onclick: move |_| show_day.set(true),
                    "{i18n.t(\"workout.exercisesTab\")}"
                }
            }
            if show_day.cloned() {
                div { class: "min-h-0 flex-1 overflow-y-auto px-5 pt-4 pb-24",
                    if let Some(day) = current.as_ref() {
                        div { class: "rounded-2xl border border-line bg-surface p-4",
                            div { class: "flex items-start gap-3",
                                div { class: "min-w-0 flex-1",
                                    span { class: "rounded-md bg-lilac/15 px-2 py-0.5 text-[12px] font-bold text-lilac",
                                        "{day_badge(&i18n, &routine.split, day.weekday, current_index.unwrap_or(0))}"
                                    }
                                    div { class: "mt-1 text-xl font-semibold", "{day.name}" }
                                    div { class: "mt-0.5 text-sm text-muted",
                                        "{day_meta(&i18n, day, false)}"
                                    }
                                }
                                div { class: "flex items-start",
                                    button {
                                        class: "grid size-10 place-items-center text-muted",
                                        "aria-label": i18n.t("workout.bodyMap"),
                                        onclick: {
                                            let id = day.id;
                                            move |_| focus.set(Some(Focus::Day(id)))
                                        },
                                        BodyIcon { size: 18 }
                                    }
                                    button {
                                        class: "grid size-10 place-items-center text-muted",
                                        "aria-label": i18n.t("workout.cloneDay"),
                                        onclick: {
                                            let id = day.id;
                                            move |_| clone_day.set(Some(id))
                                        },
                                        Icon { icon: LdCopy, width: 18, height: 18 }
                                    }
                                    RowActions {
                                        vertical: true,
                                        onedit: {
                                            let id = day.id;
                                            move |()| day_form.set(Some(Some(id)))
                                        },
                                        ondelete: {
                                            let id = day.id;
                                            move |()| match db::delete_day(id) {
                                                Ok(()) => bump(revision),
                                                Err(err) => {
                                                    dioxus::logger::tracing::error!("delete day failed: {err:#}");
                                                }
                                            }
                                        },
                                    }
                                }
                            }
                        }
                        if exercises.is_empty() {
                            p { class: "py-8 text-center text-base text-muted", "{i18n.t(\"workout.noExercises\")}" }
                        }
                        DayExercises {
                            day_id: day.id,
                            exercises,
                            onadd: {
                                let day_id = day.id;
                                move |()| {
                                    navigator.push(Route::RoutineDayAdd { day_id });
                                }
                            },
                            onremove: move |id| match db::remove_exercise(id) {
                                Ok(()) => bump(revision),
                                Err(err) => {
                                    dioxus::logger::tracing::error!("remove exercise failed: {err:#}");
                                }
                            },
                            onchange: move |()| bump(revision),
                        }
                    } else {
                        p { class: "py-10 text-center text-base text-muted", "{i18n.t(\"workout.noDays\")}" }
                    }
                }
            } else {
                div { class: "min-h-0 flex-1 overflow-y-auto px-5 pt-4 pb-24",
                    DayList {
                        routine_id: routine.id,
                        days,
                        split: routine.split.clone(),
                        selected: current_id,
                        onselect: move |id| {
                            selected.set(Some(id));
                            show_day.set(true);
                        },
                        onedit: move |id| day_form.set(Some(Some(id))),
                        ondelete: move |id| match db::delete_day(id) {
                            Ok(()) => bump(revision),
                            Err(err) => {
                                dioxus::logger::tracing::error!("delete day failed: {err:#}");
                            }
                        },
                        onclone: move |id| clone_day.set(Some(id)),
                        onchange: move |()| bump(revision),
                        onadd: move |()| day_form.set(Some(None)),
                    }
                }
            }
        }
        if active.is_some() {
            ActiveSessionPill {}
        } else {
            div { class: "fixed bottom-24 end-[max(1rem,calc(50%-13rem))] z-20",
                button {
                    class: if can_start {
                        "flex h-14 items-center gap-2 rounded-full bg-lilac px-6 text-base font-semibold text-black shadow-xl"
                    } else {
                        "flex h-14 items-center gap-2 rounded-full bg-tile px-6 text-base font-semibold text-muted"
                    },
                    disabled: !can_start,
                    onclick: move |_| {
                        let Some(day_id) = current_id else {
                            return;
                        };
                        match db::start_session(day_id) {
                            Ok(_) => {
                                navigator.push(Route::Session {});
                            }
                            Err(err) => {
                                dioxus::logger::tracing::error!("start session failed: {err:#}");
                            }
                        }
                    },
                    "{i18n.t(\"workout.startWorkout\")}"
                }
            }
        }
        if edit_routine.cloned() {
            RoutineForm {
                key: "{routine.id}",
                closing: edit_closing(),
                routine: Some(routine.clone()),
                onclose: close_edit,
                onsave: move |()| {
                    bump(revision);
                },
            }
        }
        if let Some(slot) = day_form.cloned() {
            DayForm {
                key: "{slot.unwrap_or(0)}",
                closing: day_form_closing(),
                routine_id: routine.id,
                split: routine.split,
                day: editing_day,
                onclose: close_day_form,
                onsave: move |()| {
                    bump(revision);
                },
            }
        }
        if focus_scope.is_some() {
            MuscleFocus {
                closing: focus_closing(),
                title: focus_title,
                routine_id: match focus_scope {
                    Some(Focus::Routine) => Some(routine.id),
                    _ => None,
                },
                day_id: match focus_scope {
                    Some(Focus::Day(id)) => Some(id),
                    _ => None,
                },
                onclose: close_focus,
            }
        }
        if let Some(day) = cloning_day {
            DayClone {
                key: "{day.id}",
                closing: clone_closing(),
                day,
                routines,
                onclose: close_clone,
                onsave: move |()| {
                    bump(revision);
                },
            }
        }
    }
}

// shortcut: no edge auto-scroll, so a day longer than the viewport can only be reordered within one screen.
fn drag_move(mut reorder: Signal<Option<Reorder>>, last: usize, pitch: f64, y: f64) {
    if let Some(mut state) = reorder.cloned() {
        if state.started {
            state.delta = y - state.start_y;
        } else {
            state.started = true;
            state.start_y = y;
            state.delta = 0.0;
        }
        state.to = slot(state.delta, state.from, last, pitch);
        reorder.set(Some(state));
    }
}

fn settle_reorder(
    mut reorder: Signal<Option<Reorder>>,
    day_id: i64,
    onchange: EventHandler<()>,
    mut dragged_at: Signal<Option<std::time::Instant>>,
) {
    let state = reorder.cloned();
    reorder.set(None);
    let Some(state) = state else { return };
    if !state.started || state.to == state.from {
        return;
    }
    let Ok(exercises) = db::day_exercises(day_id) else {
        dioxus::logger::tracing::error!("reorder could not read day {day_id}");
        return;
    };
    let mut ids = exercises
        .iter()
        .map(|exercise| exercise.id)
        .collect::<Vec<_>>();
    let Some(position) = ids.iter().position(|id| *id == state.id) else {
        return;
    };
    ids.remove(position);
    let to = state.to.min(ids.len());
    ids.insert(to, state.id);
    match db::set_exercise_order(day_id, &ids) {
        Ok(()) => {
            dragged_at.set(Some(std::time::Instant::now()));
            onchange.call(());
        }
        Err(err) => dioxus::logger::tracing::error!("reorder failed: {err:#}"),
    }
}

fn settle_day_reorder(
    mut reorder: Signal<Option<Reorder>>,
    routine_id: i64,
    split: &str,
    onchange: EventHandler<()>,
    mut dragged_at: Signal<Option<std::time::Instant>>,
) {
    let state = reorder.cloned();
    reorder.set(None);
    let Some(state) = state else { return };
    if !state.started || state.to == state.from {
        return;
    }
    let Ok(days) = db::days(routine_id) else {
        dioxus::logger::tracing::error!("reorder could not read routine {routine_id}");
        return;
    };
    let mut ids = days.iter().map(|day| day.id).collect::<Vec<_>>();
    let Some(position) = ids.iter().position(|id| *id == state.id) else {
        return;
    };
    ids.remove(position);
    let to = state.to.min(ids.len());
    ids.insert(to, state.id);
    match db::set_day_order(routine_id, &ids, split) {
        Ok(()) => {
            dragged_at.set(Some(std::time::Instant::now()));
            onchange.call(());
        }
        Err(err) => dioxus::logger::tracing::error!("reorder failed: {err:#}"),
    }
}

fn bump(mut revision: Signal<u32>) {
    revision.set(revision.cloned().wrapping_add(1));
}

#[component]
fn DayList(
    routine_id: i64,
    days: Vec<db::RoutineDay>,
    split: String,
    selected: Option<i64>,
    onselect: EventHandler<i64>,
    onedit: EventHandler<i64>,
    ondelete: EventHandler<i64>,
    onclone: EventHandler<i64>,
    onchange: EventHandler<()>,
    onadd: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut reorder = use_signal(|| None::<Reorder>);
    let dragged_at = use_signal(|| None::<std::time::Instant>);
    let dragging = reorder.cloned();
    let last = days.len().saturating_sub(1);
    let mut items = days;
    if let Some(drag) = dragging.as_ref() {
        if let Some(position) = items.iter().position(|day| day.id == drag.id) {
            let day = items.remove(position);
            let to = drag.to.min(items.len());
            items.insert(to, day);
        }
    }

    rsx! {
        div { class: "flex flex-col gap-3",
            for (index, day) in items.iter().enumerate() {
                div {
                    key: "{day.id}",
                    class: day_class(
                        selected == Some(day.id),
                        dragging.as_ref().is_some_and(|drag| drag.id == day.id),
                    ),
                    style: card_offset(dragging.as_ref(), day.id, DAY_PITCH, last),
                    onclick: {
                        let id = day.id;
                        move |_| {
                            if dragged_at
                                .cloned()
                                .is_some_and(|at| at.elapsed().as_millis() < 300)
                            {
                                return;
                            }
                            onselect.call(id);
                        }
                    },
                    div { class: "flex h-full items-start gap-3",
                        button {
                            class: "grid size-10 shrink-0 touch-none place-items-center self-center text-muted",
                            "aria-label": i18n.t("workout.dragReorder"),
                            onpointerdown: {
                                let id = day.id;
                                move |event: PointerEvent| {
                                    event.stop_propagation();
                                    reorder.set(Some(Reorder {
                                        id,
                                        from: index,
                                        to: index,
                                        started: false,
                                        start_y: 0.0,
                                        delta: 0.0,
                                    }));
                                }
                            },
                            onpointermove: move |event: PointerEvent| {
                                drag_move(
                                    reorder,
                                    last,
                                    DAY_PITCH,
                                    event.data().client_coordinates().y,
                                );
                            },
                            onpointerup: {
                                let split = split.clone();
                                move |_| {
                                    settle_day_reorder(reorder, routine_id, &split, onchange, dragged_at);
                                }
                            },
                            onpointercancel: move |_| reorder.set(None),
                            onclick: move |event: MouseEvent| event.stop_propagation(),
                            Icon { icon: LdGripVertical, width: 18, height: 18 }
                        }
                        Thumb { art: art_for(day.muscle_main.as_deref()), size: 56 }
                        div { class: "min-w-0 flex-1",
                            span { class: "rounded-md bg-lilac/15 px-2 py-0.5 text-[12px] font-bold text-lilac",
                                "{day_badge(&i18n, &split, day.weekday, index)}"
                            }
                            div { class: "mt-1 truncate text-lg font-semibold", "{day.name}" }
                            div { class: "mt-0.5 truncate text-sm text-muted",
                                {day_meta(&i18n, day, true)}
                            }
                        }
                        div { class: "flex shrink-0 items-start",
                            button {
                                class: "grid size-10 place-items-center text-muted",
                                "aria-label": i18n.t("workout.cloneDay"),
                                onclick: {
                                    let id = day.id;
                                    move |event: MouseEvent| {
                                        event.stop_propagation();
                                        onclone.call(id);
                                    }
                                },
                                Icon { icon: LdCopy, width: 18, height: 18 }
                            }
                            RowActions {
                                vertical: true,
                                onedit: {
                                    let id = day.id;
                                    move |()| onedit.call(id)
                                },
                                ondelete: {
                                    let id = day.id;
                                    move |()| ondelete.call(id)
                                },
                            }
                        }
                    }
                }
            }
            button {
                class: "mt-4 flex h-12 w-full items-center justify-center gap-1.5 text-sm font-semibold text-accent-ink",
                onclick: move |_| onadd.call(()),
                Icon { icon: LdPlus, width: 18, height: 18 }
                "{i18n.t(\"workout.addDay\")}"
            }
        }
        if dragging.is_some() {
            div {
                class: "fixed inset-0 z-30 touch-none select-none",
                onpointermove: move |event: PointerEvent| {
                    drag_move(
                        reorder,
                        last,
                        DAY_PITCH,
                        event.data().client_coordinates().y,
                    );
                },
                onpointerup: move |_| {
                    settle_day_reorder(reorder, routine_id, &split, onchange, dragged_at);
                },
                onpointercancel: move |_| reorder.set(None),
            }
        }
    }
}

#[component]
fn DayExercises(
    day_id: i64,
    exercises: Vec<db::RoutineExercise>,
    onadd: EventHandler<()>,
    onremove: EventHandler<i64>,
    onchange: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut open = use_signal(|| None::<i64>);
    let mut reorder = use_signal(|| None::<Reorder>);
    let dragged_at = use_signal(|| None::<std::time::Instant>);
    let dragging = reorder.cloned();
    let last = exercises.len().saturating_sub(1);
    let mut items = exercises;
    if let Some(drag) = dragging.as_ref() {
        if let Some(position) = items.iter().position(|exercise| exercise.id == drag.id) {
            let exercise = items.remove(position);
            let to = drag.to.min(items.len());
            items.insert(to, exercise);
        }
    }
    let candidates = items.clone();

    rsx! {
        div { class: "mt-3 flex flex-col gap-2",
            for (index, exercise) in items.iter().enumerate() {
                div {
                    key: "{exercise.id}",
                    class: card_class(dragging.as_ref().is_some_and(|drag| drag.id == exercise.id)),
                    style: card_offset(dragging.as_ref(), exercise.id, EXERCISE_PITCH, last),
                    div {
                        class: "flex h-20 items-center gap-2 p-3",
                        onclick: {
                            let id = exercise.id;
                            move |_| {
                                if dragged_at
                                    .cloned()
                                    .is_some_and(|at| at.elapsed().as_millis() < 300)
                                {
                                    return;
                                }
                                let next = open.cloned() != Some(id);
                                open.set(next.then_some(id));
                            }
                        },
                        button {
                            class: "grid size-10 shrink-0 touch-none place-items-center text-muted",
                            "aria-label": i18n.t("workout.dragReorder"),
                            onpointerdown: {
                                let id = exercise.id;
                                move |event: PointerEvent| {
                                    event.stop_propagation();
                                    open.set(None);
                                    reorder.set(Some(Reorder {
                                        id,
                                        from: index,
                                        to: index,
                                        started: false,
                                        start_y: 0.0,
                                        delta: 0.0,
                                    }));
                                }
                            },
                            onpointermove: move |event: PointerEvent| {
                                drag_move(
                                    reorder,
                                    last,
                                    EXERCISE_PITCH,
                                    event.data().client_coordinates().y,
                                );
                            },
                            onpointerup: move |_| {
                                settle_reorder(reorder, day_id, onchange, dragged_at);
                            },
                            onpointercancel: move |_| reorder.set(None),
                            onclick: move |event: MouseEvent| event.stop_propagation(),
                            Icon { icon: LdGripVertical, width: 18, height: 18 }
                        }
                        ExerciseThumb { id: exercise.exercise_id, size: 56 }
                        div { class: "min-w-0 flex-1",
                            div { class: "truncate text-base font-semibold leading-snug",
                                "{exercise.name}"
                            }
                            div { class: "mt-0.5 flex items-center gap-2 text-sm text-muted",
                                if exercise.superset.is_some() {
                                    span { class: "inline-flex shrink-0 items-center gap-1 rounded-md bg-mango/15 px-1.5 py-0.5 text-[10px] font-bold text-mango",
                                        Icon { icon: LdLink2, width: 11, height: 11 }
                                        "{i18n.t(\"workout.superset\")}"
                                    }
                                }
                                span { class: "truncate", "{prescription(&i18n, exercise)}" }
                            }
                        }
                        DeleteButton {
                            class: "grid size-10 shrink-0 place-items-center self-start text-muted",
                            label: i18n.t("calendar.removeExercise"),
                            onconfirm: {
                                let id = exercise.id;
                                move || onremove.call(id)
                            },
                        }
                    }
                    if open.cloned() == Some(exercise.id) {
                        SetEditor {
                            exercise: exercise.clone(),
                            day_id,
                            candidates: candidates.clone(),
                            onchange: move |()| onchange.call(()),
                        }
                    }
                }
            }
            button {
                class: "mt-2 flex h-12 w-full items-center justify-center gap-1.5 text-sm font-semibold text-accent-ink",
                onclick: move |_| onadd.call(()),
                Icon { icon: LdPlus, width: 18, height: 18 }
                "{i18n.t(\"workout.addExercise\")}"
            }
        }
        if dragging.is_some() {
            div {
                class: "fixed inset-0 z-30 touch-none select-none",
                onpointermove: move |event: PointerEvent| {
                    drag_move(
                        reorder,
                        last,
                        EXERCISE_PITCH,
                        event.data().client_coordinates().y,
                    );
                },
                onpointerup: move |_| {
                    settle_reorder(reorder, day_id, onchange, dragged_at);
                },
                onpointercancel: move |_| reorder.set(None),
            }
        }
    }
}

#[component]
fn DayForm(
    closing: bool,
    routine_id: i64,
    split: String,
    day: Option<db::RoutineDay>,
    onclose: EventHandler<()>,
    onsave: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let existing = day.clone();
    let mut name = use_signal(|| {
        existing
            .as_ref()
            .map_or(String::new(), |day| day.name.clone())
    });
    let mut weekday = use_signal(|| existing.as_ref().and_then(|day| day.weekday));
    let weekday_mode = split == "weekday";
    let kept_weekday = day.as_ref().and_then(|day| day.weekday);
    let can_save = !name.read().trim().is_empty();
    let state = if closing { " is-closing" } else { "" };

    let save = move |_| {
        let trimmed = name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        let chosen = if weekday_mode {
            weekday.cloned()
        } else {
            kept_weekday
        };
        let result = day.as_ref().map_or_else(
            || db::add_day(routine_id, &trimmed, chosen).map(|_| ()),
            |day| db::update_day(day.id, &trimmed, chosen),
        );
        match result {
            Ok(()) => {
                onsave.call(());
                onclose.call(());
            }
            Err(err) => dioxus::logger::tracing::error!("save day failed: {err:#}"),
        }
    };

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
                    placeholder: i18n.t("workout.dayNamePlaceholder"),
                    value: "{name}",
                    oninput: move |event| name.set(event.value()),
                }
                if weekday_mode {
                    div { class: "mt-4 text-sm text-muted", "{i18n.t(\"workout.weekday\")}" }
                    div { class: "mt-2 grid grid-cols-7 gap-1.5",
                        for (index, label) in i18n.weekdays_short().iter().enumerate() {
                            button {
                                key: "{index}",
                                class: if weekday.cloned() == i64::try_from(index).ok() {
                                    "h-11 rounded-xl bg-lilac text-[13px] font-medium text-black"
                                } else {
                                    "h-11 rounded-xl border border-line text-[13px] font-medium text-lead"
                                },
                                onclick: {
                                    let value = i64::try_from(index).ok();
                                    move |_| weekday.set(if weekday.cloned() == value { None } else { value })
                                },
                                "{label}"
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

#[component]
fn SetEditor(
    exercise: db::RoutineExercise,
    day_id: i64,
    candidates: Vec<db::RoutineExercise>,
    onchange: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let id = exercise.id;
    let lb = db::unit_lb();
    let group = exercise.superset;
    let member_count = candidates
        .iter()
        .filter(|candidate| candidate.id == id || (group.is_some() && candidate.superset == group))
        .count();
    let initial = boundary_pairs(&candidates);
    let mut pairs = use_signal(move || initial);
    let mut linking = use_signal(|| false);
    let (linking_closing, close_linking) = use_exit(linking, false, |value| *value);
    let uses_weight = exercise.log_type == "weight_and_reps";
    let uses_reps = matches!(
        exercise.log_type.as_str(),
        "weight_and_reps" | "reps" | "reps_and_duration"
    );
    let uses_seconds = matches!(exercise.log_type.as_str(), "duration" | "reps_and_duration");
    let uses_cardio = exercise.log_type == "cardio";
    let open_candidates = candidates.clone();
    let done_candidates = candidates.clone();
    let links = pairs.cloned();
    let bars = (0..candidates.len())
        .map(|index| {
            index
                .checked_sub(1)
                .is_some_and(|previous| links.get(previous).copied().unwrap_or(false))
                || links.get(index).copied().unwrap_or(false)
        })
        .collect::<Vec<_>>();

    rsx! {
        div { class: "border-t border-line px-3 pt-3 pb-3 reveal-enter",
            div { class: "flex items-center justify-between gap-3",
                span { class: "text-xs font-semibold uppercase tracking-wide text-muted",
                    "{i18n.t(\"workout.restSeconds\")}"
                }
                input {
                    class: "h-9 w-20 rounded-lg bg-surface px-2 text-end text-sm text-lead outline-none",
                    r#type: "number",
                    inputmode: "numeric",
                    value: "{exercise.rest_seconds}",
                    onchange: move |event| {
                        let raw = event.value();
                        let trimmed = raw.trim();
                        let seconds = if trimmed.is_empty() {
                            db::default_rest()
                        } else {
                            match trimmed.parse::<i64>() {
                                Ok(value) => value,
                                Err(_) => return,
                            }
                        };
                        match db::set_rest(id, seconds) {
                            Ok(()) => onchange.call(()),
                            Err(err) => dioxus::logger::tracing::error!("rest failed: {err:#}"),
                        }
                    },
                }
            }
            div { class: "mt-3 flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted",
                span { class: "w-8 shrink-0", "{i18n.t(\"common.labels.set\")}" }
                if uses_weight {
                    span { class: "flex-1 text-center", "{i18n.unit(lb)}" }
                }
                if uses_reps {
                    span { class: "flex-1 text-center", "{i18n.t(\"common.labels.reps\")}" }
                }
                if uses_seconds {
                    span { class: "flex-1 text-center", "{i18n.t(\"common.labels.secs\")}" }
                }
                if uses_cardio {
                    span { class: "flex-1 text-center", "{i18n.t(\"common.labels.min\")}" }
                    span { class: "flex-1 text-center", "{i18n.t(\"common.labels.km\")}" }
                }
                span { class: "size-8 shrink-0" }
            }
            for set in exercise.sets.iter() {
                SetRow {
                    key: "{set.set_number}",
                    entry: id,
                    set: set.clone(),
                    uses_weight,
                    uses_reps,
                    uses_seconds,
                    uses_cardio,
                    lb,
                    onchange,
                }
            }
            button {
                class: "mt-2 flex h-10 w-full items-center justify-center gap-1.5 rounded-xl bg-surface text-sm font-medium text-accent-ink",
                onclick: move |_| match db::add_set(id) {
                    Ok(()) => onchange.call(()),
                    Err(err) => dioxus::logger::tracing::error!("add set failed: {err:#}"),
                },
                Icon { icon: LdPlus, width: 16, height: 16 }
                "{i18n.t(\"workout.addSet\")}"
            }
            div { class: "mt-3 flex gap-2",
                button {
                    class: "flex h-11 min-w-0 flex-1 items-center justify-center gap-1 rounded-xl bg-surface px-1 text-center text-[12px] font-medium leading-tight text-lead",
                    onclick: move |_| match db::clone_exercise(id) {
                        Ok(_) => onchange.call(()),
                        Err(err) => dioxus::logger::tracing::error!("clone exercise failed: {err:#}"),
                    },
                    Icon { icon: LdCopy, width: 16, height: 16 }
                    "{i18n.t(\"workout.cloneExercise\")}"
                }
                button {
                    class: "flex h-11 min-w-0 flex-1 items-center justify-center gap-1 rounded-xl bg-surface px-1 text-center text-[12px] font-medium leading-tight text-lead",
                    onclick: move |_| {
                        navigator.push(Route::RoutineExerciseSwap { entry: id });
                    },
                    Icon { icon: LdRefreshCw, width: 16, height: 16 }
                    "{i18n.t(\"workout.swap\")}"
                }
                button {
                    class: "flex h-11 min-w-0 flex-1 items-center justify-center gap-1 rounded-xl bg-surface px-1 text-center text-[12px] font-medium leading-tight text-lead",
                    onclick: move |_| {
                        pairs.set(boundary_pairs(&open_candidates));
                        linking.set(true);
                    },
                    Icon { icon: LdLink2, width: 16, height: 16 }
                    if member_count >= 2 {
                        "{i18n.t(\"workout.superset\")}"
                    } else {
                        "{i18n.t(\"workout.linkSuperset\")}"
                    }
                }
            }
        }
        if linking.cloned() {
            Sheet {
                closing: linking_closing(),
                onclose: close_linking,
                div { class: "px-2 pb-2",
                    div { class: "flex max-h-[55dvh] flex-col overflow-y-auto",
                        for (index, candidate) in candidates.iter().enumerate() {
                            div {
                                key: "{candidate.id}",
                                class: "relative flex items-center gap-3 px-3 py-2",
                                if bars.get(index).copied().unwrap_or(false) {
                                    div { class: "absolute inset-y-2 start-0 w-1 rounded-full bg-red-500" }
                                }
                                ExerciseThumb { id: candidate.exercise_id, size: 48 }
                                span { class: "min-w-0 flex-1 truncate text-[15px] font-medium",
                                    "{candidate.name}"
                                }
                            }
                            if index.saturating_add(1) < candidates.len() {
                                div {
                                    key: "link-{index}",
                                    class: "flex h-8 shrink-0 items-center justify-end pe-3",
                                    button {
                                        class: if links.get(index).copied().unwrap_or(false) {
                                            "grid size-7 place-items-center rounded-full bg-red-500 text-black"
                                        } else {
                                            "grid size-7 place-items-center rounded-full border border-line text-lead"
                                        },
                                        "aria-label": i18n.t("workout.superset"),
                                        onclick: {
                                            let done_candidates = done_candidates.clone();
                                            move |_| {
                                                toggle_pair(pairs, index, &done_candidates, day_id, onchange);
                                            }
                                        },
                                        Icon { icon: LdLink, width: 14, height: 14 }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn boundary_pairs(candidates: &[db::RoutineExercise]) -> Vec<bool> {
    candidates
        .iter()
        .zip(candidates.iter().skip(1))
        .map(|(left, right)| left.superset.is_some() && left.superset == right.superset)
        .collect()
}

fn superset_groups(candidates: &[db::RoutineExercise], pairs: &[bool]) -> Vec<Vec<i64>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        current.push(candidate.id);
        if !pairs.get(index).copied().unwrap_or(false) {
            if current.len() >= 2 {
                groups.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        }
    }
    groups
}

fn toggle_pair(
    mut pairs: Signal<Vec<bool>>,
    index: usize,
    candidates: &[db::RoutineExercise],
    day_id: i64,
    onchange: EventHandler<()>,
) {
    pairs.with_mut(|list| {
        if let Some(value) = list.get_mut(index) {
            *value = !*value;
        }
    });
    match db::set_day_supersets(day_id, &superset_groups(candidates, &pairs.cloned())) {
        Ok(()) => onchange.call(()),
        Err(err) => dioxus::logger::tracing::error!("superset failed: {err:#}"),
    }
}

#[component]
#[allow(clippy::too_many_arguments)]
fn SetRow(
    entry: i64,
    set: db::RoutineSet,
    uses_weight: bool,
    uses_reps: bool,
    uses_seconds: bool,
    uses_cardio: bool,
    lb: bool,
    onchange: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let number = set.set_number;
    let weight = set.target_weight;
    let reps = set.target_reps;
    let seconds = set.target_seconds;
    let distance = set.target_distance_m;
    rsx! {
        div { class: "flex items-center gap-2 py-1",
            span { class: "w-8 shrink-0 text-sm text-muted", "{number}" }
            if uses_weight {
                NumField {
                    value: weight_field(weight, lb),
                    onsave: move |raw: String| {
                        let weight = parse_weight(&raw, lb);
                        save_set(entry, number, weight, reps, seconds, distance, onchange);
                    },
                }
            }
            if uses_reps {
                NumField {
                    value: number_text(reps),
                    onsave: move |raw: String| {
                        let reps = raw.trim().parse::<i64>().ok();
                        save_set(entry, number, weight, reps, seconds, distance, onchange);
                    },
                }
            }
            if uses_seconds {
                NumField {
                    value: number_text(seconds),
                    onsave: move |raw: String| {
                        let seconds = raw.trim().parse::<i64>().ok();
                        save_set(entry, number, weight, reps, seconds, distance, onchange);
                    },
                }
            }
            if uses_cardio {
                NumField {
                    value: minutes_text(seconds),
                    onsave: move |raw: String| {
                        let seconds = raw.trim().parse::<i64>().ok().map(|minutes| minutes.saturating_mul(60));
                        save_set(entry, number, weight, reps, seconds, distance, onchange);
                    },
                }
                NumField {
                    value: km_text(distance),
                    onsave: move |raw: String| {
                        let distance = raw.trim().parse::<f64>().ok().map(|km| km * 1000.0);
                        save_set(entry, number, weight, reps, seconds, distance, onchange);
                    },
                }
            }
            button {
                class: "grid size-8 shrink-0 place-items-center rounded-lg text-muted",
                "aria-label": i18n.t("workout.deleteSet"),
                onclick: move |_| match db::remove_set(entry, number) {
                    Ok(()) => onchange.call(()),
                    Err(err) => dioxus::logger::tracing::error!("delete set failed: {err:#}"),
                },
                Icon { icon: LdX, width: 16, height: 16 }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn save_set(
    entry: i64,
    set_number: i64,
    weight: Option<f64>,
    reps: Option<i64>,
    seconds: Option<i64>,
    distance_m: Option<f64>,
    onchange: EventHandler<()>,
) {
    match db::update_set(entry, set_number, weight, reps, seconds, distance_m) {
        Ok(()) => onchange.call(()),
        Err(err) => dioxus::logger::tracing::error!("set {set_number} failed: {err:#}"),
    }
}

#[component]
fn NumField(value: String, onsave: EventHandler<String>) -> Element {
    rsx! {
        input {
            class: "h-10 min-w-0 flex-1 rounded-lg bg-surface px-2 text-center text-sm text-lead outline-none",
            r#type: "number",
            inputmode: "decimal",
            value: "{value}",
            onchange: move |event| onsave.call(event.value()),
        }
    }
}

#[component]
pub fn RoutineDayAdd(day_id: i64) -> Element {
    chooser(None, Some(day_id))
}

#[component]
pub fn RoutineExerciseSwap(entry: i64) -> Element {
    chooser(Some(entry), None)
}

#[allow(clippy::too_many_lines)]
fn chooser(swap: Option<i64>, day_id: Option<i64>) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let mut chosen = use_signal(Vec::<i64>::new);
    let rows = use_memo(move || {
        i18n.language.read();
        db::exercises().unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("exercise list failed: {err:#}");
            Vec::new()
        })
    });
    let search = use_signal(String::new);
    let equipment = use_signal(Vec::<String>::new);
    let muscles = use_signal(Vec::<String>::new);
    let difficulty = use_signal(Vec::<String>::new);
    let all = rows.cloned();
    let equipment_choices = equipment_options(&all, &i18n);
    let needle = search.cloned().trim().to_lowercase();
    let selected_equipment = equipment.cloned();
    let selected_muscles = muscles.cloned();
    let selected_difficulty = difficulty.cloned();
    let shown = all
        .into_iter()
        .filter(|row| {
            matches_filters(
                &i18n,
                row,
                &needle,
                &selected_equipment,
                &selected_muscles,
                &selected_difficulty,
            )
        })
        .collect::<Vec<_>>();
    let title = if swap.is_some() {
        i18n.t("workout.swapExercise")
    } else {
        i18n.t("workout.addExercise")
    };
    let chosen_count = i64::try_from(chosen.read().len()).unwrap_or(i64::MAX);

    rsx! {
        div { class: "min-h-dvh pb-56",
            header { class: "flex items-center gap-3 px-5 pt-5",
                button {
                    class: "text-lead",
                    onclick: move |_| navigator.go_back(),
                    BackIcon { size: 24 }
                }
                h1 { class: "text-[22px] font-bold tracking-tight", "{title}" }
            }
            if shown.is_empty() {
                p { class: "px-5 py-12 text-center text-base text-muted",
                    "{i18n.t(\"exercises.noMatch\")}"
                }
            }
            div { class: "mt-2 flex flex-col px-5",
                for row in shown {
                    button {
                        key: "{row.id}",
                        class: if swap.is_none() && chosen.read().contains(&row.id) {
                            "flex w-full items-center gap-4 rounded-2xl bg-accent/10 px-2 py-3.5 text-start"
                        } else {
                            "flex w-full items-center gap-4 px-2 py-3.5 text-start"
                        },
                        onclick: {
                            let id = row.id;
                            move |_| match swap {
                                Some(entry) => match db::replace_exercise(entry, id) {
                                    Ok(()) => {
                                        navigator.go_back();
                                    }
                                    Err(err) => {
                                        dioxus::logger::tracing::error!("swap exercise failed: {err:#}");
                                    }
                                },
                                None => chosen.with_mut(|list| {
                                    if let Some(position) = list.iter().position(|item| *item == id) {
                                        list.remove(position);
                                    } else {
                                        list.push(id);
                                    }
                                }),
                            }
                        },
                        ExerciseThumb { id: row.id, size: 56 }
                        div { class: "min-w-0 flex-1",
                            div { class: "text-base font-semibold leading-snug", "{row.name}" }
                            div { class: "mt-0.5 truncate text-sm text-muted", "{summary(&i18n, &row)}" }
                        }
                        if swap.is_some() {
                            Icon { icon: LdRefreshCw, width: 20, height: 20, class: "shrink-0 text-accent-ink" }
                        } else if chosen.read().contains(&row.id) {
                            Icon { icon: LdCheck, width: 20, height: 20, class: "shrink-0 text-accent-ink" }
                        } else {
                            Icon { icon: LdPlus, width: 20, height: 20, class: "shrink-0 text-accent-ink" }
                        }
                    }
                }
            }
        }
        div { class: "fixed inset-x-0 bottom-[calc(env(safe-area-inset-bottom)+4rem)] z-20 mx-auto w-full max-w-md border-t border-line bg-surface",
            ExerciseFilters {
                search,
                equipment,
                muscles,
                difficulty,
                equipment_options: equipment_choices,
            }
            if let Some(day_id) = day_id {
                div { class: "px-5 pb-3",
                    button {
                        class: if chosen_count == 0 {
                            "h-12 w-full rounded-2xl bg-tile text-base font-semibold text-muted"
                        } else {
                            "h-12 w-full rounded-2xl bg-lilac text-base font-semibold text-black"
                        },
                        disabled: chosen_count == 0,
                        onclick: move |_| match db::add_exercises(day_id, &chosen.cloned()) {
                            Ok(()) => {
                                navigator.replace(Route::RoutineDay { day_id });
                            }
                            Err(err) => dioxus::logger::tracing::error!("add exercises failed: {err:#}"),
                        },
                        "{i18n.t(\"common.actions.save\")}"
                    }
                }
            }
        }
    }
}

#[component]
fn MuscleFocus(
    closing: bool,
    title: String,
    routine_id: Option<i64>,
    day_id: Option<i64>,
    onclose: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let rows = match (day_id, routine_id) {
        (Some(day_id), _) => db::day_muscle_counts(day_id),
        (None, Some(routine_id)) => db::routine_muscle_counts(routine_id),
        (None, None) => Ok(Vec::new()),
    }
    .unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("muscle focus failed: {err:#}");
        Vec::new()
    });
    let mut groups = FOCUS_GROUPS
        .iter()
        .map(|label| {
            let (exercises, sets) = rows
                .iter()
                .find(|(muscle, _, _)| muscle == label)
                .map_or((0, 0), |(_, exercises, sets)| (*exercises, *sets));
            ((*label).to_string(), exercises, sets)
        })
        .collect::<Vec<_>>();
    groups.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    rsx! {
        Sheet {
            closing,
            onclose,
            div { class: "px-3 pb-1",
                h2 { class: "text-lg font-semibold", "{i18n.t(\"workout.bodyMap\")}" }
                p { class: "mt-0.5 text-sm text-muted", "{title}" }
            }
            div { class: "max-h-[65dvh] overflow-y-auto px-3 pb-2",
                div { class: "mt-2",
                    BodyMap {
                        counts: rows.iter().map(|(muscle, exercises, _)| (muscle.clone(), *exercises)).collect()
                    }
                }
                div { class: "mt-4 grid grid-cols-[1fr_5.5rem_4rem] items-center gap-2 border-b border-line pb-2 text-xs font-semibold uppercase tracking-wide text-muted",
                    span {}
                    span { class: "text-right", "{i18n.t(\"nav.exercises\")}" }
                    span { class: "text-right", "{i18n.t(\"progress.sets\")}" }
                }
                div { class: "flex flex-col",
                    for (label, exercises, sets) in groups {
                        div {
                            key: "{label}",
                            class: "grid grid-cols-[1fr_5.5rem_4rem] items-center gap-2 border-b border-line py-2.5",
                            span {
                                class: if exercises == 0 { "text-[15px] text-muted" } else { "text-[15px]" },
                                "{i18n.value(\"exercises.muscles\", &label)}"
                            }
                            span {
                                class: if exercises == 0 { "text-right text-sm text-muted" } else { "text-right text-sm font-semibold text-lilac" },
                                "{exercises}"
                            }
                            span {
                                class: if sets == 0 { "text-right text-sm text-muted" } else { "text-right text-sm font-semibold text-lilac" },
                                "{sets}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DayClone(
    closing: bool,
    day: db::RoutineDay,
    routines: Vec<db::Routine>,
    onclose: EventHandler<()>,
    onsave: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut name = use_signal(|| i18n.ta("routines.copyName", &[("name", &day.name)]));
    let mut target = use_signal(|| day.routine_id);
    let can_save = !name.read().trim().is_empty();
    let state = if closing { " is-closing" } else { "" };

    let save = move |_| {
        let trimmed = name.read().trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        match db::clone_day(day.id, target.cloned(), &trimmed) {
            Ok(_) => {
                onsave.call(());
                onclose.call(());
            }
            Err(err) => dioxus::logger::tracing::error!("clone day failed: {err:#}"),
        }
    };

    rsx! {
        div { class: "fixed inset-0 z-40 flex items-end justify-center",
            div {
                class: "absolute inset-0 bg-black/60 sheet-backdrop{state}",
                onclick: move |_| onclose.call(()),
            }
            div { class: "relative max-h-[92dvh] w-full max-w-md overflow-y-auto rounded-t-3xl bg-surface p-5 pb-[calc(env(safe-area-inset-bottom)+1.25rem)] sheet-panel{state}",
                h2 { class: "text-lg font-semibold", "{i18n.t(\"workout.cloneDay\")}" }
                label { class: "mt-5 block text-sm text-muted", "{i18n.t(\"routines.name\")}" }
                input {
                    class: "mt-2 h-12 w-full rounded-2xl bg-tile px-4 text-base text-lead outline-none",
                    value: "{name}",
                    oninput: move |event| name.set(event.value()),
                }
                div { class: "mt-4 text-sm text-muted", "{i18n.t(\"workout.destination\")}" }
                div { class: "mt-2 flex flex-wrap gap-2",
                    for routine in routines.iter() {
                        button {
                            key: "{routine.id}",
                            class: if target.cloned() == routine.id {
                                "h-10 rounded-xl bg-lilac px-4 text-sm font-medium text-black"
                            } else {
                                "h-10 rounded-xl border border-line px-4 text-sm font-medium text-lead"
                            },
                            onclick: {
                                let id = routine.id;
                                move |_| target.set(id)
                            },
                            "{routine.name}"
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
                    "{i18n.t(\"workout.cloneDay\")}"
                }
            }
        }
    }
}

fn day_badge(i18n: &I18n, split: &str, weekday: Option<i64>, index: usize) -> String {
    let number = i64::try_from(index.saturating_add(1)).unwrap_or(i64::MAX);
    let fallback = i18n.ta("workout.dayBadge", &[("number", &number.to_string())]);
    if split == "weekday" {
        weekday
            .and_then(|value| usize::try_from(value).ok())
            .and_then(|value| WEEKDAYS.get(value).copied())
            .map_or(fallback, |day| i18n.weekday_long(day).to_uppercase())
    } else {
        fallback
    }
}

fn day_meta(i18n: &I18n, day: &db::RoutineDay, with_last_done: bool) -> String {
    let est = i18n.ta("workout.est", &[("minutes", &day.est_minutes.to_string())]);
    let count = i18n.tn("workout.exercises", day.exercise_count);
    let mut meta = format!("{est}, {count}");
    if with_last_done {
        if let Some(last_done) = day.last_done {
            meta.push_str(" · ");
            meta.push_str(&ago(i18n, last_done));
        }
    }
    meta
}

fn prescription(i18n: &I18n, exercise: &db::RoutineExercise) -> String {
    let count = exercise.sets.len();
    if count == 0 {
        return i18n.t("workout.noSets");
    }
    let count = i64::try_from(count).unwrap_or(i64::MAX);
    let reps = joined(exercise.sets.iter().map(|set| set.target_reps));
    let seconds = joined(exercise.sets.iter().map(|set| set.target_seconds));
    match exercise.log_type.as_str() {
        "duration" => i18n.ta(
            "workout.prescriptionDuration",
            &[("count", &count.to_string()), ("seconds", &seconds)],
        ),
        "cardio" => {
            let minutes = joined(
                exercise
                    .sets
                    .iter()
                    .map(|set| set.target_seconds.map(|value| value / 60)),
            );
            i18n.ta(
                "workout.prescriptionCardio",
                &[("count", &count.to_string()), ("minutes", &minutes)],
            )
        }
        "reps_and_duration" => i18n.ta(
            "workout.prescriptionRepsDuration",
            &[
                ("count", &count.to_string()),
                ("reps", &reps),
                ("seconds", &seconds),
            ],
        ),
        _ => i18n.ta(
            "workout.prescription",
            &[("count", &count.to_string()), ("reps", &reps)],
        ),
    }
}

fn joined(values: impl Iterator<Item = Option<i64>>) -> String {
    let list = values
        .map(|value| value.map_or_else(|| "-".to_string(), |value| value.to_string()))
        .collect::<Vec<_>>();
    let first = list.first().cloned().unwrap_or_default();
    if list.iter().all(|value| value == &first) {
        first
    } else {
        list.join(",")
    }
}

fn number_text(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn minutes_text(seconds: Option<i64>) -> String {
    seconds.map_or_else(String::new, |seconds| (seconds / 60).to_string())
}

fn km_text(distance_m: Option<f64>) -> String {
    distance_m.map_or_else(String::new, |distance| (distance / 1000.0).to_string())
}

const fn card_class(dragged: bool) -> &'static str {
    if dragged {
        "relative z-20 rounded-2xl bg-tile shadow-2xl"
    } else {
        "rounded-2xl bg-tile"
    }
}

fn day_class(selected: bool, dragged: bool) -> String {
    let mut class = String::from("h-[116px] overflow-hidden rounded-2xl border bg-tile p-4");
    class.push_str(if selected {
        " border-accent"
    } else {
        " border-line"
    });
    if dragged {
        class.push_str(" relative z-20 shadow-2xl");
    }
    class
}

fn card_offset(drag: Option<&Reorder>, id: i64, pitch: f64, last: usize) -> String {
    let Some(drag) = drag.filter(|drag| drag.id == id && drag.started) else {
        return String::new();
    };
    let from = index_f64(drag.from);
    let to = index_f64(drag.to);
    let limit = index_f64(last) * pitch;
    let visual = (from.mul_add(pitch, drag.delta)).clamp(0.0, limit);
    let offset = to.mul_add(-pitch, visual);
    format!("transform: translateY({offset}px)")
}

fn index_f64(index: usize) -> f64 {
    f64::from(u32::try_from(index).unwrap_or(u32::MAX))
}

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn slot(delta: f64, from: usize, last: usize, pitch: f64) -> usize {
    let steps = (delta / pitch).round() as i64;
    let from = i64::try_from(from).unwrap_or(i64::MAX);
    let last = i64::try_from(last).unwrap_or(i64::MAX);
    usize::try_from(from.saturating_add(steps).clamp(0, last)).unwrap_or(0)
}

fn ago(i18n: &I18n, epoch: i64) -> String {
    let days = db::now().saturating_sub(epoch) / 86_400;
    if days <= 0 {
        i18n.t("workout.today")
    } else {
        i18n.tn("workout.daysAgo", days)
    }
}
