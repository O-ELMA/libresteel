use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonTone {
    #[default]
    Lilac,
    White,
    Danger,
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Md,
    #[default]
    Lg,
    Wide,
    Xl,
    Bare,
}

#[component]
pub fn Button(
    #[props(default)] tone: ButtonTone,
    #[props(default)] size: ButtonSize,
    #[props(default)] disabled: bool,
    #[props(default)] class: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let size = match size {
        ButtonSize::Md => "h-11 rounded-xl px-4 text-sm font-semibold",
        ButtonSize::Lg => "h-12 w-full rounded-2xl px-4 text-base font-semibold",
        ButtonSize::Wide => "h-12 w-full rounded-xl px-4 text-sm font-semibold",
        ButtonSize::Xl => "h-14 rounded-2xl px-4 text-base font-semibold",
        ButtonSize::Bare => "text-sm font-semibold",
    };
    let tone = if disabled {
        "bg-tile text-muted"
    } else {
        match tone {
            ButtonTone::Lilac => "bg-lilac text-black",
            ButtonTone::White => "bg-white/10 text-lead",
            ButtonTone::Danger => "bg-red-500 text-black",
            ButtonTone::Ghost => "text-accent-ink",
        }
    };
    rsx! {
        button {
            class: "{size} {tone} {class}",
            disabled,
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum IconTone {
    #[default]
    Muted,
    Lead,
    Tile,
    TileLead,
    White,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum IconSize {
    #[default]
    Md,
    Lg,
    Xl,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum IconShape {
    #[default]
    Bare,
    Rounded,
    Full,
}

#[component]
pub fn IconButton(
    #[props(default)] tone: IconTone,
    #[props(default)] size: IconSize,
    #[props(default)] shape: IconShape,
    #[props(default)] label: String,
    #[props(default)] class: String,
    #[props(default)] onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let size = match size {
        IconSize::Md => "size-10",
        IconSize::Lg => "size-11",
        IconSize::Xl => "size-14",
    };
    let shape = match shape {
        IconShape::Bare => "",
        IconShape::Rounded => "rounded-xl",
        IconShape::Full => "rounded-full",
    };
    let tone = match tone {
        IconTone::Muted => "text-muted",
        IconTone::Lead => "text-lead",
        IconTone::Tile => "bg-tile text-muted",
        IconTone::TileLead => "bg-tile text-lead",
        IconTone::White => "bg-white/10 text-lead",
    };
    rsx! {
        button {
            class: "grid {size} place-items-center {shape} {tone} {class}",
            "aria-label": "{label}",
            onclick: move |event| onclick.call(event),
            {children}
        }
    }
}
