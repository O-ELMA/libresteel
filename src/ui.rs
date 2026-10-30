use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{
        LdArrowDownRight, LdArrowLeft, LdArrowRight, LdArrowUpRight, LdChevronLeft, LdChevronRight,
        LdPencil, LdTrash2,
    },
    Icon,
};

use crate::i18n::use_i18n;

pub fn change(current: f64, previous: f64) -> Option<f64> {
    if previous <= 0.0 {
        return None;
    }
    Some((current - previous) / previous * 100.0)
}

pub fn trim(value: f64) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

pub fn duration_label(seconds: i64) -> String {
    let minutes = seconds.saturating_add(30) / 60;
    let hours = minutes / 60;
    let rest = minutes % 60;
    if hours == 0 {
        format!("{rest}m")
    } else if rest == 0 {
        format!("{hours}h")
    } else {
        format!("{hours}h {rest}m")
    }
}

pub const LB_PER_KG: f64 = 2.204_622_621_848_775_7;

pub fn display_weight(kg: f64, lb: bool) -> f64 {
    if lb {
        kg * LB_PER_KG
    } else {
        kg
    }
}

pub fn stored_weight(value: f64, lb: bool) -> f64 {
    if lb {
        value / LB_PER_KG
    } else {
        value
    }
}

pub const fn weight_unit(lb: bool) -> &'static str {
    if lb {
        "lb"
    } else {
        "kg"
    }
}

pub fn weight_text(kg: f64, lb: bool) -> String {
    format!("{} {}", trim(display_weight(kg, lb)), weight_unit(lb))
}

pub fn weight_field(kg: Option<f64>, lb: bool) -> String {
    kg.map_or_else(String::new, |value| trim(display_weight(value, lb)))
}

pub fn parse_weight(raw: &str, lb: bool) -> Option<f64> {
    raw.trim()
        .parse::<f64>()
        .ok()
        .map(|value| stored_weight(value, lb))
}

#[component]
pub fn BackIcon(size: u32) -> Element {
    let i18n = use_i18n();
    rsx! {
        if i18n.lang() == "ar" {
            Icon { icon: LdArrowRight, width: size, height: size }
        } else {
            Icon { icon: LdArrowLeft, width: size, height: size }
        }
    }
}

#[component]
pub fn BodyIcon(size: u32) -> Element {
    rsx! {
        svg {
            width: "{size}",
            height: "{size}",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M3 7V5a2 2 0 0 1 2-2h2" }
            path { d: "M17 3h2a2 2 0 0 1 2 2v2" }
            path { d: "M21 17v2a2 2 0 0 1-2 2h-2" }
            path { d: "M7 21H5a2 2 0 0 1-2-2v-2" }
            circle { cx: "12", cy: "7.2", r: "1.5" }
            path { d: "m9 20 3-5.5 3 5.5" }
            path { d: "m8.8 10.5 3.2 1.6 3.2-1.6" }
            path { d: "M12 12.1v2.4" }
        }
    }
}

#[component]
pub fn ForwardIcon(size: u32) -> Element {
    let i18n = use_i18n();
    rsx! {
        if i18n.lang() == "ar" {
            Icon { icon: LdChevronLeft, width: size, height: size }
        } else {
            Icon { icon: LdChevronRight, width: size, height: size }
        }
    }
}

#[component]
pub fn Delta(percent: f64, #[props(default)] on_light: bool) -> Element {
    let magnitude = percent.abs();
    rsx! {
        span {
            class: if on_light {
                "flex items-center text-sm font-semibold text-black/70"
            } else if percent > 0.0 {
                "flex items-center text-sm font-semibold text-green-400"
            } else if percent < 0.0 {
                "flex items-center text-sm font-semibold text-red-300"
            } else {
                "flex items-center text-sm font-semibold text-muted"
            },
            if percent > 0.0 {
                Icon { icon: LdArrowUpRight, width: 16, height: 16 }
            } else if percent < 0.0 {
                Icon { icon: LdArrowDownRight, width: 16, height: 16 }
            }
            "{magnitude:.0}%"
        }
    }
}

#[component]
pub fn RowActions(vertical: bool, onedit: EventHandler<()>, ondelete: EventHandler<()>) -> Element {
    let i18n = use_i18n();
    let class = if vertical {
        "flex shrink-0 flex-col items-center"
    } else {
        "flex shrink-0 items-center"
    };
    rsx! {
        div { class,
            button {
                class: "grid size-10 place-items-center text-muted",
                "aria-label": i18n.t("common.actions.edit"),
                onclick: move |event: MouseEvent| {
                    event.stop_propagation();
                    onedit.call(());
                },
                Icon { icon: LdPencil, width: 18, height: 18 }
            }
            DeleteButton { class: "grid size-10 place-items-center text-muted", onconfirm: ondelete }
        }
    }
}

#[component]
pub fn DeleteButton(
    #[props(default = 18)] size: u32,
    #[props(default)] label: String,
    class: String,
    onconfirm: EventHandler<()>,
) -> Element {
    let i18n = use_i18n();
    let mut armed = use_signal(|| None::<u64>);
    let mut generation = use_signal(|| 0u64);
    let click = move |event: MouseEvent| {
        event.stop_propagation();
        if armed.try_read().is_ok_and(|value| value.is_some()) {
            armed.set(None);
            onconfirm.call(());
            return;
        }
        let next = generation.cloned().wrapping_add(1);
        generation.set(next);
        armed.set(Some(next));
        dioxus::prelude::spawn(async move {
            futures_timer::Delay::new(Duration::from_secs(3)).await;
            // shortcut: the row is usually unmounted by now after a confirmed delete, and reading a dropped scope panics.
            if armed.try_read().is_ok_and(|value| *value == Some(next)) {
                armed.set(None);
            }
        });
    };
    let ready = armed().is_some();
    let aria = if ready {
        i18n.t("common.actions.confirm")
    } else if label.is_empty() {
        i18n.t("common.actions.delete")
    } else {
        label
    };
    rsx! {
        button {
            class: "relative {class}",
            "aria-label": "{aria}",
            onclick: click,
            Icon { icon: LdTrash2, width: size, height: size }
            if ready {
                span { class: "pointer-events-none absolute top-1/2 left-1/2 z-10 -translate-x-1/2 -translate-y-1/2 whitespace-nowrap rounded-md bg-red-500 px-1.5 py-0.5 text-[11px] font-semibold text-lead",
                    "{i18n.t(\"common.actions.confirm\")}"
                }
            }
        }
    }
}

const EXIT_MS: u64 = 260;

static SHEET_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
pub struct Sheets {
    pub closes: Signal<Vec<(u64, EventHandler<()>)>>,
}

pub fn use_exit<T: Clone + PartialEq + 'static>(
    mut open: Signal<T>,
    closed: T,
    is_open: fn(&T) -> bool,
) -> (Signal<bool>, EventHandler<()>) {
    let mut closing = use_signal(|| false);
    let close = EventHandler::new(move |()| {
        if closing() || !is_open(&open.peek()) {
            return;
        }
        closing.set(true);
        let closed = closed.clone();
        dioxus::prelude::spawn(async move {
            futures_timer::Delay::new(Duration::from_millis(EXIT_MS)).await;
            open.set(closed);
            closing.set(false);
        });
    });
    (closing, close)
}

#[component]
pub fn Sheet(closing: bool, onclose: EventHandler<()>, children: Element) -> Element {
    let state = if closing { " is-closing" } else { "" };
    let mut sheets = use_context::<Sheets>();
    let id = use_hook(|| SHEET_ID.fetch_add(1, Ordering::Relaxed));
    use_hook(move || {
        sheets.closes.with_mut(|list| list.push((id, onclose)));
    });
    use_drop(move || {
        sheets
            .closes
            .with_mut(|list| list.retain(|(entry, _)| *entry != id));
    });
    let mut start = use_signal(|| None::<f64>);
    let mut offset = use_signal(|| 0.0);
    let mut dragging = use_signal(|| false);
    let transition = if dragging() {
        "transform 0s"
    } else {
        "transform 180ms ease-out"
    };
    rsx! {
        div { class: "fixed inset-0 z-40 flex items-end justify-center",
            div {
                class: "absolute inset-0 bg-black/60 sheet-backdrop{state}",
                onclick: move |_| onclose.call(()),
            }
            div { class: "sheet-panel{state} w-full max-w-md",
                div {
                    class: "rounded-t-3xl bg-surface p-2 pb-[calc(env(safe-area-inset-bottom)+1rem)]",
                    style: "transform: translateY({offset}px); transition: {transition};",
                    div {
                        class: "flex cursor-grab touch-none justify-center py-1",
                        onpointerdown: move |event: PointerEvent| {
                            start.set(Some(event.data().client_coordinates().y));
                            dragging.set(true);
                        },
                        onpointermove: move |event: PointerEvent| {
                            let Some(from) = start.cloned() else {
                                return;
                            };
                            offset.set((event.data().client_coordinates().y - from).max(0.0));
                        },
                        onpointerup: move |_| {
                            start.set(None);
                            dragging.set(false);
                            if offset() > 80.0 {
                                onclose.call(());
                            } else {
                                offset.set(0.0);
                            }
                        },
                        onpointercancel: move |_| {
                            start.set(None);
                            dragging.set(false);
                            offset.set(0.0);
                        },
                        div { class: "mb-2 h-1 w-10 rounded-full bg-line" }
                    }
                    {children}
                }
            }
        }
    }
}
