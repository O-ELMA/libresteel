use chrono::{Datelike, Days, NaiveDate};
use dioxus::prelude::*;
use dioxus_charts::{BarChart, LineChart, PieChart};
use dioxus_free_icons::{icons::ld_icons::LdChevronDown, Icon};

use crate::{
    components::ui::{use_exit, Sheet},
    i18n::{use_i18n, I18n},
    pages::calendar::midnight,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Sets,
    Time,
}

#[derive(Clone, Copy)]
pub struct Window {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl Window {
    pub const fn days(self) -> i64 {
        self.end
            .signed_duration_since(self.start)
            .num_days()
            .saturating_add(1)
    }

    pub fn start_at(self) -> i64 {
        midnight(self.start)
    }

    pub fn end_at(self) -> i64 {
        midnight(self.end.checked_add_days(Days::new(1)).unwrap_or(self.end))
    }

    pub fn previous(self) -> Option<Self> {
        let end = self.start.checked_sub_days(Days::new(1))?;
        let start = end.checked_sub_days(Days::new(
            u64::try_from(self.days().saturating_sub(1)).ok()?,
        ))?;
        Some(Self { start, end })
    }
}

#[component]
pub fn Chips(labels: Vec<String>, selected: usize, onselect: EventHandler<usize>) -> Element {
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
pub fn ModeChip(label: String, active: bool, onclick: EventHandler<()>) -> Element {
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
pub fn DateRow(
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
pub fn Bars(series: Vec<f32>, labels: Vec<String>, axis: Axis) -> Element {
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
pub fn Lines(series: Vec<f32>, labels: Vec<String>) -> Element {
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
pub fn Donut(shares: Vec<f32>) -> Element {
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

pub fn offset_window(today: NaiveDate, length: i64, offset: i64) -> Window {
    let shift = u64::try_from(offset)
        .unwrap_or(0)
        .saturating_mul(u64::try_from(length).unwrap_or(0));
    let end = today.checked_sub_days(Days::new(shift)).unwrap_or(today);
    Window {
        start: days_back(end, length),
        end,
    }
}

pub fn days_back(day: NaiveDate, days: i64) -> NaiveDate {
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

pub fn short_date(i18n: &I18n, date: NaiveDate) -> String {
    format!("{} {}", i18n.month_short(date.month()), date.day())
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

pub const fn line_anchor(index: usize, len: usize) -> bool {
    index == 0 || index.saturating_add(1) == len || index == len / 2
}

// dioxus-charts takes f32 and stable Rust has no fallible f64 -> f32 conversion.
#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
pub const fn chart_value(value: f64) -> f32 {
    value as f32
}
