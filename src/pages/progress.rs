use std::collections::HashMap;

use chrono::{Datelike, Local, Months, NaiveDate};
use dioxus::prelude::*;

use crate::{
    components::charts::{
        chart_value, offset_window, short_date, Axis, Bars, Chips, DateRow, Donut, Window,
    },
    components::ui::{change, duration_label, quiet, to_f64, BackIcon, Delta, IconButton, IconTone},
    db,
    i18n::{use_i18n, I18n},
    pages::calendar::{first_of_month, local_date, volume_text},
};

const COLORS: [&str; 8] = [
    "#eec942", "#ff9b61", "#9ae33d", "#ba84fb", "#70cbf3", "#fdde67", "#c8a0ff", "#b6f36b",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Range {
    Week,
    TwoWeeks,
    Month,
    Year,
    All,
}

const RANGES: [Range; 5] = [
    Range::Week,
    Range::TwoWeeks,
    Range::Month,
    Range::Year,
    Range::All,
];

impl Range {
    fn label(self, i18n: &I18n) -> String {
        match self {
            Self::Week => "7D".to_string(),
            Self::TwoWeeks => "14D".to_string(),
            Self::Month => "1M".to_string(),
            Self::Year => "12M".to_string(),
            Self::All => i18n.t("progress.all"),
        }
    }

    const fn days(self) -> Option<i64> {
        match self {
            Self::Week => Some(7),
            Self::TwoWeeks => Some(14),
            Self::Month => Some(30),
            Self::Year => Some(365),
            Self::All => None,
        }
    }
}

#[component]
pub fn VolumeProgress() -> Element {
    let i18n = use_i18n();
    let mut range = use_signal(|| Range::TwoWeeks);
    let mut offset = use_signal(|| 0i64);
    let today = Local::now().date_naive();
    let first = quiet(db::first_session_at()).and_then(local_date);
    let selected = range();
    let window = progress_window(selected, today, offset(), first);
    let start_at = window.start_at();
    let end_at = window.end_at();

    let total = to_f64(quiet(db::sets_between(start_at, end_at)));
    let previous = window
        .previous()
        .map(|prev| to_f64(quiet(db::sets_between(prev.start_at(), prev.end_at()))));
    let points = quiet(db::sets_sessions_between(start_at, end_at))
        .into_iter()
        .map(|(epoch, count)| (epoch, to_f64(count)))
        .collect::<Vec<_>>();
    let (labels, series) = buckets(&i18n, selected, &points, window);
    let weeks = f64::from(i32::try_from(window.days()).unwrap_or(1).max(1)) / 7.0;
    let average = total / weeks;
    let delta = previous.and_then(|prev| change(total, prev));

    rsx! {
        BackHeader {}
        div { class: "px-5 pb-8",
            div { class: "mt-4 text-sm font-semibold", "{i18n.t(\"progress.totalSets\")}" }
            div { class: "mt-2 flex items-baseline gap-3",
                span { class: "text-[40px] font-bold leading-tight tracking-tight text-mango",
                    "{volume_text(total)}"
                }
                if let Some(percent) = delta {
                    Delta { percent }
                }
            }
            if let Some(prev) = previous {
                Compare { kind: Axis::Sets, total, previous: prev, days: window.days(), range: selected }
            }
            Chips {
                labels: RANGES.iter().map(|item| item.label(&i18n)).collect::<Vec<_>>(),
                selected: RANGES.iter().position(|item| *item == selected).unwrap_or(0),
                onselect: move |index| {
                    if let Some(next) = RANGES.get(index) {
                        offset.set(0);
                        range.set(*next);
                    }
                },
            }
            div { class: "mt-4",
                DateRow {
                    start: window.start,
                    end: window.end,
                    today,
                    first,
                    days: selected.days(),
                    offset: offset(),
                    onpick: move |index| offset.set(index),
                }
            }
            if points.is_empty() {
                p { class: "mt-10 text-center text-sm text-muted", "{i18n.t(\"progress.nothingLogged\")}" }
            } else {
                Bars { series, labels, axis: Axis::Sets }
            }
            div { class: "mt-4 flex items-center justify-between rounded-xl bg-tile px-4 py-3",
                span { class: "text-sm font-semibold", "{i18n.t(\"progress.averagePerWeek\")}" }
                span { class: "text-[15px] font-bold", "{volume_text(average)}" }
            }
        }
    }
}

#[component]
pub fn TimeProgress() -> Element {
    let i18n = use_i18n();
    let mut range = use_signal(|| Range::TwoWeeks);
    let mut offset = use_signal(|| 0i64);
    let today = Local::now().date_naive();
    let first = quiet(db::first_session_at()).and_then(local_date);
    let selected = range();
    let window = progress_window(selected, today, offset(), first);
    let start_at = window.start_at();
    let end_at = window.end_at();

    let total = quiet(db::workout_seconds_between(start_at, end_at));
    let previous = window
        .previous()
        .map(|prev| quiet(db::workout_seconds_between(prev.start_at(), prev.end_at())));
    let sessions = quiet(db::workout_sessions_between(start_at, end_at));
    let minutes = sessions
        .iter()
        .map(|(epoch, seconds)| (*epoch, to_f64(*seconds) / 60.0))
        .collect::<Vec<_>>();
    let (labels, series) = buckets(&i18n, selected, &minutes, window);
    let weeks = f64::from(i32::try_from(window.days()).unwrap_or(1).max(1)) / 7.0;
    let average_minutes = to_f64(total) / 60.0 / weeks;
    let delta = previous.and_then(|prev| change(to_f64(total), to_f64(prev)));

    rsx! {
        BackHeader {}
        div { class: "px-5 pb-8",
            div { class: "mt-4 text-sm font-semibold", "{i18n.t(\"progress.totalTime\")}" }
            div { class: "mt-2 flex items-baseline gap-3",
                span { class: "text-[40px] font-bold leading-tight tracking-tight text-sky",
                    "{duration_label(total)}"
                }
                if let Some(percent) = delta {
                    Delta { percent }
                }
            }
            if let Some(prev) = previous {
                Compare { kind: Axis::Time, total: to_f64(total), previous: to_f64(prev), days: window.days(), range: selected }
            }
            Chips {
                labels: RANGES.iter().map(|item| item.label(&i18n)).collect::<Vec<_>>(),
                selected: RANGES.iter().position(|item| *item == selected).unwrap_or(0),
                onselect: move |index| {
                    if let Some(next) = RANGES.get(index) {
                        offset.set(0);
                        range.set(*next);
                    }
                },
            }
            div { class: "mt-4",
                DateRow {
                    start: window.start,
                    end: window.end,
                    today,
                    first,
                    days: selected.days(),
                    offset: offset(),
                    onpick: move |index| offset.set(index),
                }
            }
            div { class: "mt-4 text-sm font-semibold", "{i18n.t(\"progress.time\")}" }
            if minutes.is_empty() {
                p { class: "mt-10 text-center text-sm text-muted", "{i18n.t(\"progress.nothingLogged\")}" }
            } else {
                Bars { series, labels, axis: Axis::Time }
            }
            div { class: "mt-4 flex items-center justify-between rounded-xl bg-tile px-4 py-3",
                span { class: "text-sm font-semibold", "{i18n.t(\"progress.averagePerWeek\")}" }
                span { class: "text-[15px] font-bold", "{minutes_label(average_minutes)}" }
            }
        }
    }
}

#[component]
pub fn MuscleProgress() -> Element {
    let i18n = use_i18n();
    let mut range = use_signal(|| Range::Week);
    let mut offset = use_signal(|| 0i64);
    let today = Local::now().date_naive();
    let first = quiet(db::first_session_at()).and_then(local_date);
    let selected = range();
    let window = progress_window(selected, today, offset(), first);

    let mut muscles = quiet(db::muscle_sets_between(window.start_at(), window.end_at()));
    muscles.retain(|(_, count)| *count > 0);
    muscles.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let total = muscles.iter().map(|(_, count)| *count).sum::<i64>();
    let shares = muscles
        .iter()
        .map(|(_, count)| {
            if total == 0 {
                0.0
            } else {
                chart_value(to_f64(*count) / to_f64(total) * 100.0)
            }
        })
        .collect::<Vec<_>>();
    let leaders = muscles
        .iter()
        .take(2)
        .map(|(name, _)| i18n.value("exercises.muscles", name))
        .collect::<Vec<_>>()
        .join(", ");

    rsx! {
        BackHeader {}
        div { class: "px-5 pb-8",
            div { class: "mt-4 flex items-center justify-between",
                span { class: "text-sm font-semibold", "{i18n.t(\"progress.breakdown\")}" }
                DateRow {
                    start: window.start,
                    end: window.end,
                    today,
                    first,
                    days: selected.days(),
                    offset: offset(),
                    onpick: move |index| offset.set(index),
                }
            }
            div { class: "mt-3 text-[38px] font-bold leading-tight tracking-tight",
                if leaders.is_empty() { "—" } else { "{leaders}" }
            }
            div { class: "mt-1 text-sm text-muted", "{i18n.t(\"progress.topMuscles\")}" }
            Chips {
                labels: RANGES.iter().map(|item| item.label(&i18n)).collect::<Vec<_>>(),
                selected: RANGES.iter().position(|item| *item == selected).unwrap_or(0),
                onselect: move |index| {
                    if let Some(next) = RANGES.get(index) {
                        offset.set(0);
                        range.set(*next);
                    }
                },
            }
            if muscles.is_empty() {
                p { class: "mt-10 text-center text-sm text-muted", "{i18n.t(\"progress.noSets\")}" }
            } else {
                Donut { shares }
                div { class: "mt-6 flex items-center justify-between text-sm text-muted",
                    span { "{i18n.t(\"progress.muscle\")}" }
                    span { "{i18n.t(\"progress.sets\")}" }
                }
                for (index, (name, count)) in muscles.iter().enumerate() {
                    div {
                        key: "{name}",
                        class: if index % 2 == 0 {
                            "mt-1 flex items-center gap-2 px-3 py-3"
                        } else {
                            "mt-1 flex items-center gap-2 rounded-lg bg-tile px-3 py-3"
                        },
                        span {
                            class: "size-2.5 rounded-full",
                            style: "background-color: {color_at(index)}",
                        }
                        span { class: "flex-1 text-[15px] font-medium",
                            "{i18n.value(\"exercises.muscles\", name)}"
                        }
                        span { class: "text-[15px] font-semibold text-lilac", "{count}" }
                    }
                }
            }
        }
    }
}

#[component]
fn BackHeader() -> Element {
    let navigator = use_navigator();
    let i18n = use_i18n();
    rsx! {
        header { class: "flex h-14 items-center px-5",
            IconButton {
                tone: IconTone::Lead,
                label: i18n.t("common.labels.back"),
                onclick: move |_| navigator.go_back(),
                BackIcon { size: 24 }
            }
        }
    }
}

#[component]
fn Compare(
    kind: Axis,
    total: f64,
    previous: f64,
    days: i64,
    range: Range,
) -> Element {
    let i18n = use_i18n();
    if previous <= 0.0 || range == Range::All {
        return rsx! {};
    }
    let diff = total - previous;
    let word = if diff >= 0.0 {
        i18n.t("progress.increased")
    } else {
        i18n.t("progress.decreased")
    };
    let amount = match kind {
        Axis::Sets => volume_text(diff.abs()),
        Axis::Time => minutes_label(diff.abs() / 60.0),
    };
    let text = i18n.ta(
        "progress.compare",
        &[
            ("days", &comparison_days(range).to_string()),
            ("word", &word),
            ("amount", &amount),
        ],
    );
    rsx! {
        p { class: "mt-2 text-sm text-muted",
            "{text}"
        }
    }
}

fn buckets(
    i18n: &I18n,
    range: Range,
    points: &[(i64, f64)],
    window: Window,
) -> (Vec<String>, Vec<f32>) {
    if matches!(range, Range::Year | Range::All) {
        return monthly_buckets(i18n, points, window);
    }
    let len = usize::try_from(window.days()).unwrap_or(0);
    let mut days = Vec::with_capacity(len);
    let mut cursor = window.start;
    for _ in 0..len {
        days.push(cursor);
        let Some(next) = cursor.succ_opt() else {
            break;
        };
        cursor = next;
    }
    let mut values = vec![0.0f32; days.len()];
    let mut positions: HashMap<NaiveDate, usize> = HashMap::new();
    for (index, day) in days.iter().enumerate() {
        positions.insert(*day, index);
    }
    for (epoch, value) in points {
        if let Some(date) = local_date(*epoch) {
            if let Some(&index) = positions.get(&date) {
                if let Some(cell) = values.get_mut(index) {
                    *cell += chart_value(*value);
                }
            }
        }
    }
    let labels = days
        .iter()
        .enumerate()
        .map(|(index, day)| {
            if chart_anchor(index, days.len()) {
                short_date(i18n, *day)
            } else {
                String::new()
            }
        })
        .collect();
    (labels, values)
}

fn monthly_buckets(i18n: &I18n, points: &[(i64, f64)], window: Window) -> (Vec<String>, Vec<f32>) {
    let mut months = Vec::new();
    let mut cursor = first_of_month(window.start);
    let last = first_of_month(window.end);
    loop {
        months.push(cursor);
        if cursor >= last {
            break;
        }
        let Some(next) = cursor.checked_add_months(Months::new(1)) else {
            break;
        };
        cursor = next;
    }
    let mut positions: HashMap<(i32, u32), usize> = HashMap::new();
    for (index, month) in months.iter().enumerate() {
        positions.insert((month.year(), month.month()), index);
    }
    let mut values = vec![0.0f32; months.len()];
    for (epoch, value) in points {
        if let Some(date) = local_date(*epoch) {
            if let Some(&index) = positions.get(&(date.year(), date.month())) {
                if let Some(cell) = values.get_mut(index) {
                    *cell += chart_value(*value);
                }
            }
        }
    }
    let labels = months
        .iter()
        .enumerate()
        .map(|(index, month)| {
            if chart_anchor(index, months.len()) {
                i18n.month_short(month.month())
            } else {
                String::new()
            }
        })
        .collect();
    (labels, values)
}

const fn chart_anchor(index: usize, len: usize) -> bool {
    index == 0
        || index.saturating_add(1) == len
        || index == len / 4
        || index == len / 2
        || index == len.saturating_mul(3) / 4
}

fn progress_window(
    range: Range,
    today: NaiveDate,
    offset: i64,
    first: Option<NaiveDate>,
) -> Window {
    range.days().map_or_else(
        || Window {
            start: first.unwrap_or(today),
            end: today,
        },
        |length| offset_window(today, length, offset),
    )
}

fn comparison_days(range: Range) -> i64 {
    range.days().unwrap_or(0)
}

fn minutes_label(minutes: f64) -> String {
    let total = minutes.round();
    if total >= 60.0 {
        let hours = (total / 60.0).floor();
        let rest = total - hours * 60.0;
        if rest < 0.5 {
            format!("{hours:.0}h")
        } else {
            format!("{hours:.0}h {rest:.0}m")
        }
    } else {
        format!("{total:.0}m")
    }
}

fn color_at(index: usize) -> &'static str {
    COLORS.get(index).copied().unwrap_or("#555c69")
}
