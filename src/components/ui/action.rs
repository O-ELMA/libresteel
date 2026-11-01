use std::time::Duration;

use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdPencil, LdTrash2},
    Icon,
};

use crate::{components::ui::button::IconButton, i18n::use_i18n};

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
            IconButton {
                label: i18n.t("common.actions.edit"),
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
