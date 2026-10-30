use std::collections::HashMap;

use chrono::{Datelike, Days, Local, Months, NaiveDate, TimeZone, Timelike};
use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdChevronLeft, LdChevronRight, LdSquareKanban},
    Icon,
};

use crate::{
    bodymap::BodyMap,
    db::{self, ActiveSession, SessionSet},
    exercises::{thousands, ExerciseThumb},
    i18n::{use_i18n, I18n},
    shell::Route,
    ui::{display_weight, weight_field, weight_text, weight_unit, BackIcon},
};

#[derive(Clone, Copy, PartialEq)]
enum Field {
    Weight,
    Reps,
    Seconds,
    Distance,
}

struct LogSet {
    owner: i64,
    set: SessionSet,
}

struct LogExercise {
    exercise_id: i64,
    name: String,
    muscle_main: Option<String>,
    log_type: String,
    best_1rm: Option<f64>,
    sets: Vec<LogSet>,
}

#[component]
pub fn CalendarCard() -> Element {
    let i18n = use_i18n();
    let today = Local::now().date_naive();
    let mut month = use_signal(|| first_of_month(today));
    let sessions = db::finished_sessions().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("sessions failed: {err:#}");
        Vec::new()
    });
    let first = db::first_week_day();
    let days = sessions
        .iter()
        .filter_map(|session| local_date(session.started_at))
        .collect::<Vec<_>>();
    let total = thousands(sessions.len());
    let streak = streak(&days, today, first);
    let weeks = i18n.tn("common.week", streak);
    let cells = month_cells(month(), first);
    let title = format!("{} {}", i18n.month_long(month().month()), month().year());

    rsx! {
        div { class: "mt-5 rounded-3xl bg-tile/40 px-3 pt-4 pb-5",
                div { class: "flex items-center justify-between ps-2",
                    div { class: "text-lg font-semibold", "{title}" }
                    div { class: "flex items-center",
                        button {
                            class: "grid size-10 place-items-center rounded-full text-muted",
                            "aria-label": i18n.t("calendar.previousMonth"),
                            onclick: move |_| {
                                if let Some(previous) = month().checked_sub_months(Months::new(1)) {
                                    month.set(previous);
                                }
                            },
                            if i18n.lang() == "ar" {
                                Icon { icon: LdChevronRight, width: 22, height: 22 }
                            } else {
                                Icon { icon: LdChevronLeft, width: 22, height: 22 }
                            }
                        }
                        button {
                            class: "grid size-10 place-items-center rounded-full text-muted",
                            "aria-label": i18n.t("calendar.nextMonth"),
                            onclick: move |_| {
                                if let Some(next) = month().checked_add_months(Months::new(1)) {
                                    month.set(next);
                                }
                            },
                            if i18n.lang() == "ar" {
                                Icon { icon: LdChevronLeft, width: 22, height: 22 }
                            } else {
                                Icon { icon: LdChevronRight, width: 22, height: 22 }
                            }
                        }
                    }
                }
                div { class: "mt-3 grid grid-cols-7 place-items-center gap-y-1",
                    for (index, label) in day_labels(first, &i18n).iter().enumerate() {
                        span {
                            key: "{index}",
                            class: "grid h-8 place-items-center text-xs text-muted",
                            "{label}"
                        }
                    }
                    for (index, cell) in cells.iter().enumerate() {
                        if let Some(day) = cell {
                            if days.contains(day) {
                                Link {
                                    key: "{index}",
                                    to: Route::DayLog { date: day.format("%Y-%m-%d").to_string() },
                                    class: "grid size-11 place-items-center rounded-full bg-accent text-sm font-semibold text-black",
                                    "{day.day()}"
                                }
                            } else if *day == today {
                                div {
                                    key: "{index}",
                                    class: "grid size-11 place-items-center rounded-full border border-accent text-sm font-semibold text-lead",
                                    "{day.day()}"
                                }
                            } else {
                                div {
                                    key: "{index}",
                                    class: "grid size-11 place-items-center rounded-full bg-tile text-sm font-medium text-lead",
                                    "{day.day()}"
                                }
                            }
                        } else {
                            div { key: "{index}", class: "size-11" }
                        }
                    }
                }
                div { class: "mt-4 flex border-t border-line pt-4",
                    div { class: "flex-1 text-center",
                        div { class: "text-2xl font-bold", "{total}" }
                        div { class: "mt-1 text-sm text-muted", "{i18n.t(\"calendar.totalSessions\")}" }
                    }
                    div { class: "flex-1 text-center",
                        div { class: "text-2xl font-bold",
                            "{streak} "
                            span { class: "text-base font-semibold", "{weeks}" }
                        }
                        div { class: "mt-1 text-sm text-muted", "{i18n.t(\"calendar.currentStreak\")}" }
                    }
                }
            }
    }
}

#[component]
pub fn DayLog(date: String) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let Ok(day) = NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
        return rsx! {
            p { class: "p-6 text-base text-muted", "{i18n.t(\"calendar.dateNotFound\")}" }
        };
    };

    let sessions = db::finished_sessions()
        .unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("sessions failed: {err:#}");
            Vec::new()
        })
        .into_iter()
        .filter(|session| local_date(session.started_at) == Some(day))
        .collect::<Vec<_>>();
    if sessions.is_empty() {
        return rsx! {
            DayHeader { day }
            div { class: "flex flex-col items-center gap-4 px-5 py-16 text-center",
                p { class: "text-base text-muted", "{i18n.t(\"calendar.nothingLogged\")}" }
                Link {
                    to: Route::Workout {},
                    class: "rounded-xl bg-lilac px-5 py-3 text-sm font-semibold text-black",
                    "{i18n.t(\"calendar.logWorkout\")}"
                }
            }
        };
    }

    let rows = day_rows(&sessions);
    let started_at = sessions
        .iter()
        .map(|session| session.started_at)
        .min()
        .unwrap_or(0);
    let duration = sessions
        .iter()
        .map(|session| {
            session
                .ended_at
                .unwrap_or(session.started_at)
                .saturating_sub(session.started_at)
        })
        .sum::<i64>();
    let volume = rows
        .iter()
        .flat_map(|row| row.sets.iter())
        .filter_map(|set| {
            Some(set.set.weight? * f64::from(i32::try_from(set.set.reps?).unwrap_or(0)))
        })
        .sum::<f64>();
    let records = record_count(&rows);
    let counts = body_counts(&rows);
    let lb = db::unit_lb();
    let exercise_count = rows.len();
    let started = i18n.ta(
        "calendar.startedAt",
        &[("time", &time_text(&i18n, started_at))],
    );
    let volume_label = i18n.ta("calendar.volume", &[("unit", weight_unit(lb))]);
    let exercise_label = i18n.tn(
        "calendar.exercises",
        i64::try_from(exercise_count).unwrap_or(i64::MAX),
    );

    rsx! {
        DayHeader { day }
        div { class: "px-5",
            div { class: "mt-5 inline-flex rounded-full bg-tile px-4 py-2 text-sm text-lead",
                "{started}"
            }
            div { class: "mt-3 text-[34px] font-bold tracking-tight", "{duration_text(duration)}" }
            div { class: "mt-4",
                BodyMap { counts }
            }
            div { class: "mt-5 flex",
                for (value, label) in [
                    (
                        volume_text(display_weight(volume, lb)),
                        volume_label,
                    ),
                    (records.to_string(), i18n.t("calendar.records")),
                    ("N/A".to_string(), i18n.t("calendar.exertion")),
                ] {
                    div {
                        key: "{label}",
                        class: "flex-1 border-b border-line pb-3 text-center",
                        div { class: "text-xl font-bold", "{value}" }
                        div { class: "mt-1 text-sm text-muted", "{label}" }
                    }
                }
            }
            div { class: "mt-7 text-lg font-semibold", "{exercise_label}" }
            for exercise in rows.iter() {
                div { key: "{exercise.exercise_id}", class: "mt-6",
                    div { class: "flex items-center gap-3",
                        ExerciseThumb { id: exercise.exercise_id, size: 56 }
                        div { class: "min-w-0 flex-1 truncate text-base font-semibold",
                            "{exercise.name}"
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
                    div { class: "mt-3 flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted",
                        span { class: "w-8 shrink-0", "{i18n.t(\"common.labels.set\")}" }
                        for (index, field) in fields(&exercise.log_type).iter().enumerate() {
                            span {
                                key: "{index}",
                                class: "flex-1 text-center",
                                "{field_label(&i18n, *field, &exercise.log_type)}"
                            }
                        }
                        span { class: "w-20 shrink-0 text-end", "{i18n.t(\"common.labels.oneRm\")}" }
                    }
                    for (index, entry) in exercise.sets.iter().enumerate() {
                        LogSetRow {
                            key: "{entry.owner}-{entry.set.set_number}",
                            number: i64::try_from(index).unwrap_or(0).saturating_add(1),
                            set: entry.set.clone(),
                            log_type: exercise.log_type.clone(),
                            best_1rm: exercise.best_1rm,
                            lb,
                        }
                    }
                }
            }
            div { class: "h-8" }
        }
    }
}

#[component]
fn DayHeader(day: NaiveDate) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let title = format!(
        "{} {}, {}",
        i18n.month_short(day.month()),
        day.day(),
        day.year()
    );
    rsx! {
        header { class: "flex items-center gap-3 px-5 pt-5",
            button {
                class: "text-lead",
                "aria-label": i18n.t("common.labels.back"),
                onclick: move |_| navigator.go_back(),
                BackIcon { size: 24 }
            }
            h1 { class: "text-[20px] font-bold tracking-tight",
                "{title}"
            }
        }
    }
}

#[component]
fn LogSetRow(
    number: i64,
    set: SessionSet,
    log_type: String,
    best_1rm: Option<f64>,
    lb: bool,
) -> Element {
    let orm = set_orm(&set);
    let highlight = orm.is_some_and(|value| best_1rm.is_some_and(|best| close(value, best)));
    let value = orm.map_or_else(|| "—".to_string(), |value| weight_text(value, lb));

    rsx! {
        div { class: "flex items-center gap-2 py-1",
            span { class: "w-8 shrink-0 text-sm text-muted", "{number}" }
            for (index, field) in fields(&log_type).iter().copied().enumerate() {
                span {
                    key: "{index}",
                    class: "flex-1 text-center text-sm text-lead",
                    "{field_cell(field, &log_type, &set, lb)}"
                }
            }
            span {
                class: if highlight {
                    "w-20 shrink-0 text-end text-sm font-semibold text-accent-ink"
                } else {
                    "w-20 shrink-0 text-end text-sm text-muted"
                },
                "{value}"
            }
        }
    }
}

fn day_rows(sessions: &[ActiveSession]) -> Vec<LogExercise> {
    let mut rows: Vec<LogExercise> = Vec::new();
    for session in sessions {
        let exercises = db::finished_session_exercises(session).unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("session exercises failed: {err:#}");
            Vec::new()
        });
        for exercise in exercises.into_iter().filter(|row| !row.logged.is_empty()) {
            let sets = exercise
                .logged
                .into_iter()
                .map(|set| LogSet {
                    owner: exercise.id,
                    set,
                })
                .collect::<Vec<_>>();
            if let Some(row) = rows
                .iter_mut()
                .find(|row| row.exercise_id == exercise.exercise_id)
            {
                row.sets.extend(sets);
            } else {
                rows.push(LogExercise {
                    exercise_id: exercise.exercise_id,
                    name: exercise.name,
                    muscle_main: exercise.muscle_main,
                    log_type: exercise.log_type,
                    best_1rm: exercise.best_1rm,
                    sets,
                });
            }
        }
    }
    rows
}

fn record_count(rows: &[LogExercise]) -> i64 {
    let records = rows
        .iter()
        .filter(|row| {
            let day_best = row
                .sets
                .iter()
                .filter_map(|entry| set_orm(&entry.set))
                .reduce(f64::max);
            row.best_1rm
                .is_some_and(|best| day_best.is_some_and(|value| close(value, best)))
        })
        .count();
    i64::try_from(records).unwrap_or(0)
}

fn body_counts(rows: &[LogExercise]) -> Vec<(String, i64)> {
    let mut counts: HashMap<String, i64> = HashMap::new();
    for row in rows {
        let Some(label) = row.muscle_main.as_deref() else {
            continue;
        };
        let sets = i64::try_from(
            row.sets
                .iter()
                .filter(|set| {
                    set.set.weight.unwrap_or(0.0) > 0.0
                        || set.set.reps.unwrap_or(0) > 0
                        || set.set.seconds.unwrap_or(0) > 0
                        || set.set.distance_m.unwrap_or(0.0) > 0.0
                })
                .count(),
        )
        .unwrap_or(i64::MAX);
        if sets == 0 {
            continue;
        }
        let count = counts.entry(label.to_string()).or_insert(0);
        *count = count.saturating_add(sets);
    }
    counts.into_iter().collect()
}

fn fields(log_type: &str) -> &'static [Field] {
    match log_type {
        "weight_and_reps" => &[Field::Weight, Field::Reps],
        "reps" => &[Field::Reps],
        "duration" => &[Field::Seconds],
        "reps_and_duration" => &[Field::Reps, Field::Seconds],
        "cardio" => &[Field::Seconds, Field::Distance],
        _ => &[],
    }
}

fn field_label(i18n: &I18n, field: Field, log_type: &str) -> String {
    match field {
        Field::Weight => i18n.t("common.labels.weight"),
        Field::Reps => i18n.t("common.labels.reps"),
        Field::Seconds if log_type == "cardio" => i18n.t("common.labels.min"),
        Field::Seconds => i18n.t("common.labels.secs"),
        Field::Distance => i18n.t("common.labels.km"),
    }
}

fn field_text(field: Field, log_type: &str, set: &SessionSet, lb: bool) -> String {
    match field {
        Field::Weight => weight_field(set.weight, lb),
        Field::Reps => set.reps.map_or_else(String::new, |value| value.to_string()),
        Field::Seconds if log_type == "cardio" => set
            .seconds
            .map_or_else(String::new, |value| (value / 60).to_string()),
        Field::Seconds => set
            .seconds
            .map_or_else(String::new, |value| value.to_string()),
        Field::Distance => set
            .distance_m
            .map_or_else(String::new, |value| trim(value / 1000.0)),
    }
}

fn field_cell(field: Field, log_type: &str, set: &SessionSet, lb: bool) -> String {
    let text = field_text(field, log_type, set, lb);
    if text.is_empty() {
        "—".to_string()
    } else {
        text
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

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 0.005
}

fn trim(value: f64) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

pub fn volume_text(volume: f64) -> String {
    let digits = format!("{volume:.0}");
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && digits.len().saturating_sub(index) % 3 == 0 {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

fn duration_text(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn time_text(i18n: &I18n, epoch: i64) -> String {
    Local.timestamp_opt(epoch, 0).single().map_or_else(
        || "—".to_string(),
        |moment| {
            if i18n.t("common.time.format") == "24" {
                return moment.format("%H:%M").to_string();
            }
            let hour = moment.hour() % 12;
            let suffix = if moment.hour() < 12 {
                i18n.t("common.time.am")
            } else {
                i18n.t("common.time.pm")
            };
            format!(
                "{}:{:02} {suffix}",
                if hour == 0 { 12 } else { hour },
                moment.minute()
            )
        },
    )
}

fn local_date(epoch: i64) -> Option<NaiveDate> {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .map(|moment| moment.date_naive())
}

fn first_of_month(day: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(day.year(), day.month(), 1).unwrap_or(day)
}

fn month_cells(month: NaiveDate, first: u32) -> Vec<Option<NaiveDate>> {
    let lead = usize::try_from(offset_from(month, first)).unwrap_or(0);
    let mut cells = vec![None; lead];
    let mut day = month;
    while day.month() == month.month() {
        cells.push(Some(day));
        let Some(next) = day.succ_opt() else {
            break;
        };
        day = next;
    }
    while cells.len() % 7 != 0 {
        cells.push(None);
    }
    cells
}

fn day_labels(first: u32, i18n: &I18n) -> Vec<String> {
    i18n.weekdays_short()
        .into_iter()
        .cycle()
        .skip(usize::try_from(first % 7).unwrap_or(0))
        .take(7)
        .collect()
}

fn streak(days: &[NaiveDate], today: NaiveDate, first: u32) -> i64 {
    let weeks = days
        .iter()
        .map(|day| week_start(*day, first))
        .collect::<Vec<_>>();
    let mut cursor = week_start(today, first);
    if !weeks.contains(&cursor) {
        let Some(previous) = cursor.checked_sub_days(Days::new(7)) else {
            return 0;
        };
        cursor = previous;
    }
    let mut count = 0i64;
    while weeks.contains(&cursor) {
        count = count.saturating_add(1);
        let Some(previous) = cursor.checked_sub_days(Days::new(7)) else {
            break;
        };
        cursor = previous;
    }
    count
}

pub fn week_start(day: NaiveDate, first: u32) -> NaiveDate {
    day.checked_sub_days(Days::new(u64::from(offset_from(day, first))))
        .unwrap_or(day)
}

fn offset_from(day: NaiveDate, first: u32) -> u32 {
    day.weekday()
        .num_days_from_monday()
        .wrapping_add(7)
        .wrapping_sub(first.rem_euclid(7))
        .rem_euclid(7)
}
