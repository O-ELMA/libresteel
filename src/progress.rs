use std::collections::HashMap;

use chrono::{Datelike, Days, Local, Months, NaiveDate, NaiveTime, TimeZone};
use dioxus::prelude::*;
use dioxus_charts::{BarChart, LineChart, PieChart};
use dioxus_free_icons::{icons::ld_icons::LdChevronDown, Icon};

use crate::{
    calendar::volume_text,
    db,
    i18n::{use_i18n, I18n},
    ui::{
        change, display_weight, duration_label, trim, use_exit, weight_unit, BackIcon, Delta, Sheet,
    },
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum RmRange {
    TwoWeeks,
    Month,
    ThreeMonths,
    SixMonths,
    Year,
    All,
}

const RM_RANGES: [RmRange; 6] = [
    RmRange::TwoWeeks,
    RmRange::Month,
    RmRange::ThreeMonths,
    RmRange::SixMonths,
    RmRange::Year,
    RmRange::All,
];

impl RmRange {
    fn label(self, i18n: &I18n) -> String {
        match self {
            Self::TwoWeeks => "14D".to_string(),
            Self::Month => "1M".to_string(),
            Self::ThreeMonths => "3M".to_string(),
            Self::SixMonths => "6M".to_string(),
            Self::Year => "12M".to_string(),
            Self::All => i18n.t("progress.all"),
        }
    }

    const fn days(self) -> Option<i64> {
        match self {
            Self::TwoWeeks => Some(14),
            Self::Month => Some(30),
            Self::ThreeMonths => Some(90),
            Self::SixMonths => Some(180),
            Self::Year => Some(365),
            Self::All => None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    OneRm,
    Volume,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    Sets,
    Time,
}

#[derive(Clone, Copy)]
struct Window {
    start: NaiveDate,
    end: NaiveDate,
}

impl Window {
    const fn days(self) -> i64 {
        self.end
            .signed_duration_since(self.start)
            .num_days()
            .saturating_add(1)
    }

    fn start_at(self) -> i64 {
        midnight(self.start)
    }

    fn end_at(self) -> i64 {
        midnight(self.end.checked_add_days(Days::new(1)).unwrap_or(self.end))
    }

    fn previous(self) -> Option<Self> {
        let end = self.start.checked_sub_days(Days::new(1))?;
        let start = end.checked_sub_days(Days::new(
            u64::try_from(self.days().saturating_sub(1)).ok()?,
        ))?;
        Some(Self { start, end })
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

    let total = seconds_of(quiet(db::sets_between(start_at, end_at)));
    let previous = window
        .previous()
        .map(|prev| seconds_of(quiet(db::sets_between(prev.start_at(), prev.end_at()))));
    let points = quiet(db::sets_sessions_between(start_at, end_at))
        .into_iter()
        .map(|(epoch, count)| (epoch, seconds_of(count)))
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
        .map(|(epoch, seconds)| (*epoch, seconds_of(*seconds) / 60.0))
        .collect::<Vec<_>>();
    let (labels, series) = buckets(&i18n, selected, &minutes, window);
    let weeks = f64::from(i32::try_from(window.days()).unwrap_or(1).max(1)) / 7.0;
    let average_minutes = seconds_of(total) / 60.0 / weeks;
    let delta = previous.and_then(|prev| change(seconds_of(total), seconds_of(prev)));

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
                Compare { kind: Axis::Time, total: seconds_of(total), previous: seconds_of(prev), days: window.days(), range: selected }
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
                chart_value(seconds_of(*count) / seconds_of(total) * 100.0)
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
pub fn RmContent(id: i64, show_name: bool) -> Element {
    let i18n = use_i18n();
    let mut range = use_signal(|| RmRange::ThreeMonths);
    let mut mode = use_signal(|| Mode::OneRm);
    let today = Local::now().date_naive();
    let first = quiet(db::first_session_at()).and_then(local_date);
    let lb = db::unit_lb();
    let selected = range();
    let start = selected
        .days()
        .map_or_else(|| first.unwrap_or(today), |days| days_back(today, days));
    let start_at = midnight(start);
    let end_at = midnight(today.checked_add_days(Days::new(1)).unwrap_or(today));
    let name = quiet(db::exercise_name(id)).unwrap_or_default();
    let points = match mode() {
        Mode::OneRm => quiet(db::exercise_1rm_sessions(id, start_at, end_at)),
        Mode::Volume => quiet(db::exercise_volume_sessions(id, start_at, end_at)),
    }
    .into_iter()
    .map(|(epoch, value)| (epoch, display_weight(value, lb)))
    .collect::<Vec<_>>();
    let dated = points
        .iter()
        .filter_map(|(epoch, value)| local_date(*epoch).map(|date| (date, *value)))
        .collect::<Vec<_>>();
    let latest = dated.last().copied();
    let lowest = dated
        .iter()
        .min_by(|left, right| left.1.total_cmp(&right.1));
    let highest = dated
        .iter()
        .max_by(|left, right| left.1.total_cmp(&right.1));
    let series = dated
        .iter()
        .map(|(_, value)| chart_value(*value))
        .collect::<Vec<_>>();
    let labels = dated
        .iter()
        .enumerate()
        .map(|(index, (date, _))| {
            if line_anchor(index, dated.len()) {
                short_date(&i18n, *date)
            } else {
                String::new()
            }
        })
        .collect::<Vec<_>>();
    let last_logged = latest
        .map(|(date, _)| i18n.ta("progress.lastLogged", &[("date", &long_date(&i18n, date))]));

    rsx! {
        div { class: "px-5 pb-8",
            if show_name {
                div { class: "mt-4 text-sm font-semibold", "{name}" }
                div { class: "mt-2 text-[40px] font-bold leading-tight tracking-tight",
                    if let Some((_, value)) = latest {
                        "{trim(value)} {weight_unit(lb)}"
                    } else {
                        "—"
                    }
                }
                div { class: "mt-1 text-sm text-muted",
                    if let Some(text) = last_logged {
                        "{text}"
                    } else {
                        "{i18n.t(\"progress.noSetsYet\")}"
                    }
                }
            }
            Chips {
                labels: RM_RANGES.iter().map(|item| item.label(&i18n)).collect::<Vec<_>>(),
                selected: RM_RANGES.iter().position(|item| *item == selected).unwrap_or(0),
                onselect: move |index| {
                    if let Some(next) = RM_RANGES.get(index) {
                        range.set(*next);
                    }
                },
            }
            div { class: "mt-4 flex rounded-lg border border-line p-1",
                ModeChip {
                    label: i18n.t("progress.oneRepMax"),
                    active: mode() == Mode::OneRm,
                    onclick: move |()| mode.set(Mode::OneRm),
                }
                ModeChip {
                    label: i18n.t("session.volume"),
                    active: mode() == Mode::Volume,
                    onclick: move |()| mode.set(Mode::Volume),
                }
            }
            div { class: "mt-4",
                DateRow {
                    start,
                    end: today,
                    today,
                    first,
                    days: None,
                    offset: 0,
                    onpick: move |_| {},
                }
            }
            if series.is_empty() {
                p { class: "mt-10 text-center text-sm text-muted", "{i18n.t(\"progress.nothingLogged\")}" }
            } else {
                Lines { series, labels }
                div { class: "mt-4 grid grid-cols-2 gap-3",
                    for (caption, entry) in [(i18n.t("progress.min"), lowest), (i18n.t("progress.max"), highest)] {
                        if let Some((date, value)) = entry {
                            div { class: "rounded-xl bg-tile px-4 py-3",
                                div { class: "text-lg font-bold", "{trim(*value)} {weight_unit(lb)}" }
                                div { class: "mt-1 text-sm text-muted",
                                    "{caption} ({long_date(&i18n, *date)})"
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
fn BackHeader() -> Element {
    let navigator = use_navigator();
    let i18n = use_i18n();
    rsx! {
        header { class: "flex h-14 items-center px-5",
            button {
                class: "grid size-10 place-items-center text-lead",
                "aria-label": i18n.t("common.labels.back"),
                onclick: move |_| navigator.go_back(),
                BackIcon { size: 24 }
            }
        }
    }
}

#[component]
fn Chips(labels: Vec<String>, selected: usize, onselect: EventHandler<usize>) -> Element {
    rsx! {
        div { class: "mt-5 flex rounded-full border border-line p-1",
            for (index, label) in labels.iter().enumerate() {
                button {
                    key: "{label}",
                    class: if index == selected {
                        "h-10 flex-1 rounded-full bg-white text-sm font-semibold text-black"
                    } else {
                        "h-10 flex-1 text-sm text-muted"
                    },
                    onclick: move |_| onselect.call(index),
                    "{label}"
                }
            }
        }
    }
}

#[component]
fn ModeChip(label: String, active: bool, onclick: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: if active {
                "h-9 flex-1 rounded-md bg-accent text-sm font-semibold text-black"
            } else {
                "h-9 flex-1 rounded-md text-sm font-semibold text-muted"
            },
            onclick: move |_| onclick.call(()),
            "{label}"
        }
    }
}

#[component]
fn DateRow(
    start: NaiveDate,
    end: NaiveDate,
    today: NaiveDate,
    first: Option<NaiveDate>,
    days: Option<i64>,
    offset: i64,
    onpick: EventHandler<i64>,
) -> Element {
    let i18n = use_i18n();
    let mut open = use_signal(|| false);
    let (closing, close) = use_exit(open, false, |value| *value);
    let Some(length) = days else {
        return rsx! {
            span { class: "text-sm text-lead", "{range_label(&i18n, start, end)}" }
        };
    };
    let count = first.map_or(1, |first| {
        today
            .signed_duration_since(first)
            .num_days()
            .max(0)
            .checked_div(length)
            .unwrap_or(0)
            .saturating_add(1)
    });
    rsx! {
        button {
            class: "flex items-center gap-1 text-sm text-lead",
            onclick: move |_| {
                open.set(true);
            },
            "{range_label(&i18n, start, end)}"
            Icon { icon: LdChevronDown, width: 16, height: 16 }
        }
        if open() {
            Sheet {
                closing: closing(),
                onclose: close,
                div { class: "px-2",
                    div { class: "flex h-12 items-center",
                        span { class: "text-[15px] font-semibold", "{picker_title(&i18n, length)}" }
                    }
                    div { class: "max-h-[55dvh] overflow-y-auto pt-2",
                        for index in 0..count {
                            button {
                                key: "{index}",
                                class: if index == offset {
                                    "mt-2 flex h-14 w-full items-center justify-center rounded-xl bg-lilac text-base font-semibold text-black"
                                } else {
                                    "mt-2 flex h-14 w-full items-center justify-center rounded-xl text-base text-muted"
                                },
                                onclick: move |_| {
                                    onpick.call(index);
                                    close.call(());
                                },
                                "{window_option_label(&i18n, today, length, index)}"
                            }
                        }
                    }
                }
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

#[component]
fn Bars(series: Vec<f32>, labels: Vec<String>, axis: Axis) -> Element {
    let interpolation: fn(f32) -> String = if axis == Axis::Sets {
        number_axis
    } else {
        time_axis
    };
    let tone = if axis == Axis::Sets {
        "dx-chart-sets"
    } else {
        "dx-chart-time"
    };
    rsx! {
        div { class: "mt-4 aspect-[3/2] w-full rounded-3xl border border-line px-1 pt-2 {tone}",
            BarChart {
                width: "100%",
                height: "100%",
                viewbox_width: 360,
                viewbox_height: 240,
                padding_left: 52,
                padding_right: 10,
                padding_top: 12,
                padding_bottom: 26,
                max_ticks: 6,
                bar_width: "10",
                show_series_labels: false,
                show_dotted_grid: false,
                label_interpolation: Some(interpolation),
                series: vec![series],
                labels: labels,
            }
        }
    }
}

#[component]
fn Lines(series: Vec<f32>, labels: Vec<String>) -> Element {
    let interpolation: fn(f32) -> String = number_axis;
    rsx! {
        div { class: "mt-4 aspect-[3/2] w-full",
            LineChart {
                width: "100%",
                height: "100%",
                viewbox_width: 360,
                viewbox_height: 240,
                padding_left: 44,
                padding_right: 10,
                padding_top: 14,
                padding_bottom: 26,
                max_ticks: 5,
                line_width: "2.5",
                dot_size: "7",
                show_dotted_grid: false,
                label_interpolation: Some(interpolation),
                series: vec![series],
                labels: labels,
            }
        }
    }
}

#[component]
fn Donut(shares: Vec<f32>) -> Element {
    let interpolation: fn(f32) -> String = percent_label;
    rsx! {
        div { class: "mx-auto mt-6 aspect-square w-full max-w-[290px]",
            PieChart {
                width: "100%",
                height: "100%",
                viewbox_width: 280,
                viewbox_height: 280,
                donut: true,
                donut_width: 42.0,
                padding: 10.0,
                start_angle: -90.0,
                label_offset: 29.0,
                label_interpolation: Some(interpolation),
                series: shares,
            }
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

const fn line_anchor(index: usize, len: usize) -> bool {
    index == 0 || index.saturating_add(1) == len || index == len / 2
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

fn offset_window(today: NaiveDate, length: i64, offset: i64) -> Window {
    let shift = u64::try_from(offset)
        .unwrap_or(0)
        .saturating_mul(u64::try_from(length).unwrap_or(0));
    let end = today.checked_sub_days(Days::new(shift)).unwrap_or(today);
    Window {
        start: days_back(end, length),
        end,
    }
}

fn days_back(day: NaiveDate, days: i64) -> NaiveDate {
    let span = u64::try_from(days.saturating_sub(1)).unwrap_or(0);
    day.checked_sub_days(Days::new(span)).unwrap_or(day)
}

fn picker_title(i18n: &I18n, length: i64) -> String {
    match length {
        7 => i18n.t("progress.select7"),
        14 => i18n.t("progress.select14"),
        30 => i18n.t("progress.selectMonth"),
        365 => i18n.t("progress.selectYear"),
        _ => i18n.t("progress.selectRange"),
    }
}

fn window_option_label(i18n: &I18n, today: NaiveDate, length: i64, offset: i64) -> String {
    let window = offset_window(today, length, offset);
    format!(
        "{} - {}, {}",
        short_date(i18n, window.start),
        short_date(i18n, window.end),
        window.end.year()
    )
}

fn short_date(i18n: &I18n, date: NaiveDate) -> String {
    format!("{} {}", i18n.month_short(date.month()), date.day())
}

fn long_date(i18n: &I18n, date: NaiveDate) -> String {
    format!(
        "{} {}, {}",
        i18n.month_short(date.month()),
        date.day(),
        date.year()
    )
}

fn range_label(i18n: &I18n, start: NaiveDate, end: NaiveDate) -> String {
    if start.year() == end.year() {
        if start.month() == end.month() {
            format!("{}–{}, {}", short_date(i18n, start), end.day(), end.year())
        } else {
            format!(
                "{}–{}, {}",
                short_date(i18n, start),
                short_date(i18n, end),
                end.year()
            )
        }
    } else {
        format!(
            "{}, {}–{}, {}",
            short_date(i18n, start),
            start.year(),
            short_date(i18n, end),
            end.year()
        )
    }
}

fn comparison_days(range: Range) -> i64 {
    range.days().unwrap_or(0)
}

fn time_axis(value: f32) -> String {
    let total = value.round();
    if total >= 60.0 {
        let hours = (total / 60.0).floor();
        let rest = total - hours * 60.0;
        if rest < 0.5 {
            format!("{hours:.0}h")
        } else {
            format!("{hours:.0}h{rest:.0}m")
        }
    } else {
        format!("{total:.0}m")
    }
}

fn number_axis(value: f32) -> String {
    format!("{value:.0}")
}

fn percent_label(value: f32) -> String {
    format!("{value:.0}%")
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

fn quiet<T: Default>(result: anyhow::Result<T>) -> T {
    result.unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("progress query failed: {err:#}");
        T::default()
    })
}

fn local_date(epoch: i64) -> Option<NaiveDate> {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .map(|moment| moment.date_naive())
}

fn midnight(day: NaiveDate) -> i64 {
    Local
        .from_local_datetime(&day.and_time(NaiveTime::MIN))
        .earliest()
        .map_or(0, |moment| moment.timestamp())
}

fn first_of_month(day: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(day.year(), day.month(), 1).unwrap_or(day)
}

fn seconds_of(value: i64) -> f64 {
    f64::from(i32::try_from(value).unwrap_or(i32::MAX))
}

// dioxus-charts takes f32 and stable Rust has no fallible f64 -> f32 conversion.
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
const fn chart_value(value: f64) -> f32 {
    value as f32
}
