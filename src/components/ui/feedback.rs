use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdArrowDownRight, LdArrowUpRight},
    Icon,
};

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
