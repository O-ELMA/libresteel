use std::time::Duration;

use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdCheck, LdLink2, LdPlus, LdSquareKanban, LdTarget, LdTimer, LdTrash2},
    Icon,
};

use crate::{
    calendar::volume_text,
    db::{self, ActiveSession, SessionExercise, SessionSet},
    dioxus_router::Navigator,
    exercises::{art_for, ExerciseThumb},
    i18n::{use_i18n, I18n},
    media,
    shell::Route,
    suggest::{self, Block, Goal},
    ui::{display_weight, parse_weight, weight_field, weight_unit, BackIcon, DeleteButton},
};

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Weight,
    Reps,
    Seconds,
    Distance,
}

#[derive(Clone, Copy, PartialEq)]
enum Column {
    Weight,
    Reps,
    Seconds,
    Minutes,
    Km,
}

#[derive(Clone, Default, PartialEq)]
struct Inputs {
    weight: String,
    reps: String,
    seconds: String,
    distance: String,
}

impl Inputs {
    const fn field(&self, field: Field) -> &String {
        match field {
            Field::Weight => &self.weight,
            Field::Reps => &self.reps,
            Field::Seconds => &self.seconds,
            Field::Distance => &self.distance,
        }
    }

    const fn field_mut(&mut self, field: Field) -> &mut String {
        match field {
            Field::Weight => &mut self.weight,
            Field::Reps => &mut self.reps,
            Field::Seconds => &mut self.seconds,
            Field::Distance => &mut self.distance,
        }
    }

    fn with_fallback(&self, fallback: &Self) -> Self {
        Self {
            weight: pick(&self.weight, &fallback.weight).to_string(),
            reps: pick(&self.reps, &fallback.reps).to_string(),
            seconds: pick(&self.seconds, &fallback.seconds).to_string(),
            distance: pick(&self.distance, &fallback.distance).to_string(),
        }
    }
}

#[derive(Clone, PartialEq)]
struct SetState {
    number: i64,
    inputs: Inputs,
    defaults: Inputs,
    suggested: Option<suggest::Target>,
    done: bool,
}

#[derive(Clone, PartialEq)]
struct ExerciseState {
    id: i64,
    exercise_id: i64,
    name: String,
    muscle_main: Option<String>,
    log_type: String,
    rest_seconds: i64,
    superset: Option<i64>,
    best_1rm: Option<f64>,
    prior_1rm: Option<f64>,
    sets: Vec<SetState>,
}

#[component]
pub fn Session() -> Element {
    let i18n = use_i18n();
    let session = db::active_session().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("active session failed: {err:#}");
        None
    });
    let Some(session) = session else {
        return rsx! {
            div { class: "flex min-h-dvh flex-col items-center justify-center gap-4 px-6 text-center",
                p { class: "text-base text-muted", "{i18n.t(\"session.noWorkout\")}" }
                Link {
                    to: Route::Workout {},
                    class: "text-sm font-semibold text-accent-ink",
                    "{i18n.t(\"session.backToWorkout\")}"
                }
            }
        };
    };
    rsx! {
        LiveSession { session }
    }
}

#[component]
pub(crate) fn ActiveSessionPill() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let Some(active) = db::active_session().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("active session failed: {err:#}");
        None
    }) else {
        return rsx! {};
    };
    let name = db::session_exercises(&active).map_or_else(
        |_| active.name.clone(),
        |rows| {
            rows.get(db::current_position(&rows))
                .map_or_else(|| active.name.clone(), |row| row.name.clone())
        },
    );
    rsx! {
        div { class: "fixed bottom-24 left-1/2 z-20 w-full max-w-md -translate-x-1/2 px-5",
            div {
                class: "flex items-center gap-3 rounded-full border border-line bg-tile py-1.5 ps-5 pe-1.5 shadow-xl",
                onclick: move |_| {
                    navigator.push(Route::Session {});
                },
                div { class: "min-w-0 flex-1",
                    Elapsed { started_at: active.started_at }
                    div { class: "truncate text-sm font-semibold", "{name}" }
                }
                button {
                    class: "h-11 shrink-0 rounded-full bg-lilac px-4 text-sm font-semibold text-black",
                    onclick: move |event: MouseEvent| {
                        event.stop_propagation();
                        navigator.push(Route::SessionSummary {});
                    },
                    "{i18n.t(\"workout.endWorkout\")}"
                }
            }
        }
    }
}

#[component]
fn LiveSession(session: ActiveSession) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let lb = db::unit_lb();
    let block = use_signal(|| {
        db::routine_block(session.routine_id, session.started_at).unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("routine block failed: {err:#}");
            None
        })
    });
    let mut states = use_signal(|| load(&session, lb, block.cloned()));
    let mut index = use_signal(|| start_index(&session));
    let mut selected = use_signal(|| None::<usize>);
    let rest = use_signal(|| session.rest_until);
    let mut complete = use_signal(|| false);
    let mut drag_from = use_signal(|| None::<f64>);

    let exercises = states.cloned();
    let total = exercises.len();
    let current_index = index().min(total.saturating_sub(1));
    let current = exercises.get(current_index).cloned();
    let orm = current
        .as_ref()
        .and_then(|exercise| exercise.best_1rm)
        .map_or_else(
            || "—".to_string(),
            |value| {
                format!(
                    "{} {}",
                    orm_text(display_weight(value, lb)),
                    weight_unit(lb)
                )
            },
        );
    let logged_sets = exercises
        .iter()
        .flat_map(|exercise| exercise.sets.iter())
        .filter(|set| set.done)
        .count();
    let set_count = current.as_ref().map_or(0, |exercise| exercise.sets.len());
    let selected_index = selected()
        .filter(|value| *value < set_count)
        .unwrap_or_else(|| {
            current
                .as_ref()
                .and_then(|exercise| exercise.sets.iter().position(|set| !set.done))
                .unwrap_or_else(|| set_count.saturating_sub(1))
        });
    let all_logged = current.as_ref().is_some_and(|exercise| {
        !exercise.sets.is_empty() && exercise.sets.iter().all(|set| set.done)
    });
    let last_index = current_index.saturating_add(1) >= total;

    if complete() {
        return rsx! {
            CompleteScreen {
                session,
                onback: move |()| complete.set(false),
            }
        };
    }

    let mut set_field =
        move |exercise_index: usize, set_index: usize, field: Field, value: String| {
            let mut persist = None;
            states.with_mut(|list| {
                if let Some(exercise) = list.get_mut(exercise_index) {
                    if let Some(set) = exercise.sets.get_mut(set_index) {
                        set.inputs.field_mut(field).clone_from(&value);
                        if set.done {
                            persist = Some((
                                exercise.id,
                                exercise.log_type.clone(),
                                set.number,
                                set.inputs.clone(),
                                set.defaults.clone(),
                            ));
                        }
                    }
                }
            });
            if let Some((id, log_type, number, inputs, defaults)) = persist {
                let (weight, reps, seconds, distance) =
                    values(&log_type, &inputs.with_fallback(&defaults), lb);
                if let Err(err) = db::log_set(id, number, weight, reps, seconds, distance) {
                    dioxus::logger::tracing::error!("update set failed: {err:#}");
                }
            }
        };

    let mut log_selected = move |()| {
        let Some(exercise) = states.cloned().get(current_index).cloned() else {
            return;
        };
        let Some(position) = selected()
            .filter(|value| *value < exercise.sets.len())
            .or_else(|| exercise.sets.iter().position(|set| !set.done))
        else {
            return;
        };
        let Some(set) = exercise.sets.get(position).cloned() else {
            return;
        };
        let (weight, reps, seconds, distance) = values(
            &exercise.log_type,
            &set.inputs.with_fallback(&set.defaults),
            lb,
        );
        if let Err(err) = db::log_set(exercise.id, set.number, weight, reps, seconds, distance) {
            dioxus::logger::tracing::error!("log set failed: {err:#}");
            return;
        }
        let complete = states.with_mut(|list| {
            let Some(exercise) = list.get_mut(current_index) else {
                return false;
            };
            let Some(set) = exercise.sets.get_mut(position) else {
                return false;
            };
            let was_done = set.done;
            let effective = set.inputs.with_fallback(&set.defaults);
            set.inputs = effective;
            set.done = true;
            !was_done && exercise.sets.iter().all(|set| set.done)
        });
        selected.set(None);
        if exercise.rest_seconds > 0 {
            save_rest(
                session.id,
                rest,
                Some(db::now().saturating_add(exercise.rest_seconds)),
            );
        }
        if exercise.superset.is_some() {
            advance_superset(states, index, selected);
        } else if complete {
            advance(states, index, selected);
        }
    };

    let skip_rest = move |()| {
        save_rest(session.id, rest, None);
        advance(states, index, selected);
    };

    let mut skip_exercise = move |()| {
        let Some(exercise) = states.cloned().get(current_index).cloned() else {
            return;
        };
        let zeros = Inputs {
            weight: "0".to_string(),
            reps: "0".to_string(),
            seconds: "0".to_string(),
            distance: "0".to_string(),
        };
        let (weight, reps, seconds, distance) = values(&exercise.log_type, &zeros, lb);
        let numbers = if exercise.sets.is_empty() {
            vec![1]
        } else {
            exercise
                .sets
                .iter()
                .filter(|set| !set.done)
                .map(|set| set.number)
                .collect()
        };
        for number in numbers {
            if let Err(err) = db::log_set(exercise.id, number, weight, reps, seconds, distance) {
                dioxus::logger::tracing::error!("skip exercise failed: {err:#}");
                return;
            }
        }
        states.with_mut(|list| {
            if let Some(exercise) = list.get_mut(current_index) {
                if exercise.sets.is_empty() {
                    exercise.sets.push(SetState {
                        number: 1,
                        inputs: zeros.clone(),
                        defaults: Inputs::default(),
                        suggested: None,
                        done: true,
                    });
                }
                for set in &mut exercise.sets {
                    if !set.done {
                        set.inputs = zeros.clone();
                        set.done = true;
                    }
                }
            }
        });
        selected.set(None);
        save_rest(session.id, rest, None);
        advance(states, index, selected);
    };

    use_future(move || async move {
        loop {
            futures_timer::Delay::new(Duration::from_millis(500)).await;
            let Some(deadline) = rest.cloned() else {
                continue;
            };
            if db::now() >= deadline {
                save_rest(session.id, rest, None);
                advance_after_rest(states, index, selected);
            }
        }
    });

    let mut add_set = move |()| {
        states.with_mut(|list| {
            if let Some(exercise) = list.get_mut(current_index) {
                let defaults = exercise.sets.last().map_or_else(Inputs::default, |set| {
                    set.inputs.with_fallback(&set.defaults)
                });
                let number = i64::try_from(exercise.sets.len())
                    .unwrap_or(i64::MAX)
                    .saturating_add(1);
                exercise.sets.push(SetState {
                    number,
                    inputs: Inputs::default(),
                    defaults,
                    suggested: None,
                    done: false,
                });
            }
        });
        selected.set(None);
    };

    let mut remove_set = move |()| {
        let mut removed = None;
        states.with_mut(|list| {
            if let Some(exercise) = list.get_mut(current_index) {
                let last = exercise.sets.len().saturating_sub(1);
                if last > 0 && exercise.sets.get(last).is_some_and(|set| !set.done) {
                    removed = exercise.sets.get(last).map(|set| (exercise.id, set.number));
                    exercise.sets.remove(last);
                }
            }
        });
        if let Some((id, number)) = removed {
            if let Err(err) = db::remove_session_set(id, number) {
                dioxus::logger::tracing::error!("remove set failed: {err:#}");
            }
        }
        selected.set(None);
    };

    let can_log = current.as_ref().is_some_and(|exercise| {
        exercise
            .sets
            .get(selected_index)
            .is_some_and(|set| ready(&exercise.log_type, &set.inputs.with_fallback(&set.defaults)))
    });
    let done_exercises = exercises
        .iter()
        .map(|exercise| !exercise.sets.is_empty() && exercise.sets.iter().all(|set| set.done))
        .collect::<Vec<_>>();
    let (action, enabled) = if rest.cloned().is_some() {
        (i18n.t("session.skipRest"), true)
    } else if all_logged && last_index {
        (i18n.t("session.finish"), logged_sets > 0)
    } else {
        (i18n.t("session.logSet"), can_log)
    };

    rsx! {
        div { class: "flex h-dvh flex-col bg-surface",
            div { class: "relative h-40 w-full shrink-0 overflow-hidden bg-tile",
                if let Some(exercise) = current.as_ref() {
                    if media::animation(exercise.exercise_id).is_some() {
                        img {
                            src: "/media/gifs/{exercise.exercise_id}.webp",
                            alt: "",
                            class: "h-full w-full object-cover opacity-60",
                        }
                    } else if let Some(art) = art_for(exercise.muscle_main.as_deref()) {
                        img { src: art, alt: "", class: "h-full w-full object-cover opacity-30" }
                    }
                }
                button {
                    class: "absolute start-5 top-5 grid size-11 place-items-center rounded-xl bg-white/10 text-lead",
                    "aria-label": i18n.t("common.labels.back"),
                    onclick: move |_| {
                        navigator.go_back();
                    },
                    BackIcon { size: 22 }
                }
                div { class: "absolute inset-x-0 bottom-0 h-10 bg-linear-to-t from-surface to-transparent" }
            }
            div { class: "flex gap-1 px-5",
                for (step, done) in done_exercises.iter().enumerate() {
                    button {
                        key: "{step}",
                        class: if step == current_index {
                            "h-1.5 flex-1 rounded-full bg-accent"
                        } else if *done {
                            "h-1.5 flex-1 rounded-full bg-green-500"
                        } else {
                            "h-1.5 flex-1 rounded-full bg-line"
                        },
                        "aria-label": i18n.ta(
                            "session.exerciseStep",
                            &[("number", &step.saturating_add(1).to_string())],
                        ),
                        onclick: move |_| {
                            index.set(step);
                            selected.set(None);
                        },
                    }
                }
            }
            if let Some(exercise) = current.as_ref() {
                for key in [current_index] {
                    div {
                        key: "{key}",
                        class: "min-h-0 flex-1 overflow-y-auto px-5 pt-5 reveal-enter",
                        style: "touch-action: pan-y",
                        onpointerdown: move |event: PointerEvent| {
                            drag_from.set(Some(event.data().client_coordinates().x));
                        },
                        onpointerup: move |event: PointerEvent| {
                            let Some(from) = drag_from.cloned() else {
                                return;
                            };
                            drag_from.set(None);
                            let delta = event.data().client_coordinates().x - from;
                            if delta <= -SWIPE {
                                if last_index {
                                    finish(navigator, session.id, logged_sets, complete);
                                } else {
                                    index.set(current_index.saturating_add(1));
                                    selected.set(None);
                                }
                            } else if delta >= SWIPE && current_index > 0 {
                                index.set(current_index.saturating_sub(1));
                                selected.set(None);
                            }
                        },
                        onpointercancel: move |_| drag_from.set(None),
                        div { class: "flex items-baseline gap-3",
                            h1 { class: "min-w-0 flex-1 truncate text-[24px] font-bold tracking-tight",
                                "{exercise.name}"
                            }
                            div { class: "shrink-0 text-sm text-muted",
                                "{i18n.t(\"session.oneRm\")} "
                                span { class: "font-semibold text-lilac", "{orm}" }
                            }
                        }
                        div { class: "mt-3 flex flex-wrap items-center gap-2",
                            div { class: "flex items-center gap-2 rounded-xl bg-tile px-4 py-2.5 text-sm",
                                Icon { icon: LdTimer, width: 16, height: 16, class: "text-muted" }
                                "{i18n.t(\"session.restTimer\")}"
                                span { class: "font-semibold text-sky",
                                    "{clock(exercise.rest_seconds)}"
                                }
                            }
                            if let Some(block) = block.cloned() {
                                div { class: "flex items-center gap-2 rounded-xl bg-tile px-4 py-2.5 text-sm",
                                    if block.deload() {
                                        "{i18n.t(\"session.deload\")}"
                                    } else {
                                        "{i18n.ta(\"session.blockWeek\", &[(\"week\", &block.week.to_string()), (\"weeks\", &block.weeks.to_string())])}"
                                        span { class: "font-semibold text-lilac",
                                            "{i18n.ta(\"session.rir\", &[(\"rir\", &block.rir().to_string())])}"
                                        }
                                    }
                                }
                            }
                            if exercise.superset.is_some() {
                                div { class: "flex items-center gap-2 rounded-xl bg-mango/15 px-4 py-2.5 text-sm text-mango",
                                    Icon { icon: LdLink2, width: 16, height: 16 }
                                    "{i18n.t(\"session.superset\")}"
                                }
                            }
                            button {
                                class: "ms-auto grid size-10 shrink-0 place-items-center rounded-xl bg-tile text-muted",
                                "aria-label": i18n.t("session.history"),
                                onclick: {
                                    let id = exercise.exercise_id;
                                    move |_| {
                                        navigator.push(Route::ExerciseDetailTab {
                                            id,
                                            tab: "history".to_string(),
                                        });
                                    }
                                },
                                Icon { icon: LdSquareKanban, width: 18, height: 18 }
                            }
                        }
                        SetTable {
                            exercise: exercise.clone(),
                            selected: selected_index,
                            lb,
                            onfield: move |(set_index, field, value): (usize, Field, String)| {
                                set_field(current_index, set_index, field, value);
                            },
                            onselect: move |set_index: usize| selected.set(Some(set_index)),
                            onadd: move |()| add_set(()),
                            onremove: move |()| remove_set(()),
                        }
                    }
                }
            } else {
                div { class: "flex flex-1 items-center justify-center px-6",
                    p { class: "text-center text-base text-muted",
                        "{i18n.t(\"session.noExercises\")}"
                    }
                }
            }
            div { class: "shrink-0",
                if let Some(deadline) = rest.cloned() {
                    RestBand {
                        deadline,
                        onadjust: move |delta: i64| {
                            save_rest(session.id, rest, Some(deadline.saturating_add(delta).max(db::now())));
                        },
                    }
                }
                div { class: "flex items-center gap-2 px-5 pt-3 pb-[calc(env(safe-area-inset-bottom)+1rem)]",
                    button {
                        class: if rest.cloned().is_some() {
                            "h-14 min-w-0 flex-1 rounded-2xl bg-tile text-base font-semibold text-lilac"
                        } else if enabled {
                            "h-14 min-w-0 flex-1 rounded-2xl bg-lilac text-base font-semibold text-black"
                        } else {
                            "h-14 min-w-0 flex-1 rounded-2xl bg-tile text-base font-semibold text-muted"
                        },
                        disabled: !enabled,
                        onclick: move |_| {
                            if rest.cloned().is_some() {
                                skip_rest(());
                            } else if all_logged && last_index {
                                finish(navigator, session.id, logged_sets, complete);
                            } else {
                                log_selected(());
                            }
                        },
                        "{action}"
                    }
                    if current.is_some() && !all_logged {
                        button {
                            class: "h-14 shrink-0 rounded-2xl bg-tile px-4 text-sm font-semibold text-sky",
                            onclick: move |_| skip_exercise(()),
                            "{i18n.t(\"session.skipExercise\")}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SetTable(
    exercise: ExerciseState,
    selected: usize,
    lb: bool,
    onfield: EventHandler<(usize, Field, String)>,
    onselect: EventHandler<usize>,
    onadd: EventHandler<()>,
    onremove: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let columns = columns(&exercise.log_type);
    let fields = columns
        .iter()
        .map(|column| column_field(*column))
        .collect::<Vec<_>>();
    let log_type = exercise.log_type;
    let preview = exercise
        .sets
        .last()
        .map(|set| input_cells(&log_type, &set.inputs.with_fallback(&set.defaults)));

    rsx! {
        div { class: "mt-4 px-2",
            div { class: "flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted",
                span { class: "w-8 shrink-0", "{i18n.t(\"common.labels.set\")}" }
                for (index, column) in columns.iter().enumerate() {
                    span { key: "{index}", class: "flex-1 text-center",
                        "{column_label(&i18n, *column, lb)}"
                    }
                }
                span { class: "w-9 shrink-0" }
            }
            if exercise.sets.is_empty() {
                div { class: "flex items-center gap-2 pt-3",
                    button {
                        class: "grid w-8 shrink-0 place-items-center py-2 text-lilac",
                        "aria-label": i18n.t("session.addSet"),
                        onclick: move |_| onadd.call(()),
                        Icon { icon: LdPlus, width: 18, height: 18 }
                    }
                    span { class: "flex-1 text-center text-sm text-muted", "{i18n.t(\"session.noSetsPlanned\")}" }
                }
            }
            for (set_index, set) in exercise.sets.iter().enumerate() {
                SetRow {
                    key: "{set.number}",
                    number: set.number,
                    fields: fields.clone(),
                    inputs: set.inputs.clone(),
                    defaults: set.defaults.clone(),
                    suggested: if set.done {
                        None
                    } else {
                        set.suggested
                            .map(|target| suggestion_cells(&log_type, target, lb))
                    },
                    current: selected == set_index,
                    done: set.done,
                    onfield: move |(field, value): (Field, String)| {
                        onfield.call((set_index, field, value));
                    },
                    onselect: move |()| onselect.call(set_index),
                }
            }
            if let Some(cells) = preview {
                div { class: "mt-1 flex items-center gap-2 pt-2",
                    button {
                        class: "grid w-8 shrink-0 place-items-center py-2 text-lilac",
                        "aria-label": i18n.t("session.addSet"),
                        onclick: move |_| onadd.call(()),
                        Icon { icon: LdPlus, width: 18, height: 18 }
                    }
                    for (index, cell) in cells.iter().enumerate() {
                        span { key: "{index}", class: "flex-1 text-center text-sm text-muted",
                            "{cell}"
                        }
                    }
                    button {
                        class: "grid w-9 shrink-0 place-items-center py-2 text-muted",
                        "aria-label": i18n.t("session.deleteLastSet"),
                        onclick: move |_| onremove.call(()),
                        Icon { icon: LdTrash2, width: 16, height: 16 }
                    }
                }
            }
        }
    }
}

#[component]
fn SetRow(
    number: i64,
    fields: Vec<Field>,
    inputs: Inputs,
    defaults: Inputs,
    suggested: Option<Vec<String>>,
    current: bool,
    done: bool,
    onfield: EventHandler<(Field, String)>,
    onselect: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: if current {
                "rounded-xl bg-tile/50 px-2 py-1"
            } else {
                "py-1"
            },
            onclick: move |_| onselect.call(()),
            div { class: "flex items-center gap-2",
                span { class: "w-8 shrink-0 text-sm text-muted", "{number}" }
                for (index, field) in fields.iter().copied().enumerate() {
                    FieldInput {
                        key: "{index}",
                        value: inputs.field(field).clone(),
                        placeholder: defaults.field(field).clone(),
                        active: current,
                        onvalue: move |value: String| onfield.call((field, value)),
                    }
                }
                span {
                    class: if done {
                        "grid w-9 shrink-0 place-items-center text-green-400"
                    } else {
                        "grid w-9 shrink-0 place-items-center text-muted/60"
                    },
                    Icon { icon: LdCheck, width: 20, height: 20 }
                }
            }
            if current {
                if let Some(cells) = suggested {
                    div { class: "flex items-center gap-2 pb-0.5",
                        span { class: "w-8 shrink-0" }
                        for (index, cell) in cells.iter().enumerate() {
                            span { key: "{index}", class: "flex flex-1 items-center justify-center gap-1 text-[11px] font-medium text-accent-ink/80",
                                if !cell.is_empty() {
                                    Icon { icon: LdTarget, width: 11, height: 11 }
                                }
                                "{cell}"
                            }
                        }
                        span { class: "w-9 shrink-0" }
                    }
                }
            }
        }
    }
}

#[component]
fn FieldInput(
    value: String,
    placeholder: String,
    active: bool,
    onvalue: EventHandler<String>,
) -> Element {
    rsx! {
        input {
            class: if active {
                "h-11 min-w-0 flex-1 rounded-lg bg-surface px-2 text-center text-[15px] font-semibold text-mango underline decoration-mango/60 underline-offset-4 outline-none placeholder:text-muted"
            } else {
                "h-11 min-w-0 flex-1 rounded-lg bg-transparent px-2 text-center text-[15px] font-medium text-lead outline-none placeholder:text-muted"
            },
            r#type: "number",
            inputmode: "decimal",
            value: "{value}",
            placeholder: "{placeholder}",
            oninput: move |event| onvalue.call(event.value()),
            onpointerdown: move |event: PointerEvent| event.stop_propagation(),
        }
    }
}

#[component]
fn RestBand(deadline: i64, onadjust: EventHandler<i64>) -> Element {
    let i18n = use_i18n();
    let mut now = use_signal(db::now);
    use_future(move || async move {
        loop {
            futures_timer::Delay::new(Duration::from_millis(500)).await;
            now.set(db::now());
        }
    });
    let remaining = deadline.saturating_sub(now());
    rsx! {
        div { class: "flex items-center justify-between gap-2 rounded-t-2xl bg-green-500 px-5 py-2 text-black reveal-enter",
            button {
                class: "h-11 rounded-xl px-3 text-sm font-semibold",
                onclick: move |_| onadjust.call(-10),
                "−10s"
            }
            div { class: "flex items-baseline gap-1.5",
                span { class: "text-4xl font-bold tabular-nums", "{clock(remaining)}" }
                span { class: "text-sm", "{i18n.t(\"session.rest\")}" }
            }
            button {
                class: "h-11 rounded-xl px-3 text-sm font-semibold",
                onclick: move |_| onadjust.call(10),
                "+10s"
            }
        }
    }
}

#[component]
pub fn SessionSummary() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let session = db::active_session().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("active session failed: {err:#}");
        None
    });
    let Some(session) = session else {
        return rsx! {
            div { class: "flex min-h-dvh flex-col items-center justify-center gap-4 px-6 text-center",
                p { class: "text-base text-muted", "{i18n.t(\"session.noWorkout\")}" }
                Link {
                    to: Route::Workout {},
                    class: "text-sm font-semibold text-accent-ink",
                    "{i18n.t(\"session.backToWorkout\")}"
                }
            }
        };
    };
    rsx! {
        CompleteScreen {
            session,
            onback: move |()| {
                navigator.go_back();
            },
        }
    }
}

#[component]
fn CompleteScreen(session: ActiveSession, onback: EventHandler<()>) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let lb = db::unit_lb();
    let rows = db::session_exercises(&session).unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("session exercises failed: {err:#}");
        Vec::new()
    });
    let duration = db::now().saturating_sub(session.started_at);
    let volume = rows
        .iter()
        .flat_map(|exercise| exercise.logged.iter())
        .filter_map(|set| match (set.weight, set.reps) {
            (Some(weight), Some(reps)) => Some((weight, reps)),
            _ => None,
        })
        .map(|(weight, reps)| weight * f64::from(i32::try_from(reps).unwrap_or(0)))
        .sum::<f64>();
    let records = rows
        .iter()
        .filter(|exercise| {
            exercise
                .best_1rm
                .is_some_and(|best| exercise.prior_1rm.is_none_or(|prior| best > prior))
        })
        .count();

    rsx! {
        div { class: "flex h-dvh flex-col bg-surface",
            header { class: "flex items-center gap-2 px-5 pt-5",
                button {
                    class: "grid size-10 place-items-center text-muted",
                    "aria-label": i18n.t("common.labels.back"),
                    onclick: move |_| onback.call(()),
                    BackIcon { size: 24 }
                }
            }
            div { class: "min-h-0 flex-1 overflow-y-auto px-5",
                h1 { class: "mt-2 text-[28px] font-bold tracking-tight", "{i18n.t(\"session.complete\")}" }
                span { class: "mt-2 block truncate text-base text-sky", "{session.name}" }
                div { class: "mt-6 flex justify-between",
                    Stat { label: i18n.t("session.duration"), value: long_clock(duration) }
                    Stat {
                        label: i18n.t("session.volume"),
                        value: format!(
                            "{} {}",
                            volume_text(display_weight(volume, lb)),
                            weight_unit(lb),
                        ),
                    }
                    Stat { label: i18n.t("session.records"), value: records.to_string() }
                }
                for exercise in rows.iter() {
                    div { key: "{exercise.id}", class: "mt-8",
                        div { class: "flex items-center gap-3",
                            ExerciseThumb { id: exercise.exercise_id, size: 56 }
                            div { class: "min-w-0 flex-1 text-base font-semibold", "{exercise.name}" }
                        }
                        div { class: "mt-3 flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted",
                            span { class: "w-8 shrink-0", "{i18n.t(\"common.labels.set\")}" }
                            for (index, column) in columns(&exercise.log_type).iter().enumerate() {
                                span { key: "{index}", class: "flex-1 text-center",
                                    "{pretty_column(&i18n, *column)}"
                                }
                            }
                            span { class: "w-20 shrink-0 text-end", "{i18n.t(\"common.labels.oneRm\")}" }
                        }
                        for set in exercise.logged.iter() {
                            SummarySetRow {
                                key: "{set.set_number}",
                                number: set.set_number,
                                cells: logged_cells(&exercise.log_type, set, lb),
                                orm: set_orm(set),
                                best: exercise.best_1rm,
                                lb,
                            }
                        }
                    }
                }
                div { class: "h-6" }
            }
            div { class: "flex shrink-0 items-center gap-3 px-5 pt-3 pb-[calc(env(safe-area-inset-bottom)+1rem)]",
                DeleteButton {
                    size: 22,
                    class: "grid size-14 shrink-0 place-items-center rounded-2xl bg-tile text-muted",
                    label: i18n.t("session.discard"),
                    onconfirm: move |()| match db::discard_session(session.id) {
                        Ok(()) => {
                            navigator.replace(Route::Workout {});
                        }
                        Err(err) => dioxus::logger::tracing::error!("discard failed: {err:#}"),
                    },
                }
                button {
                    class: "h-14 flex-1 rounded-2xl bg-lilac text-base font-semibold text-black",
                    onclick: move |_| {
                        match db::finish_session(session.id) {
                            Ok(()) => {
                                navigator.replace(Route::Workout {});
                            }
                            Err(err) => {
                                dioxus::logger::tracing::error!("finish session failed: {err:#}");
                            }
                        }
                    },
                    "{i18n.t(\"session.endSave\")}"
                }
            }
        }
    }
}

#[component]
fn SummarySetRow(
    number: i64,
    cells: Vec<String>,
    orm: Option<f64>,
    best: Option<f64>,
    lb: bool,
) -> Element {
    let value = orm.map_or_else(String::new, |value| {
        format!(
            "{} {}",
            orm_text(display_weight(value, lb)),
            weight_unit(lb)
        )
    });
    let highlight = orm.is_some_and(|value| Some(value) == best);
    rsx! {
        div { class: "flex items-center gap-2 py-2",
            span { class: "w-8 shrink-0 text-sm text-muted", "{number}" }
            for (index, cell) in cells.iter().enumerate() {
                span { key: "{index}", class: "flex-1 text-center text-sm text-lead", "{cell}" }
            }
            span {
                class: if highlight {
                    "w-20 shrink-0 text-end text-sm font-semibold text-mango"
                } else {
                    "w-20 shrink-0 text-end text-sm text-muted"
                },
                "{value}"
            }
        }
    }
}

#[component]
fn Stat(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex-1 text-center",
            div { class: "text-sm text-muted", "{label}" }
            div { class: "mt-1 text-xl font-semibold", "{value}" }
        }
    }
}

const SWIPE: f64 = 60.0;

fn finish(navigator: Navigator, session_id: i64, logged_sets: usize, mut complete: Signal<bool>) {
    if logged_sets == 0 {
        match db::finish_session(session_id) {
            Ok(()) => {
                navigator.replace(Route::Workout {});
            }
            Err(err) => dioxus::logger::tracing::error!("finish session failed: {err:#}"),
        }
    } else {
        complete.set(true);
    }
}

fn advance(
    states: Signal<Vec<ExerciseState>>,
    mut index: Signal<usize>,
    mut selected: Signal<Option<usize>>,
) {
    let exercises = states.cloned();
    let current = index();
    let Some(exercise) = exercises.get(current) else {
        return;
    };
    if exercise.sets.is_empty() || exercise.sets.iter().any(|set| !set.done) {
        return;
    }
    if current.saturating_add(1) < exercises.len() {
        index.set(current.saturating_add(1));
        selected.set(None);
    }
}

fn advance_after_rest(
    states: Signal<Vec<ExerciseState>>,
    index: Signal<usize>,
    selected: Signal<Option<usize>>,
) {
    let superset = states
        .cloned()
        .get(index())
        .is_some_and(|exercise| exercise.superset.is_some());
    if !superset {
        advance(states, index, selected);
    }
}

fn advance_superset(
    states: Signal<Vec<ExerciseState>>,
    mut index: Signal<usize>,
    mut selected: Signal<Option<usize>>,
) {
    let exercises = states.cloned();
    if exercises.is_empty() {
        return;
    }
    let current = index();
    let Some(group) = exercises
        .get(current)
        .and_then(|exercise| exercise.superset)
    else {
        return;
    };
    let total = exercises.len();
    let next = (1..total)
        .map(|step| current.saturating_add(step).checked_rem(total).unwrap_or(0))
        .find(|candidate| {
            exercises.get(*candidate).is_some_and(|exercise| {
                exercise.superset == Some(group)
                    && !exercise.sets.is_empty()
                    && exercise.sets.iter().any(|set| !set.done)
            })
        });
    if let Some(next) = next {
        index.set(next);
        selected.set(None);
        return;
    }
    let last = (0..total).rev().find(|candidate| {
        exercises
            .get(*candidate)
            .is_some_and(|exercise| exercise.superset == Some(group))
    });
    if let Some(last) = last.filter(|last| last.saturating_add(1) < total) {
        index.set(last.saturating_add(1));
        selected.set(None);
    }
}

fn save_rest(session_id: i64, mut rest: Signal<Option<i64>>, deadline: Option<i64>) {
    rest.set(deadline);
    if let Err(err) = db::set_rest_until(session_id, deadline) {
        dioxus::logger::tracing::error!("rest timer failed: {err:#}");
    }
}

fn start_index(session: &ActiveSession) -> usize {
    db::session_exercises(session).map_or(0, |rows| db::current_position(&rows))
}

fn load(session: &ActiveSession, lb: bool, block: Option<Block>) -> Vec<ExerciseState> {
    let rows = db::session_exercises(session).unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("session exercises failed: {err:#}");
        Vec::new()
    });
    let goal = Goal::from_routine(
        db::routine_goal(session.routine_id)
            .unwrap_or_else(|err| {
                dioxus::logger::tracing::error!("routine goal failed: {err:#}");
                None
            })
            .as_deref(),
    );
    let counts = block.map_or_else(Vec::new, |block| {
        db::muscle_sets_between(block.pass_start, db::now()).unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("muscle sets failed: {err:#}");
            Vec::new()
        })
    });
    let mut topped: Vec<String> = Vec::new();
    rows.into_iter()
        .map(|row| {
            let extra = extra_set(
                block,
                &counts,
                row.muscle_main.as_deref(),
                row.logged.len(),
                row.targets.len(),
                !row.removed.is_empty(),
                &mut topped,
            );
            exercise_state(row, lb, session.id, goal, block, extra)
        })
        .collect()
}

fn extra_set(
    block: Option<Block>,
    counts: &[(String, i64)],
    muscle: Option<&str>,
    logged: usize,
    planned: usize,
    removed: bool,
    topped: &mut Vec<String>,
) -> bool {
    let (Some(block), Some(muscle)) = (block, muscle) else {
        return false;
    };
    if block.deload() || topped.iter().any(|done| done == muscle) {
        return false;
    }
    if removed {
        topped.push(muscle.to_string());
        return false;
    }
    if logged > planned {
        return false;
    }
    let done = counts
        .iter()
        .find(|(name, _)| name == muscle)
        .map_or(0, |(_, count)| *count);
    if done >= block.muscle_sets() {
        return false;
    }
    topped.push(muscle.to_string());
    true
}

fn exercise_state(
    row: SessionExercise,
    lb: bool,
    session_id: i64,
    goal: Goal,
    block: Option<Block>,
    extra: bool,
) -> ExerciseState {
    let cardio = row.log_type == "cardio";
    let mut targets = row.targets;
    if block.is_some_and(Block::deload) {
        let keep = targets.len().saturating_add(1) / 2;
        targets.truncate(keep);
    }
    let mut numbers = targets
        .iter()
        .map(|set| set.set_number)
        .chain(row.logged.iter().map(|set| set.set_number))
        .collect::<Vec<_>>();
    numbers.sort_unstable();
    numbers.dedup();
    if extra {
        numbers.push(numbers.last().copied().unwrap_or(0).saturating_add(1));
    }
    numbers.retain(|number| !row.removed.contains(number));
    let history = db::exercise_history(row.exercise_id, session_id, 4).unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("exercise history failed: {err:#}");
        Vec::new()
    });
    let history = match block {
        Some(block) if block.deload() => history
            .into_iter()
            .filter(|session| session.started_at < block.pass_start)
            .collect(),
        _ => history,
    };
    let suggestions = suggest::targets(&numbers, &targets, &history, goal, block);
    let sets = numbers
        .into_iter()
        .enumerate()
        .map(|(index, number)| {
            let logged = row.logged.iter().find(|set| set.set_number == number);
            let target = targets
                .iter()
                .find(|set| set.set_number == number)
                .or_else(|| targets.last());
            let defaults = Inputs {
                weight: weight_field(target.and_then(|set| set.target_weight), lb),
                reps: text(target.and_then(|set| set.target_reps)),
                seconds: minute_text(target.and_then(|set| set.target_seconds), cardio),
                distance: km_text(target.and_then(|set| set.target_distance_m)),
            };
            let suggested = suggestions.get(index).copied().flatten();
            let inputs = logged.map_or_else(Inputs::default, |set| Inputs {
                weight: weight_field(set.weight, lb),
                reps: text(set.reps),
                seconds: minute_text(set.seconds, cardio),
                distance: km_text(set.distance_m),
            });
            SetState {
                number,
                inputs,
                defaults,
                suggested,
                done: logged.is_some(),
            }
        })
        .collect();
    ExerciseState {
        id: row.id,
        exercise_id: row.exercise_id,
        name: row.name,
        muscle_main: row.muscle_main,
        log_type: row.log_type,
        rest_seconds: row.rest_seconds,
        superset: row.superset,
        best_1rm: row.best_1rm,
        prior_1rm: row.prior_1rm,
        sets,
    }
}

fn columns(log_type: &str) -> &'static [Column] {
    match log_type {
        "weight_and_reps" => &[Column::Weight, Column::Reps],
        "reps" => &[Column::Reps],
        "duration" => &[Column::Seconds],
        "reps_and_duration" => &[Column::Reps, Column::Seconds],
        "cardio" => &[Column::Minutes, Column::Km],
        _ => &[],
    }
}

fn column_label(i18n: &I18n, column: Column, lb: bool) -> String {
    match column {
        Column::Weight => i18n.unit(lb),
        Column::Reps => i18n.t("common.labels.reps"),
        Column::Seconds => i18n.t("common.labels.secs"),
        Column::Minutes => i18n.t("common.labels.min"),
        Column::Km => i18n.t("common.labels.km"),
    }
}

const fn column_field(column: Column) -> Field {
    match column {
        Column::Weight => Field::Weight,
        Column::Reps => Field::Reps,
        Column::Seconds | Column::Minutes => Field::Seconds,
        Column::Km => Field::Distance,
    }
}

fn logged_cells(log_type: &str, set: &SessionSet, lb: bool) -> Vec<String> {
    columns(log_type)
        .iter()
        .map(|column| match column {
            Column::Weight => weight_field(set.weight, lb),
            Column::Reps => text(set.reps),
            Column::Seconds => text(set.seconds),
            Column::Minutes => set
                .seconds
                .map_or_else(String::new, |seconds| (seconds / 60).to_string()),
            Column::Km => set
                .distance_m
                .map_or_else(String::new, |meters| (meters / 1000.0).to_string()),
        })
        .collect()
}

fn suggestion_cells(log_type: &str, target: suggest::Target, lb: bool) -> Vec<String> {
    columns(log_type)
        .iter()
        .map(|column| match column {
            Column::Weight => target
                .weight
                .map_or_else(String::new, |weight| weight_field(Some(weight), lb)),
            Column::Reps => target
                .reps
                .map_or_else(String::new, |reps| reps.to_string()),
            Column::Seconds | Column::Minutes | Column::Km => String::new(),
        })
        .collect()
}

fn input_cells(log_type: &str, inputs: &Inputs) -> Vec<String> {
    columns(log_type)
        .iter()
        .map(|column| match column {
            Column::Weight => inputs.weight.clone(),
            Column::Reps => inputs.reps.clone(),
            Column::Seconds | Column::Minutes => inputs.seconds.clone(),
            Column::Km => inputs.distance.clone(),
        })
        .collect()
}

fn pretty_column(i18n: &I18n, column: Column) -> String {
    match column {
        Column::Weight => i18n.t("common.labels.weight"),
        Column::Reps => i18n.t("common.labels.reps"),
        Column::Seconds => i18n.t("common.labels.secs"),
        Column::Minutes => i18n.t("common.labels.min"),
        Column::Km => i18n.t("common.labels.km"),
    }
}

fn set_orm(set: &SessionSet) -> Option<f64> {
    let weight = set.weight?;
    let reps = set.reps?;
    if weight <= 0.0 || !(1..=30).contains(&reps) {
        return None;
    }
    Some(weight * (1.0 + f64::from(i32::try_from(reps).unwrap_or(0)) / 30.0))
}

fn values(
    log_type: &str,
    inputs: &Inputs,
    lb: bool,
) -> (Option<f64>, Option<i64>, Option<i64>, Option<f64>) {
    let weight = parse_weight(&inputs.weight, lb);
    let reps = inputs.reps.trim().parse::<i64>().ok();
    let seconds = inputs.seconds.trim().parse::<i64>().ok();
    let distance = inputs.distance.trim().parse::<f64>().ok();
    match log_type {
        "weight_and_reps" => (weight, reps, None, None),
        "reps" => (None, reps, None, None),
        "duration" => (None, None, seconds, None),
        "reps_and_duration" => (None, reps, seconds, None),
        "cardio" => (
            None,
            None,
            seconds.map(|minutes| minutes.saturating_mul(60)),
            distance.map(|km| km * 1000.0),
        ),
        _ => (None, None, None, None),
    }
}

fn ready(log_type: &str, inputs: &Inputs) -> bool {
    match log_type {
        "weight_and_reps" | "reps" => !inputs.reps.trim().is_empty(),
        "reps_and_duration" => !inputs.reps.trim().is_empty() && !inputs.seconds.trim().is_empty(),
        "duration" | "cardio" => !inputs.seconds.trim().is_empty(),
        _ => true,
    }
}

fn text(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn pick<'a>(value: &'a str, fallback: &'a str) -> &'a str {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.trim()
    } else {
        trimmed
    }
}

fn minute_text(seconds: Option<i64>, cardio: bool) -> String {
    seconds.map_or_else(String::new, |value| {
        if cardio {
            (value / 60).to_string()
        } else {
            value.to_string()
        }
    })
}

fn km_text(distance_m: Option<f64>) -> String {
    distance_m.map_or_else(String::new, |value| (value / 1000.0).to_string())
}

fn clock(seconds: i64) -> String {
    let minutes = seconds / 60;
    let rest = seconds % 60;
    format!("{minutes:02}:{rest:02}")
}

fn long_clock(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    let rest = seconds % 60;
    format!("{hours}:{minutes:02}:{rest:02}")
}

fn orm_text(value: f64) -> String {
    let text = format!("{:.2}", (value * 4.0).round() / 4.0);
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[component]
pub(crate) fn Elapsed(started_at: i64) -> Element {
    let mut now = use_signal(db::now);
    use_future(move || async move {
        loop {
            futures_timer::Delay::new(Duration::from_secs(1)).await;
            now.set(db::now());
        }
    });
    let elapsed = now().saturating_sub(started_at);
    rsx! {
        span { class: "font-semibold tabular-nums text-sky", "{long_clock(elapsed)}" }
    }
}
