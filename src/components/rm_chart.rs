use chrono::{Days, Local};
use dioxus::prelude::*;

use crate::{
    components::charts::{
        chart_value, days_back, line_anchor, short_date, Chips, DateRow, Lines, ModeChip,
    },
    components::ui::{display_weight, quiet, trim, weight_unit},
    db,
    i18n::{use_i18n, I18n},
    pages::calendar::{local_date, long_date, midnight},
};

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
