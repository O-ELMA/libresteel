use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdArrowLeft, LdArrowRight, LdChevronLeft, LdChevronRight},
    Icon,
};

use crate::i18n::use_i18n;

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
