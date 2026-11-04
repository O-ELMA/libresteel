use dioxus::prelude::*;
use dioxus_free_icons::{
    icons::ld_icons::{LdClipboardList, LdDumbbell, LdGauge, LdSettings},
    Icon,
};

use crate::{
    pages::body::Body,
    pages::calendar::DayLog,
    pages::data::DataTransfer,
    pages::energy::BodyEnergy,
    pages::exercises::{ExerciseDetail, ExerciseDetailTab, ExerciseList, Exercises},
    i18n::use_i18n,
    pages::overview::Overview,
    pages::progress::{MuscleProgress, TimeProgress, VolumeProgress},
    pages::routines::Routines,
    pages::session::{Session, SessionSummary},
    pages::settings::Settings,
    pages::workout::{RoutineDay, RoutineDayAdd, RoutineExerciseSwap, Workout},
};

#[derive(Debug, Clone, Routable, PartialEq, Eq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(Shell)]
    #[route("/")]
    Overview {},
    #[route("/body")]
    Body {},
    #[route("/body/energy")]
    BodyEnergy {},
    #[route("/settings")]
    Settings {},
    #[route("/settings/data")]
    DataTransfer {},
    #[route("/workout")]
    Workout {},
    #[route("/workout/routines")]
    Routines {},
    #[route("/workout/day/:day_id")]
    RoutineDay { day_id: i64 },
    #[route("/workout/day/:day_id/add")]
    RoutineDayAdd { day_id: i64 },
    #[route("/workout/exercise/:entry/swap")]
    RoutineExerciseSwap { entry: i64 },
    #[route("/exercises")]
    Exercises {},
    #[route("/exercises/detail/:id")]
    ExerciseDetail { id: i64 },
    #[route("/exercises/detail/:id/:tab")]
    ExerciseDetailTab { id: i64, tab: String },
    #[route("/exercises/:group")]
    ExerciseList { group: String },
    #[route("/calendar/:date")]
    DayLog { date: String },
    #[route("/progress/volume")]
    VolumeProgress {},
    #[route("/progress/time")]
    TimeProgress {},
    #[route("/progress/muscles")]
    MuscleProgress {},
    #[route("/session")]
    Session {},
    #[route("/session/complete")]
    SessionSummary {},
}

#[component]
fn Shell() -> Element {
    let i18n = use_i18n();
    let route = use_route::<Route>();

    #[cfg(target_os = "android")]
    {
        let closes = use_context::<crate::components::ui::Sheets>().closes;
        let navigator = use_navigator();
        use_future(move || async move {
            let mut eval = document::eval(
                r#"
                history.pushState({ guard: true }, "");
                window.addEventListener("popstate", () => {
                    dioxus.send("back");
                    setTimeout(() => history.pushState({ guard: true }, ""), 0);
                });
                "#,
            );
            while eval.recv::<String>().await.is_ok() {
                if let Some((_, close)) = closes.cloned().last().copied() {
                    close.call(());
                    continue;
                }
                if navigator.can_go_back() {
                    navigator.go_back();
                } else {
                    finish_activity();
                }
            }
        });

        use_future(move || async move {
            // A killed process leaves its last notification in the tray; start from a clean slate.
            crate::notify::clear();
            let mut posted = false;
            loop {
                let live = match crate::db::active_session() {
                    Ok(session) => session.map(|session| {
                        let row = crate::components::active_session::current_row(&session);
                        let name = row
                            .as_ref()
                            .map_or_else(|| session.name.clone(), |row| row.name.clone());
                        let rest = session.rest_until.and_then(|deadline| {
                            let remaining = deadline.saturating_sub(crate::db::now());
                            let total = crate::db::rest_total(&session).unwrap_or_else(|err| {
                                dioxus::logger::tracing::error!("rest total failed: {err:#}");
                                0
                            });
                            (remaining > 0 && total > 0).then_some((remaining, total))
                        });
                        let body = rest.map_or_else(
                            || i18n.t("session.startSet"),
                            |(remaining, _)| {
                                format!(
                                    "{} {:02}:{:02}",
                                    i18n.t("session.rest"),
                                    remaining / 60,
                                    remaining % 60
                                )
                            },
                        );
                        (name, body, session.started_at, rest)
                    }),
                    Err(err) => {
                        dioxus::logger::tracing::error!("active session failed: {err:#}");
                        None
                    }
                };
                if let Some((name, body, started_at, rest)) = &live {
                    crate::notify::show(name, body, *started_at, *rest);
                    posted = true;
                } else if posted {
                    crate::notify::clear();
                    posted = false;
                }
                futures_timer::Delay::new(std::time::Duration::from_secs(1)).await;
            }
        });

        let start = route.clone();
        use_future(move || {
            let start = start.clone();
            async move {
                if !matches!(start, Route::Session {} | Route::SessionSummary {})
                    && matches!(crate::db::active_session(), Ok(Some(_)))
                {
                    navigator.push(Route::Session {});
                }
            }
        });
    }

    let live = matches!(
        route,
        Route::Session {}
            | Route::SessionSummary {}
            | Route::VolumeProgress {}
            | Route::TimeProgress {}
            | Route::MuscleProgress {}
    );
    let page = format!("{route:?}");
    let tab = match route {
        Route::Overview {}
        | Route::Body {}
        | Route::BodyEnergy {}
        | Route::Session {}
        | Route::VolumeProgress {}
        | Route::TimeProgress {}
        | Route::MuscleProgress {}
        | Route::DayLog { .. } => 0,
        Route::Workout {}
        | Route::Routines {}
        | Route::RoutineDay { .. }
        | Route::RoutineDayAdd { .. }
        | Route::RoutineExerciseSwap { .. }
        | Route::SessionSummary {} => 1,
        Route::Exercises {}
        | Route::ExerciseList { .. }
        | Route::ExerciseDetail { .. }
        | Route::ExerciseDetailTab { .. } => 2,
        Route::Settings {} | Route::DataTransfer {} => 3,
    };

    rsx! {
        div { class: "min-h-dvh bg-surface text-lead",
            div {
                class: if live { "mx-auto w-full max-w-md" } else { "mx-auto w-full max-w-md pb-24" },
                for key in [page] {
                    div { key: "{key}", class: "page-shell",
                        Outlet::<Route> {}
                    }
                }
            }
            if !live {
                nav {
                    class: "fixed bottom-0 left-1/2 flex w-full max-w-md -translate-x-1/2 justify-around border-t border-line bg-surface pt-1.5 pb-[calc(env(safe-area-inset-bottom)+0.5rem)]",
                    Tab {
                        to: Route::Overview {},
                        active: tab == 0,
                        Icon { icon: LdGauge, width: 26, height: 26 }
                        span { "{i18n.t(\"nav.overview\")}" }
                    }
                    Tab {
                        to: Route::Workout {},
                        active: tab == 1,
                        Icon { icon: LdClipboardList, width: 26, height: 26 }
                        span { "{i18n.t(\"nav.workout\")}" }
                    }
                    Tab {
                        to: Route::Exercises {},
                        active: tab == 2,
                        Icon { icon: LdDumbbell, width: 26, height: 26 }
                        span { "{i18n.t(\"nav.exercises\")}" }
                    }
                    Tab {
                        to: Route::Settings {},
                        active: tab == 3,
                        Icon { icon: LdSettings, width: 26, height: 26 }
                        span { "{i18n.t(\"nav.settings\")}" }
                    }
                }
            }
        }
    }
}

#[component]
fn Tab(to: Route, active: bool, children: Element) -> Element {
    rsx! {
        Link {
            to,
            class: if active {
                "flex flex-col items-center gap-1.5 px-4 py-1 text-[13px] font-medium text-accent-ink"
            } else {
                "flex flex-col items-center gap-1.5 px-4 py-1 text-[13px] font-medium text-muted"
            },
            {children}
        }
    }
}

#[cfg(target_os = "android")]
fn finish_activity() {
    use jni::{objects::JObject, JavaVM};

    let ctx = ndk_context::android_context();
    // SAFETY: tao initialises ndk-context with the live JVM and application Context before launch.
    let Ok(vm) = (unsafe { JavaVM::from_raw(ctx.vm().cast()) }) else {
        return;
    };
    let Ok(mut env) = vm.attach_current_thread() else {
        return;
    };
    // SAFETY: same source as above; the reference is not kept past this function.
    let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
    let _ = env.call_method(&activity, "finish", "()V", &[]);
}
