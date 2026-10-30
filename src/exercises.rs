use chrono::{Datelike, Local, TimeZone};
use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{
        LdArrowDownUp, LdBan, LdCheck, LdChevronDown, LdGauge, LdSearch, LdTag, LdX,
    },
    Icon,
};

use crate::{
    db,
    i18n::{use_i18n, I18n},
    media,
    progress::RmContent,
    shell::Route,
    ui::{display_weight, trim, use_exit, weight_field, weight_unit, BackIcon, Sheet},
};

struct Group {
    key: &'static str,
    label: &'static str,
    art: Asset,
}

// clippy::volatile_composites false-fires on asset!(), which embeds &[u8] in a const descriptor.
#[rustfmt::skip]
#[allow(clippy::volatile_composites)]
const GROUPS: [Group; 11] = [
    Group { key: "abs", label: "Abs", art: asset!("/assets/muscles/abs.svg") },
    Group { key: "upper_legs", label: "Upper Legs", art: asset!("/assets/muscles/upper_legs.svg") },
    Group { key: "back", label: "Back", art: asset!("/assets/muscles/back.svg") },
    Group { key: "shoulders", label: "Shoulders", art: asset!("/assets/muscles/shoulders.svg") },
    Group { key: "chest", label: "Chest", art: asset!("/assets/muscles/chest.svg") },
    Group { key: "biceps", label: "Biceps", art: asset!("/assets/muscles/biceps.svg") },
    Group { key: "triceps", label: "Triceps", art: asset!("/assets/muscles/triceps.svg") },
    Group { key: "forearms", label: "Forearms", art: asset!("/assets/muscles/forearms.svg") },
    Group { key: "lower_legs", label: "Lower Legs", art: asset!("/assets/muscles/lower_legs.svg") },
    Group { key: "glutes", label: "Glutes", art: asset!("/assets/muscles/glutes.svg") },
    Group { key: "cardio", label: "Cardio", art: asset!("/assets/muscles/cardio.svg") },
];

#[allow(clippy::volatile_composites)]
const SHOW_ALL_ART: Asset = asset!("/assets/muscles/body.svg");

#[allow(clippy::volatile_composites)]
const BODY_ART: Asset = asset!("/assets/equipment/body.png");

// clippy::volatile_composites false-fires on asset!(), which embeds &[u8] in a const descriptor.
#[rustfmt::skip]
#[allow(clippy::volatile_composites)]
const EQUIPMENT_ART: [(&str, Asset); 12] = [
    ("bands", asset!("/assets/equipment/bands.png")),
    ("barbell", asset!("/assets/equipment/barbell.png")),
    ("bench", asset!("/assets/equipment/bench.png")),
    ("body_weight", asset!("/assets/equipment/body.png")),
    ("cardio_machine", asset!("/assets/equipment/machine_cardio.png")),
    ("dumbbell", asset!("/assets/equipment/dumbbell.png")),
    ("exercise_ball", asset!("/assets/equipment/exercise_ball.png")),
    ("ez_curl_bar", asset!("/assets/equipment/ez_curl_bar.png")),
    ("kettlebell", asset!("/assets/equipment/kettlebell.png")),
    ("pullup_bar", asset!("/assets/equipment/pullup_bar.png")),
    ("strength_machine", asset!("/assets/equipment/machine_strength.png")),
    ("weight_plate", asset!("/assets/equipment/weight_plate.png")),
];

const DIFFICULTIES: [&str; 3] = ["beginner", "intermediate", "advanced"];

const ROW_PX: usize = 84;
const OVERSCAN: usize = 4;

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn row_at(offset: f64) -> usize {
    (offset.max(0.0) / 84.0) as usize
}

#[derive(Clone, Copy, PartialEq)]
enum Facet {
    Equipment,
    Muscles,
    Difficulty,
}

#[derive(Clone, PartialEq)]
enum RowArt {
    Image(Asset),
    Gauge,
    Tag,
}

#[component]
pub fn Exercises() -> Element {
    let i18n = use_i18n();
    let counts = db::group_counts().unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("group counts failed: {err:#}");
        Vec::new()
    });
    let mut grid: Vec<(&Group, i64)> = GROUPS
        .iter()
        .map(|group| {
            let count = counts
                .iter()
                .find(|(main, _)| main == group.label)
                .map_or(0, |(_, count)| *count);
            (group, count)
        })
        .collect();
    grid.sort_by(|a, b| b.1.cmp(&a.1));

    rsx! {
        div { class: "flex min-h-[calc(100dvh-6rem)] flex-col px-5 pt-5 pb-5",
            h1 { class: "text-[26px] font-bold tracking-tight", "{i18n.t(\"exercises.title\")}" }
            div { class: "mt-4 grid flex-1 grid-cols-3 content-between gap-x-4 gap-y-4",
                for (group, _) in grid {
                    Link {
                        to: Route::ExerciseList { group: group.key.to_string() },
                        class: "flex flex-col items-center justify-center",
                        div { class: "aspect-square w-full overflow-hidden rounded-full",
                            img {
                                src: group.art,
                                alt: "{i18n.value(\"exercises.muscles\", group.label)}",
                                class: "h-full w-full object-cover",
                            }
                        }
                        span { class: "mt-3 text-[15px] font-medium",
                            "{i18n.value(\"exercises.muscles\", group.label)}"
                        }
                    }
                }
                Link {
                    to: Route::ExerciseList { group: "all".to_string() },
                    class: "flex flex-col items-center justify-center",
                    div { class: "aspect-square w-full overflow-hidden rounded-full",
                        img {
                            src: SHOW_ALL_ART,
                            alt: "{i18n.t(\"exercises.showAll\")}",
                            class: "h-full w-full object-contain",
                        }
                    }
                    span { class: "mt-3 text-[15px] font-medium", "{i18n.t(\"exercises.showAll\")}" }
                }
            }
        }
    }
}

#[component]
pub fn ExerciseList(group: String) -> Element {
    let i18n = use_i18n();
    let rows = use_memo(move || {
        i18n.language.read();
        db::exercises().unwrap_or_else(|err| {
            dioxus::logger::tracing::error!("exercise list failed: {err:#}");
            Vec::new()
        })
    });
    let search = use_signal(String::new);
    let equipment = use_signal(Vec::<String>::new);
    let group_label = GROUPS
        .iter()
        .find(|entry| entry.key == group)
        .map(|entry| entry.label);
    let muscles = use_signal(|| group_label.map_or_else(Vec::new, |label| vec![label.to_string()]));
    let difficulty = use_signal(Vec::<String>::new);
    let mut sort_by_name = use_signal(|| false);
    let mut viewport = use_signal(|| 900.0);
    let mut first = use_signal(|| 0usize);

    use_effect(move || {
        search.read();
        equipment.read();
        muscles.read();
        difficulty.read();
        sort_by_name.read();
        first.set(0);
        let _ = document::eval("document.getElementById('exercise-list')?.scrollTo({ top: 0 });");
    });

    let all = rows.cloned();
    let equipment_choices = equipment_options(&all, &i18n);

    let selected_equipment = equipment.cloned();
    let selected_muscles = muscles.cloned();
    let selected_difficulty = difficulty.cloned();
    let needle = search.cloned().to_lowercase();

    let mut shown = all
        .iter()
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
    if sort_by_name() {
        shown.sort_by(|a, b| a.name.cmp(&b.name));
    }

    let single_group = selected_muscles.len() == 1
        && group_label.is_some_and(|label| {
            selected_muscles
                .first()
                .is_some_and(|only| only.as_str() == label)
        });
    let title = if single_group {
        group_label.map_or_else(
            || i18n.t("exercises.allExercises"),
            |label| i18n.value("exercises.muscles", label),
        )
    } else {
        i18n.t("exercises.allExercises")
    };
    let found = i18n.tn(
        "exercises.found",
        i64::try_from(shown.len()).unwrap_or(i64::MAX),
    );

    let total = shown.len();
    let visible = row_at(viewport())
        .saturating_add(1)
        .saturating_add(OVERSCAN.saturating_mul(2));
    let start = first()
        .saturating_sub(OVERSCAN)
        .min(total.saturating_sub(visible));
    let end = start.saturating_add(visible).min(total);
    let top = start.saturating_mul(ROW_PX);
    let bottom = total.saturating_sub(end).saturating_mul(ROW_PX);

    rsx! {
        div { class: "flex h-[calc(100dvh-6rem)] flex-col",
            header { class: "flex items-center gap-3 px-5 pt-5",
                Link { to: Route::Exercises {}, class: "text-lead",
                    BackIcon { size: 24 }
                }
                h1 { class: "text-[22px] font-bold tracking-tight", "{i18n.t(\"exercises.title\")}" }
            }
            div { class: "mt-6 flex items-end justify-between px-5",
                div {
                    div { class: "text-lg font-semibold", "{title}" }
                    div { class: "mt-0.5 text-sm text-muted", "{found}" }
                }
                button {
                    class: "flex items-center gap-1.5 pb-1 text-sm font-medium text-accent-ink",
                    onclick: move |_| sort_by_name.toggle(),
                    Icon { icon: LdArrowDownUp, width: 18, height: 18 }
                    if sort_by_name() {
                        "{i18n.t(\"exercises.sortName\")}"
                    } else {
                        "{i18n.t(\"exercises.sortPopularity\")}"
                    }
                }
            }
            if shown.is_empty() {
                p { class: "px-5 py-12 text-center text-base text-muted",
                    "{i18n.t(\"exercises.noMatch\")}"
                }
            }
            div {
                id: "exercise-list",
                class: "mt-2 min-h-0 flex-1 overflow-y-auto px-5 pb-36",
                onscroll: move |event: ScrollEvent| {
                    let height = f64::from(event.data().client_height());
                    if (height - viewport()).abs() > 0.5 {
                        viewport.set(height);
                    }
                    let next = row_at(event.data().scroll_top());
                    if next != first() {
                        first.set(next);
                    }
                },
                div { style: "height: {top}px" }
                for row in shown.iter().skip(start).take(end.saturating_sub(start)) {
                    Link {
                        key: "{row.id}",
                        to: Route::ExerciseDetail { id: row.id },
                        class: "flex h-[84px] shrink-0 items-center gap-4 py-3.5 [content-visibility:auto] [contain-intrinsic-size:auto_84px]",
                        ExerciseThumb { id: row.id, size: 56 }
                        div { class: "min-w-0",
                            div { class: "text-base font-semibold leading-snug", "{row.name}" }
                            div { class: "mt-0.5 truncate text-sm text-muted", "{summary(&i18n, row)}" }
                        }
                    }
                }
                div { style: "height: {bottom}px" }
            }
            div { class: "fixed inset-x-0 bottom-[calc(env(safe-area-inset-bottom)+4rem)] z-20 mx-auto w-full max-w-md border-t border-line bg-surface",
                ExerciseFilters {
                    search,
                    equipment,
                    muscles,
                    difficulty,
                    equipment_options: equipment_choices,
                }
            }
        }
    }
}

#[component]
pub fn ExerciseFilters(
    search: Signal<String>,
    equipment: Signal<Vec<String>>,
    muscles: Signal<Vec<String>>,
    difficulty: Signal<Vec<String>>,
    equipment_options: Vec<String>,
) -> Element {
    let i18n = use_i18n();
    let mut open = use_signal(|| None::<Facet>);
    let (closing, close) = use_exit(open, None, Option::is_some);
    let selected_equipment = equipment.cloned();
    let selected_muscles = muscles.cloned();
    let selected_difficulty = difficulty.cloned();
    let filters_active = !(search.cloned().is_empty()
        && selected_equipment.is_empty()
        && selected_muscles.is_empty()
        && selected_difficulty.is_empty());

    rsx! {
        div { class: "px-5 py-3",
            label { class: "flex h-12 items-center gap-3 rounded-2xl bg-tile px-4",
                Icon { icon: LdSearch, width: 20, height: 20, class: "text-muted" }
                input {
                    class: "w-full bg-transparent text-base text-lead outline-none placeholder:text-muted",
                    placeholder: i18n.t("exercises.searchPlaceholder"),
                    value: "{search}",
                    oninput: move |event| search.set(event.value()),
                }
            }
            div { class: "mt-3 flex items-center gap-2",
                div { class: "flex min-w-0 flex-1 items-center gap-2 overflow-x-auto pb-1",
                    FacetChip {
                        label: i18n.t("exercises.filters.equipment"),
                        active: !selected_equipment.is_empty(),
                        opened: open.cloned() == Some(Facet::Equipment),
                        onclick: move |_| open.set(Some(Facet::Equipment)),
                    }
                    FacetChip {
                        label: i18n.t("exercises.filters.muscles"),
                        active: !selected_muscles.is_empty(),
                        opened: open.cloned() == Some(Facet::Muscles),
                        onclick: move |_| open.set(Some(Facet::Muscles)),
                    }
                    FacetChip {
                        label: i18n.t("exercises.filters.difficulty"),
                        active: !selected_difficulty.is_empty(),
                        opened: open.cloned() == Some(Facet::Difficulty),
                        onclick: move |_| open.set(Some(Facet::Difficulty)),
                    }
                }
                button {
                    class: if filters_active {
                        "grid size-11 shrink-0 place-items-center rounded-xl border border-accent text-accent-ink"
                    } else {
                        "grid size-11 shrink-0 place-items-center rounded-xl border border-line text-muted"
                    },
                    "aria-label": i18n.t("exercises.clearFilters"),
                    onclick: move |_| {
                        search.set(String::new());
                        equipment.set(Vec::new());
                        muscles.set(Vec::new());
                        difficulty.set(Vec::new());
                        close.call(());
                    },
                    Icon { icon: LdX, width: 18, height: 18 }
                }
            }
        }
        if let Some(facet) = open.cloned() {
            Sheet {
                closing: closing(),
                onclose: close,
                h3 { class: "px-3 pb-1 text-base font-semibold", "{facet_title(&i18n, facet)}" }
                div { class: "flex max-h-[65dvh] flex-col overflow-y-auto p-1.5",
                    match facet {
                        Facet::Equipment => rsx! {
                            for option in equipment_options.iter() {
                                MenuOption {
                                    label: i18n.value("exercises.equipment", option),
                                    art: RowArt::Image(equipment_art(option)),
                                    selected: selected_equipment.contains(option),
                                    onclick: {
                                        let option = option.clone();
                                        move |_| toggle(equipment, &option)
                                    },
                                }
                            }
                        },
                        Facet::Muscles => rsx! {
                            for entry in GROUPS.iter() {
                                MenuOption {
                                    label: i18n.value("exercises.muscles", entry.label),
                                    art: RowArt::Image(
                                        art_for(Some(entry.label)).unwrap_or(BODY_ART),
                                    ),
                                    selected: selected_muscles.iter().any(|item| item == entry.label),
                                    onclick: move |_| toggle(muscles, entry.label),
                                }
                            }
                        },
                        Facet::Difficulty => rsx! {
                            for level in DIFFICULTIES {
                                MenuOption {
                                    label: i18n.value("exercises.levels", level),
                                    art: RowArt::Gauge,
                                    selected: selected_difficulty.iter().any(|item| item == level),
                                    onclick: move |_| toggle(difficulty, level),
                                }
                            }
                        },
                    }
                }
            }
        }
    }
}

fn facet_title(i18n: &I18n, facet: Facet) -> String {
    match facet {
        Facet::Equipment => i18n.t("exercises.filters.equipment"),
        Facet::Muscles => i18n.t("exercises.filters.muscles"),
        Facet::Difficulty => i18n.t("exercises.filters.difficulty"),
    }
}

#[derive(Clone, Copy, PartialEq)]
enum DetailTab {
    History,
    Chart,
    Guide,
}

const DETAIL_TABS: [DetailTab; 3] = [DetailTab::Guide, DetailTab::History, DetailTab::Chart];

#[component]
pub fn ExerciseDetail(id: i64) -> Element {
    exercise_page(id, DetailTab::Guide)
}

#[component]
pub fn ExerciseDetailTab(id: i64, tab: String) -> Element {
    let initial = match tab.as_str() {
        "history" => DetailTab::History,
        "chart" => DetailTab::Chart,
        _ => DetailTab::Guide,
    };
    exercise_page(id, initial)
}

fn exercise_page(id: i64, default_tab: DetailTab) -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let mut tab = use_signal(move || default_tab);
    let detail = match db::exercise(id) {
        Ok(detail) => detail,
        Err(err) => {
            dioxus::logger::tracing::error!("exercise {id} failed: {err:#}");
            None
        }
    };
    let Some(exercise) = detail else {
        return rsx! {
            p { class: "p-6 text-base text-muted", "{i18n.t(\"exercises.detail.notFound\")}" }
        };
    };
    let muscle = exercise
        .muscles_main
        .first()
        .map_or_else(String::new, |name| i18n.value("exercises.muscles", name));
    let log_type = exercise.log_type.clone();
    let guide = exercise.clone();

    rsx! {
        div { class: "min-h-dvh pb-12",
            header { class: "flex items-center gap-3 px-5 pt-5",
                button {
                    class: "text-lead",
                    onclick: move |_| navigator.go_back(),
                    BackIcon { size: 24 }
                }
            }
            div { class: "mt-4 flex items-start gap-4 px-5",
                div { class: "min-w-0 flex-1",
                    h1 { class: "text-[34px] font-bold leading-tight tracking-tight",
                        "{exercise.name}"
                    }
                    p { class: "mt-1 text-lg text-muted", "{muscle}" }
                    p { class: "mt-0.5 text-xs text-muted", "#{id}" }
                }
                ExerciseThumb { id, size: 64 }
            }
            div { class: "mt-6 flex border-b border-line px-5",
                for (index, item) in DETAIL_TABS.iter().enumerate() {
                    button {
                        key: "{index}",
                        class: if tab() == *item {
                            "flex-1 border-b-2 border-accent pb-3 text-[15px] font-semibold text-accent-ink"
                        } else {
                            "flex-1 border-b-2 border-transparent pb-3 text-[15px] text-muted"
                        },
                        onclick: {
                            let item = *item;
                            move |_| tab.set(item)
                        },
                        "{tab_label(&i18n, *item)}"
                    }
                }
            }
            match tab() {
                DetailTab::History => rsx! {
                    HistoryTab { id, log_type }
                },
                DetailTab::Chart => rsx! {
                    RmContent { id, show_name: false }
                },
                DetailTab::Guide => rsx! {
                    GuideTab { id, detail: guide }
                },
            }
        }
    }
}

fn tab_label(i18n: &I18n, tab: DetailTab) -> String {
    match tab {
        DetailTab::History => i18n.t("exercises.detail.history"),
        DetailTab::Chart => i18n.t("exercises.detail.chart"),
        DetailTab::Guide => i18n.t("exercises.detail.guide"),
    }
}

#[component]
fn HistoryTab(id: i64, log_type: String) -> Element {
    let i18n = use_i18n();
    let lb = db::unit_lb();
    let sessions = db::exercise_history(id, 0, i64::MAX).unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("exercise history failed: {err:#}");
        Vec::new()
    });
    if sessions.is_empty() {
        return rsx! {
            p { class: "px-5 py-16 text-center text-base text-muted",
                "{i18n.t(\"progress.nothingLogged\")}"
            }
        };
    }
    rsx! {
        div { class: "px-5",
            for session in sessions.iter() {
                div { key: "{session.started_at}", class: "mt-7",
                    div { class: "text-xl font-bold tracking-tight",
                        "{history_date(&i18n, session.started_at)}"
                    }
                    div { class: "mt-3 flex items-center gap-2 text-[11px] uppercase tracking-wide text-muted",
                        span { class: "w-8 shrink-0", "{i18n.t(\"common.labels.set\")}" }
                        span { class: "flex-1", "{history_columns(&i18n, &log_type)}" }
                        span { class: "w-20 shrink-0 text-end", "{i18n.t(\"common.labels.oneRm\")}" }
                    }
                    {
                        let best = session
                            .sets
                            .iter()
                            .filter_map(|set| set_orm(set.weight, set.reps))
                            .max_by(f64::total_cmp);
                        rsx! {
                            for (index, set) in session.sets.iter().enumerate() {
                                HistoryRow {
                                    key: "{set.set_number}",
                                    number: index.saturating_add(1),
                                    value: history_value(&i18n, &log_type, set, lb),
                                    orm: set_orm(set.weight, set.reps),
                                    best,
                                    lb,
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
fn HistoryRow(
    number: usize,
    value: String,
    orm: Option<f64>,
    best: Option<f64>,
    lb: bool,
) -> Element {
    let highlight = orm.is_some_and(|value| best == Some(value));
    rsx! {
        div { class: "flex items-center gap-2 py-2",
            span { class: "w-8 shrink-0 text-[15px] text-muted", "{number}" }
            span { class: "flex-1 text-[15px] text-lead", "{value}" }
            span {
                class: if highlight {
                    "w-20 shrink-0 text-end text-[15px] font-semibold text-lilac"
                } else {
                    "w-20 shrink-0 text-end text-[15px] text-muted"
                },
                {orm.map_or_else(String::new, |value| {
                    format!("{} {}", trim(display_weight(value, lb)), weight_unit(lb))
                })}
            }
        }
    }
}

fn history_columns(i18n: &I18n, log_type: &str) -> String {
    match log_type {
        "weight_and_reps" => i18n.t("exercises.history.weightReps"),
        "reps" => i18n.t("exercises.history.reps"),
        "duration" => i18n.t("exercises.history.duration"),
        "reps_and_duration" => i18n.t("exercises.history.repsDuration"),
        "cardio" => i18n.t("exercises.history.cardio"),
        _ => String::new(),
    }
}

fn history_value(i18n: &I18n, log_type: &str, set: &db::HistorySet, lb: bool) -> String {
    let reps = option_text(set.reps);
    let seconds = option_text(set.seconds);
    let reps_label = i18n.t("common.labels.reps");
    let secs_label = i18n.t("common.labels.secs");
    match log_type {
        "weight_and_reps" => set.weight.map_or_else(
            || format!("{reps} {reps_label}"),
            |weight| {
                format!(
                    "{} {} × {reps} {reps_label}",
                    weight_field(Some(weight), lb),
                    weight_unit(lb)
                )
            },
        ),
        "reps" => format!("{reps} {reps_label}"),
        "duration" => format!("{seconds} {secs_label}"),
        "reps_and_duration" => format!("{reps} {reps_label} × {seconds} {secs_label}"),
        "cardio" => {
            let minutes = set
                .seconds
                .map_or_else(String::new, |value| (value / 60).to_string());
            let distance = set
                .distance_m
                .map_or_else(String::new, |value| trim(value / 1000.0));
            format!(
                "{minutes} {} · {distance} {}",
                i18n.t("common.labels.min"),
                i18n.t("common.labels.km")
            )
        }
        _ => String::new(),
    }
}

fn history_date(i18n: &I18n, epoch: i64) -> String {
    Local
        .timestamp_opt(epoch, 0)
        .single()
        .map_or_else(String::new, |moment| {
            let date = moment.date_naive();
            format!(
                "{} {}, {}",
                i18n.month_short(date.month()),
                date.day(),
                date.year()
            )
        })
}

fn set_orm(weight: Option<f64>, reps: Option<i64>) -> Option<f64> {
    let weight = weight?;
    let reps = reps?;
    if weight <= 0.0 || !(1..=30).contains(&reps) {
        return None;
    }
    Some(weight * (1.0 + f64::from(i32::try_from(reps).unwrap_or(0)) / 30.0))
}

fn option_text(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

#[component]
fn GuideTab(id: i64, detail: db::ExerciseDetail) -> Element {
    let i18n = use_i18n();
    rsx! {
        div { class: "px-5",
            if media::animation(id).is_some() {
                img {
                    src: "/media/gifs/{id}.webp",
                    alt: "{detail.name}",
                    class: "mt-5 aspect-video w-full rounded-2xl bg-tile object-cover",
                }
            }
            div { class: "mt-6",
                Spec {
                    label: i18n.t("exercises.detail.main"),
                    value: join_or_dash(&translated(&i18n, &detail.muscles_main)),
                    art: RowArt::Image(first_muscle_art(&detail.muscles_main)),
                }
                if !detail.muscles_secondary.is_empty() {
                    Spec {
                        label: i18n.t("exercises.detail.secondary"),
                        value: translated(&i18n, &detail.muscles_secondary).join(", "),
                        art: RowArt::Image(first_muscle_art(&detail.muscles_secondary)),
                        }
                }
                Spec {
                    label: i18n.t("exercises.detail.equipment"),
                    value: db::json_list(&detail.equipment_json)
                        .into_iter()
                        .map(|name| i18n.value("exercises.equipment", name))
                        .collect::<Vec<_>>()
                        .join(", "),
                    art: RowArt::Image(
                        db::json_list(&detail.equipment_json)
                            .first()
                            .map_or(BODY_ART, |name| equipment_art(name)),
                    ),
                }
                Spec {
                    label: i18n.t("exercises.detail.difficulty"),
                    value: detail
                        .difficulty
                        .clone()
                        .map_or_else(String::new, |level| i18n.value("exercises.levels", &level)),
                    art: RowArt::Gauge,
                }
                Spec {
                    label: i18n.t("exercises.detail.type"),
                    value: i18n.value("exercises.types", &detail.exercise_type),
                    art: RowArt::Tag,
                }
            }
            p { class: "mt-8 whitespace-pre-line pb-12 text-[15px] leading-7 text-muted",
                "{detail.description}"
            }
        }
    }
}

fn translated(i18n: &I18n, names: &[String]) -> Vec<String> {
    names
        .iter()
        .map(|name| i18n.value("exercises.muscles", name))
        .collect()
}

fn join_or_dash(names: &[String]) -> String {
    if names.is_empty() {
        "—".to_string()
    } else {
        names.join(", ")
    }
}

fn first_muscle_art(names: &[String]) -> Asset {
    names
        .first()
        .and_then(|name| art_for(Some(name)))
        .unwrap_or(BODY_ART)
}

pub fn equipment_art(name: &str) -> Asset {
    let key = name.to_lowercase().replace(' ', "_");
    EQUIPMENT_ART
        .iter()
        .find(|(known, _)| *known == key)
        .map_or(BODY_ART, |(_, art)| *art)
}

pub fn art_for(main: Option<&str>) -> Option<Asset> {
    GROUPS
        .iter()
        .find(|group| Some(group.label) == main)
        .map(|group| group.art)
}

pub fn matches_filters(
    i18n: &I18n,
    row: &db::Exercise,
    needle: &str,
    equipment: &[String],
    muscles: &[String],
    difficulty: &[String],
) -> bool {
    if !needle.is_empty()
        && !row.name.to_lowercase().contains(needle)
        && !row
            .muscles
            .iter()
            .any(|muscle| muscle.to_lowercase().contains(needle))
        && !row.muscles.iter().any(|muscle| {
            i18n.value("exercises.muscles", muscle)
                .to_lowercase()
                .contains(needle)
        })
        && !row.body_parts_json.to_lowercase().contains(needle)
    {
        return false;
    }
    if !difficulty.is_empty()
        && row
            .difficulty
            .as_ref()
            .is_none_or(|level| !difficulty.contains(level))
    {
        return false;
    }
    (equipment.is_empty()
        || equipment
            .iter()
            .any(|item| row.equipment_json.contains(item.as_str())))
        && (muscles.is_empty() || muscles.iter().any(|item| row.muscles.contains(item)))
}

pub fn equipment_options(rows: &[db::Exercise], i18n: &I18n) -> Vec<String> {
    let mut options = rows
        .iter()
        .flat_map(|row| db::json_list(&row.equipment_json))
        .map(str::to_string)
        .collect::<Vec<_>>();
    options.sort_unstable();
    options.dedup();
    options.sort_by_cached_key(|option| i18n.value("exercises.equipment", option));
    options
}

pub fn summary(i18n: &I18n, row: &db::Exercise) -> String {
    row.muscles
        .iter()
        .take(3)
        .map(|muscle| i18n.value("exercises.muscles", muscle))
        .collect::<Vec<_>>()
        .join(", ")
}

fn toggle(mut list: Signal<Vec<String>>, value: &str) {
    list.with_mut(|items| {
        if let Some(position) = items.iter().position(|item| item == value) {
            items.remove(position);
        } else {
            items.push(value.to_string());
        }
    });
}

pub fn thousands(n: usize) -> String {
    if n < 1000 {
        n.to_string()
    } else {
        format!("{},{}", n / 1000, n % 1000)
    }
}

#[component]
pub fn ExerciseThumb(id: i64, size: u32) -> Element {
    let style = format!("width:{size}px;height:{size}px");
    const RINGS: [&str; 5] = [
        "ring-mango/40",
        "ring-sky/40",
        "ring-lilac/40",
        "ring-lime/40",
        "ring-accent/40",
    ];
    let tone = RINGS[id.rem_euclid(5) as usize];
    rsx! {
        div { style: "{style}", class: "shrink-0 overflow-hidden rounded-full bg-tile ring-2 {tone}",
            img {
                src: "/media/stills/{id}.webp",
                alt: "",
                loading: "lazy",
                decoding: "async",
                class: "h-full w-full object-cover",
            }
        }
    }
}

#[component]
pub(crate) fn Thumb(art: Option<Asset>, size: u32) -> Element {
    let style = format!("width:{size}px;height:{size}px");
    art.map_or_else(
        || {
            rsx! {
                div {
                    style: "{style}",
                    class: "grid shrink-0 place-items-center rounded-full bg-tile",
                    Icon { icon: LdBan, width: 24, height: 24, class: "text-muted" }
                }
            }
        },
        |art| {
            rsx! {
                div { style: "{style}", class: "shrink-0 overflow-hidden rounded-full",
                    img { src: art, alt: "", class: "h-full w-full object-cover" }
                }
            }
        },
    )
}

#[component]
fn FacetChip(
    label: String,
    active: bool,
    opened: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: if active {
                "flex h-11 shrink-0 items-center gap-1.5 rounded-xl bg-sky px-4 text-sm font-medium text-black"
            } else if opened {
                "flex h-11 shrink-0 items-center gap-1.5 rounded-xl border border-sky px-4 text-sm font-medium text-lead"
            } else {
                "flex h-11 shrink-0 items-center gap-1.5 rounded-xl border border-line px-4 text-sm font-medium text-lead"
            },
            onclick,
            "{label}"
            Icon { icon: LdChevronDown, width: 16, height: 16 }
        }
    }
}

#[component]
fn MenuOption(
    label: String,
    art: RowArt,
    selected: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button {
            class: if selected {
                "flex w-full items-center gap-3 rounded-xl bg-sky/10 px-3 py-2 text-start text-[15px]"
            } else {
                "flex w-full items-center gap-3 rounded-xl px-3 py-2 text-start text-[15px]"
            },
            onclick,
            match art {
                RowArt::Image(art) => rsx! {
                    div { class: "size-8 shrink-0 overflow-hidden rounded-full",
                        img { src: art, alt: "", class: "h-full w-full object-cover" }
                    }
                },
                RowArt::Gauge => rsx! {
                    div { class: "grid size-8 shrink-0 place-items-center rounded-full bg-surface text-muted",
                        Icon { icon: LdGauge, width: 16, height: 16 }
                    }
                },
                RowArt::Tag => rsx! {
                    div { class: "grid size-8 shrink-0 place-items-center rounded-full bg-surface text-muted",
                        Icon { icon: LdTag, width: 16, height: 16 }
                    }
                },
            }
            span {
                class: if selected { "flex-1 text-sky" } else { "flex-1 text-lead" },
                "{label}"
            }
            if selected {
                Icon { icon: LdCheck, width: 18, height: 18, class: "text-sky" }
            }
        }
    }
}

#[component]
fn Spec(label: String, value: String, art: RowArt) -> Element {
    rsx! {
        div { class: "flex items-center justify-between gap-6 border-b border-line py-3",
            div { class: "flex items-center gap-3",
                match art {
                    RowArt::Image(art) => rsx! {
                        div { class: "size-9 shrink-0 overflow-hidden rounded-full",
                            img { src: art, alt: "", class: "h-full w-full object-cover" }
                        }
                    },
                    RowArt::Gauge => rsx! {
                        div { class: "grid size-9 shrink-0 place-items-center rounded-full bg-tile text-muted",
                            Icon { icon: LdGauge, width: 18, height: 18 }
                        }
                    },
                    RowArt::Tag => rsx! {
                        div { class: "grid size-9 shrink-0 place-items-center rounded-full bg-tile text-muted",
                            Icon { icon: LdTag, width: 18, height: 18 }
                        }
                    },
                }
                span { class: "text-sm text-muted", "{label}" }
            }
            span { class: "text-end text-[15px] font-medium capitalize", "{value}" }
        }
    }
}
