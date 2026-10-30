mod body;
mod bodymap;
mod calendar;
mod data;
mod db;
mod energy;
mod exercises;
mod i18n;
mod media;
mod onboarding;
mod overview;
mod progress;
mod routines;
mod session;
mod settings;
mod shell;
mod suggest;
mod transfer;
mod ui;
mod workout;

use std::path::Path;

use dioxus::prelude::*;
use shell::Route;
use ui::Sheets;

#[cfg(feature = "desktop")]
use dioxus::desktop as platform;
#[cfg(all(feature = "mobile", not(feature = "desktop")))]
use dioxus::mobile as platform;

// clippy::volatile_composites false-fires on asset!(), which embeds &[u8] in a const descriptor.
#[allow(clippy::volatile_composites)]
const MAIN_CSS: Asset = asset!("/assets/main.css");
#[allow(clippy::volatile_composites)]
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    // WebKitGTK's DMA-BUF renderer leaves a black window on some GPU/driver combos.
    #[cfg(feature = "desktop")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    if let Err(err) = db::init() {
        dioxus::logger::tracing::error!("database init failed: {err:#}");
    }

    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    platform::use_asset_handler("media", |request, responder| {
        let body = request
            .uri()
            .path()
            .strip_prefix("/media/")
            .filter(|name| {
                Path::new(name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("webp"))
            })
            .and_then(media::file);
        let found = body.is_some();
        let mut response = platform::wry::http::Response::builder()
            .status(if found { 200 } else { 404 })
            .header("Cache-Control", "public, max-age=31536000, immutable");
        if found {
            response = response.header("Content-Type", "image/webp");
        }
        if let Ok(response) = response.body(body.unwrap_or_default()) {
            responder.respond(response);
        }
    });

    let language = use_signal(db::language);
    let i18n = i18n::I18n { language };
    use_context_provider(|| i18n);
    let closes = use_signal(Vec::<(u64, EventHandler<()>)>::new);
    use_context_provider(|| Sheets { closes });
    let mut onboarded = use_signal(db::onboarded);
    use_effect(move || {
        let lang = language();
        let dir = if lang == "ar" { "rtl" } else { "ltr" };
        let _ = document::eval(&format!(
            "document.documentElement.lang = {lang:?}; document.documentElement.dir = {dir:?};"
        ));
    });

    rsx! {
        document::Stylesheet { href: MAIN_CSS }
        document::Stylesheet { href: TAILWIND_CSS }
        if onboarded() {
            Router::<Route> {}
        } else {
            onboarding::Onboarding { ondone: move |()| onboarded.set(true) }
        }
    }
}
