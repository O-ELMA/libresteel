use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdBan, LdCheck, LdChevronDown, LdGauge, LdSearch, LdTag, LdX},
    Icon,
};

use crate::{
    components::ui::{use_exit, Sheet},
    db,
    i18n::{use_i18n, I18n},
};

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
pub fn Thumb(art: Option<Asset>, size: u32) -> Element {
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

pub struct Group {
    pub key: &'static str,
    pub label: &'static str,
    pub art: Asset,
}

// clippy::volatile_composites false-fires on asset!(), which embeds &[u8] in a const descriptor.
#[rustfmt::skip]
#[allow(clippy::volatile_composites)]
pub const GROUPS: [Group; 11] = [
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
pub const BODY_ART: Asset = asset!("/assets/equipment/body.png");

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

#[derive(Clone, Copy, PartialEq)]
enum Facet {
    Equipment,
    Muscles,
    Difficulty,
}

#[derive(Clone, PartialEq)]
pub enum RowArt {
    Image(Asset),
    Gauge,
    Tag,
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

fn toggle(mut list: Signal<Vec<String>>, value: &str) {
    list.with_mut(|items| {
        if let Some(position) = items.iter().position(|item| item == value) {
            items.remove(position);
        } else {
            items.push(value.to_string());
        }
    });
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

pub fn thousands(n: usize) -> String {
    if n < 1000 {
        n.to_string()
    } else {
        format!("{},{}", n / 1000, n % 1000)
    }
}
