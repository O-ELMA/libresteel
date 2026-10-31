use chrono::{Days, Local, NaiveDate, NaiveTime, TimeZone};
use dioxus::prelude::*;

use crate::{
    calendar::{week_start, CalendarCard},
    db,
    exercises::ExerciseThumb,
    i18n::{use_i18n, I18n},
    session::ActiveSessionPill,
    shell::Route,
    ui::{change, duration_label, weight_text, Delta, ForwardIcon},
};

const SEGMENTS: [&str; 4] = ["bg-mango", "bg-lime", "bg-lilac", "bg-sky"];
const OTHER: &str = "bg-[#787f9a]";

#[component]
pub fn Overview() -> Element {
    let i18n = use_i18n();
    let today = Local::now().date_naive();
    let first_week_day = db::first_week_day();
    let this_week = week_start(today, first_week_day);
    let last_week = this_week
        .checked_sub_days(Days::new(7))
        .unwrap_or(this_week);
    let next_week = this_week
        .checked_add_days(Days::new(7))
        .unwrap_or(this_week);
    let week_start_at = midnight(this_week);
    let week_end_at = midnight(next_week);
    let last_start_at = midnight(last_week);
    let since = today.checked_sub_days(Days::new(90)).unwrap_or(today);

    let sets = quiet(db::sets_between(week_start_at, week_end_at));
    let last_sets = quiet(db::sets_between(last_start_at, week_start_at));
    let seconds = quiet(db::workout_seconds_between(week_start_at, week_end_at));
    let last_seconds = quiet(db::workout_seconds_between(last_start_at, week_start_at));
    let breakdown = groups(
        quiet(db::muscle_sets_between(week_start_at, week_end_at)),
        &i18n,
    );
    let lifts = quiet(db::best_1rm_since(midnight(since)));
    let lb = db::unit_lb();

    rsx! {
        div { class: "px-5",
            OverviewTabs {}
            CalendarCard {}
            div { class: "mt-5 grid grid-cols-2 gap-3",
                Metric {
                    to: Route::VolumeProgress {},
                    title: i18n.t("progress.sets"),
                    value: sets.to_string(),
                    delta: change(sets_of(sets), sets_of(last_sets)),
                    tone: "bg-mango",
                }
                Metric {
                    to: Route::TimeProgress {},
                    title: i18n.t("overview.workoutTime"),
                    value: duration_label(seconds),
                    delta: change(seconds_of(seconds), seconds_of(last_seconds)),
                    tone: "bg-sky",
                }
            }
            Link {
                to: Route::MuscleProgress {},
                class: "mt-3 block rounded-2xl bg-tile p-4",
                div { class: "flex items-center justify-between",
                    h2 { class: "text-sm font-semibold", "{i18n.t(\"overview.muscleBreakdown\")}" }
                    ForwardIcon { size: 18 }
                }
                if breakdown.is_empty() {
                    p { class: "mt-4 text-sm text-muted", "{i18n.t(\"overview.noSets\")}" }
                } else {
                    div { class: "mt-4 flex h-3 overflow-hidden rounded-full",
                        for (index, (label, count)) in breakdown.iter().enumerate() {
                            div {
                                key: "{label}",
                                class: segment(index),
                                style: "flex: {count}",
                            }
                        }
                    }
                    div { class: "mt-3 flex flex-wrap gap-x-4 gap-y-2",
                        for (index, (label, _)) in breakdown.iter().enumerate() {
                            div { key: "{label}", class: "flex items-center gap-1.5 text-xs",
                                span { class: "size-2.5 rounded-full {segment(index)}" }
                                "{label}"
                            }
                        }
                    }
                    div { class: "mt-2 text-sm text-muted", "{i18n.t(\"overview.thisWeek\")}" }
                }
            }
            div { class: "mt-3 rounded-2xl border border-line px-4 py-4",
                h2 { class: "text-sm font-semibold", "{i18n.t(\"common.labels.oneRm\")}" }
                if lifts.is_empty() {
                    p { class: "mt-3 text-sm text-muted", "{i18n.t(\"overview.noLifts\")}" }
                } else {
                    for (index, (id, name, value)) in lifts.iter().enumerate() {
                        Link {
                            key: "{id}",
                            to: Route::ExerciseDetailTab { id: *id, tab: "chart".to_string() },
                            class: if index == 0 {
                                "mt-1 flex items-center gap-2 py-3.5"
                            } else {
                                "flex items-center gap-2 border-t border-line py-3.5"
                            },
                            ExerciseThumb { id: *id, size: 40 }
                            span { class: "min-w-0 flex-1 truncate text-[15px] font-medium", "{name}" }
                            span { class: "flex shrink-0 items-center gap-1 text-lilac",                                span { class: "text-[15px] font-semibold", "{weight_text(*value, lb)}" }
                                ForwardIcon { size: 18 }
                            }
                        }
                    }
                }
            }
            div { class: "h-8" }
        }
        ActiveSessionPill {}
    }
}

#[component]
pub fn OverviewTabs() -> Element {
    let i18n = use_i18n();
    let body = matches!(use_route::<Route>(), Route::Body {});
    rsx! {
        div { class: "sticky top-0 z-20 -mx-5 flex bg-surface px-5 pt-5",
            OverviewTab {
                to: Route::Overview {},
                label: i18n.t("overview.workoutTab"),
                active: !body,
            }
            OverviewTab {
                to: Route::Body {},
                label: i18n.t("overview.bodyTab"),
                active: body,
            }
        }
    }
}

#[component]
fn OverviewTab(to: Route, label: String, active: bool) -> Element {
    rsx! {
        Link {
            to,
            class: if active {
                "flex-1 border-b-[3px] border-accent pt-1 pb-3 text-center text-[22px] font-semibold tracking-tight text-accent-ink"
            } else {
                "flex-1 border-b-[3px] border-transparent pt-1 pb-3 text-center text-[22px] font-semibold tracking-tight text-muted"
            },
            "{label}"
        }
    }
}

#[component]
fn Metric(to: Route, title: String, value: String, delta: Option<f64>, tone: &'static str) -> Element {
    let i18n = use_i18n();
    rsx! {
        Link { to, class: "rounded-2xl {tone} p-4 text-black",
            div { class: "flex items-center justify-between",
                span { class: "text-sm font-semibold", "{title}" }
                ForwardIcon { size: 18 }
            }
            div { class: "mt-2 flex items-baseline gap-1.5",
                span { class: "text-[21px] font-bold leading-7", "{value}" }
                if let Some(percent) = delta {
                    Delta { percent, on_light: true }
                }
            }
            div { class: "mt-1 text-sm text-black/60", "{i18n.t(\"overview.thisWeek\")}" }
        }
    }
}

pub fn quiet<T: Default>(result: anyhow::Result<T>) -> T {
    result.unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("overview query failed: {err:#}");
        T::default()
    })
}

fn groups(mut rows: Vec<(String, i64)>, i18n: &I18n) -> Vec<(String, i64)> {
    rows.retain(|(_, count)| *count > 0);
    rows = rows
        .into_iter()
        .map(|(label, count)| (i18n.value("exercises.muscles", &label), count))
        .collect();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    if rows.len() > 4 {
        let other = rows.split_off(4).iter().map(|(_, count)| *count).sum();
        rows.push((i18n.t("overview.other"), other));
    }
    rows
}

fn segment(index: usize) -> &'static str {
    SEGMENTS.get(index).copied().unwrap_or(OTHER)
}

fn seconds_of(seconds: i64) -> f64 {
    f64::from(i32::try_from(seconds).unwrap_or(i32::MAX))
}

fn sets_of(sets: i64) -> f64 {
    f64::from(i32::try_from(sets).unwrap_or(i32::MAX))
}

fn midnight(day: NaiveDate) -> i64 {
    Local
        .from_local_datetime(&day.and_time(NaiveTime::MIN))
        .earliest()
        .map_or(0, |moment| moment.timestamp())
}
