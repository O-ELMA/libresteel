use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

use dioxus::prelude::*;

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
