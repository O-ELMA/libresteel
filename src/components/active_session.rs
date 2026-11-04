use std::time::Duration;

use dioxus::prelude::*;

use crate::{components::shell::Route, components::ui::long_clock, db, i18n::use_i18n};

#[component]
pub fn ActiveSessionPill() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let Some(active) = db::active_session().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("active session failed: {err:#}");
        None
    }) else {
        return rsx! {};
    };
    let name = exercise_name(&active);
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
pub fn Elapsed(started_at: i64) -> Element {
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

pub fn current_row(active: &db::ActiveSession) -> Option<db::SessionExercise> {
    let rows = match db::session_exercises(active) {
        Ok(rows) => rows,
        Err(err) => {
            dioxus::logger::tracing::error!("session exercises failed: {err:#}");
            return None;
        }
    };
    rows.get(db::current_position(&rows)).cloned()
}

pub fn exercise_name(active: &db::ActiveSession) -> String {
    current_row(active).map_or_else(|| active.name.clone(), |row| row.name)
}
