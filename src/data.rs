use dioxus::prelude::*;

use crate::{
    db,
    i18n::{use_i18n, I18n},
    transfer::{self, Backup, Counts},
    ui::{use_exit, BackIcon, Sheet},
};

#[derive(Clone, PartialEq)]
struct Preview {
    backup: Backup,
    file_name: String,
    current: Counts,
}

#[component]
pub fn DataTransfer() -> Element {
    let i18n = use_i18n();
    let navigator = use_navigator();
    let mut preview = use_signal(|| None::<Preview>);
    let mut report = use_signal(|| None::<Counts>);
    let mut error = use_signal(|| None::<String>);
    let mut notice = use_signal(|| None::<String>);
    let mut asking = use_signal(|| false);
    let (asking_closing, close_asking) = use_exit(asking, false, |value| *value);

    use_effect(move || {
        let _ = document::eval(
            r"
            if (!window.__jefeetHook) {
              window.__jefeetHook = true;
              window.__jefeetRead = null;
              const openPicker = () => {
                window.__jefeetRead = null;
                const input = document.createElement('input');
                input.type = 'file';
                input.accept = '.csv,text/csv';
                input.style.display = 'none';
                const cleanup = () => {
                  if (input.parentNode) input.parentNode.removeChild(input);
                };
                input.addEventListener('change', () => {
                  const file = input.files && input.files[0];
                  if (!file) { cleanup(); return; }
                  const reader = new FileReader();
                  reader.onload = () => {
                    let text = '';
                    let bytes = 0;
                    if (typeof reader.result === 'string') {
                      text = reader.result;
                      bytes = reader.result.length;
                    } else if (reader.result) {
                      text = new TextDecoder('utf-8').decode(new Uint8Array(reader.result));
                      bytes = reader.result.byteLength;
                    }
                    window.__jefeetRead = {
                      name: file.name,
                      size: file.size,
                      bytes,
                      text,
                      error: null,
                    };
                    cleanup();
                  };
                  reader.onerror = () => {
                    window.__jefeetRead = {
                      name: file.name,
                      size: file.size,
                      bytes: 0,
                      text: '',
                      error: String(reader.error || 'FileReader failed'),
                    };
                    cleanup();
                  };
                  reader.readAsArrayBuffer(file);
                });
                document.body.appendChild(input);
                input.click();
              };
              document.addEventListener('click', (event) => {
                const target = event.target;
                if (!(target instanceof Element)) return;
                if (!target.closest('#jefeet-pick')) return;
                openPicker();
              }, true);
            }
            ",
        );
    });

    let export = move |_| {
        error.set(None);
        notice.set(None);
        let result = transfer::export().and_then(|contents| {
            let name = format!("libresteel-{}.csv", chrono::Local::now().format("%Y-%m-%d-%H%M"));
            transfer::save_file(&name, &contents)
        });
        match result {
            Ok(path) => notice.set(Some(i18n.ta("data.saved", &[("path", &path)]))),
            Err(err) => error.set(Some(
                i18n.ta("data.saveFailed", &[("message", &err.to_string())]),
            )),
        }
    };

    let template = move |_| {
        error.set(None);
        notice.set(None);
        match transfer::save_file("libresteel-template.csv", &transfer::template()) {
            Ok(path) => notice.set(Some(i18n.ta("data.saved", &[("path", &path)]))),
            Err(err) => error.set(Some(
                i18n.ta("data.saveFailed", &[("message", &err.to_string())]),
            )),
        }
    };

    let choose = move |_| {
        dioxus::prelude::dioxus_core::spawn_isomorphic(async move {
            let result = read_file().await;
            error.set(None);
            notice.set(None);
            report.set(None);
            preview.set(None);
            match result {
                Ok((name, text)) => {
                    if text.is_empty() {
                        error.set(Some(i18n.t("data.emptyFile")));
                        return;
                    }
                    if text == "__too_big__" {
                        error.set(Some(i18n.t("data.fileTooBig")));
                        return;
                    }
                    if text.starts_with("__read_error__") {
                        dioxus::logger::tracing::info!("import read failed: {text}");
                        error.set(Some(i18n.t("data.emptyFile")));
                        return;
                    }
                    match transfer::parse(&text) {
                        Ok(backup) if backup.is_empty() => {
                            error.set(Some(i18n.t("data.noRows")));
                        }
                        Ok(backup) => match transfer::current_counts() {
                            Ok(current) => {
                                preview.set(Some(Preview {
                                    backup,
                                    file_name: name,
                                    current,
                                }));
                                asking.set(true);
                            }
                            Err(err) => error.set(Some(err.to_string())),
                        },
                        Err(errors) => {
                            error.set(Some(format!("{}\n{errors}", i18n.t("data.errors"))));
                        }
                    }
                }
                Err(err) => {
                    error.set(Some(err));
                }
            }
        });
    };

    let do_replace = EventHandler::new(move |()| {
        let Some(item) = preview.cloned() else {
            return;
        };
        match db::active_session() {
            Ok(Some(_)) => {
                error.set(Some(i18n.t("data.activeSession")));
                close_asking.call(());
            }
            Ok(None) => match transfer::replace(&item.backup) {
                Ok(()) => {
                    report.set(Some(item.backup.counts()));
                    error.set(None);
                    notice.set(None);
                    let mut language = i18n.language;
                    language.set(db::language());
                    close_asking.call(());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                    close_asking.call(());
                }
            },
            Err(err) => {
                error.set(Some(err.to_string()));
                close_asking.call(());
            }
        }
    });

    let pending = preview.cloned();
    let current_text = pending
        .as_ref()
        .map(|item| counts_text(&i18n, "data.previewCurrent", item.current));
    let new_text = pending
        .as_ref()
        .map(|item| counts_text(&i18n, "data.previewNew", item.backup.counts()));
    let report_text = report
        .cloned()
        .map(|counts| counts_text(&i18n, "data.imported", counts));

    rsx! {
        header { class: "flex items-center gap-3 px-5 pt-5",
            button {
                class: "text-lead",
                onclick: move |_| navigator.go_back(),
                BackIcon { size: 24 }
            }
            h1 { class: "text-[22px] font-bold tracking-tight", "{i18n.t(\"data.title\")}" }
        }
        div { class: "px-5 pb-10",
            p { class: "mt-4 text-sm leading-6 text-muted", "{i18n.t(\"data.intro\")}" }

            div { class: "mt-5 rounded-2xl bg-tile p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"data.export\")}" }
                p { class: "mt-1 text-sm leading-6 text-muted", "{i18n.t(\"data.exportNote\")}" }
                button {
                    class: "mt-3 h-12 w-full rounded-2xl bg-lilac text-base font-semibold text-black",
                    onclick: export,
                    "{i18n.t(\"data.export\")}"
                }
            }

            div { class: "mt-4 rounded-2xl bg-tile p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"data.template\")}" }
                p { class: "mt-1 text-sm leading-6 text-muted", "{i18n.t(\"data.templateNote\")}" }
                button {
                    class: "mt-3 h-12 w-full rounded-2xl bg-white/10 text-base font-semibold",
                    onclick: template,
                    "{i18n.t(\"data.template\")}"
                }
            }

            div { class: "mt-4 rounded-2xl bg-tile p-4",
                h2 { class: "text-base font-semibold", "{i18n.t(\"data.import\")}" }
                p { class: "mt-1 text-sm leading-6 text-muted", "{i18n.t(\"data.importNote\")}" }
                button {
                    id: "jefeet-pick",
                    class: "mt-3 flex h-12 w-full items-center justify-center rounded-2xl bg-lilac text-base font-semibold text-black",
                    onclick: choose,
                    "{i18n.t(\"data.choose\")}"
                }
            }

            if let Some(message) = notice.cloned() {
                p { class: "mt-4 rounded-xl bg-tile p-3 text-sm leading-6 text-green-400",
                    "{message}"
                }
            }
            if let Some(message) = error.cloned() {
                p { class: "mt-4 whitespace-pre-line rounded-xl bg-red-500/15 p-3 text-sm leading-6 text-red-300",
                    "{message}"
                }
            }
            if let Some(message) = report_text {
                p { class: "mt-4 rounded-xl bg-tile p-3 text-sm leading-6 text-green-400",
                    "{message}"
                }
            }

            h2 { class: "mt-7 text-sm font-semibold text-muted", "{i18n.t(\"data.format\")}" }
            div { class: "mt-2 rounded-2xl bg-tile p-4",
                p { class: "text-sm leading-6 text-muted", "{i18n.t(\"data.formatNote\")}" }
                for (key, label) in [
                    ("settings", i18n.t("data.sections.settings")),
                    ("measurements", i18n.t("data.sections.measurements")),
                    ("plans", i18n.t("data.sections.plans")),
                    ("workouts", i18n.t("data.sections.workouts")),
                ] {
                    div { key: "{key}", class: "mt-3",
                        div { class: "text-sm font-semibold", "{label}" }
                    }
                }
                p { class: "mt-4 text-sm leading-6 text-muted", "{i18n.t(\"data.unitsNote\")}" }
            }
        }

        if asking() {
            Sheet {
                closing: asking_closing(),
                onclose: close_asking,
                div { class: "px-3 pb-2",
                    h2 { class: "text-lg font-semibold", "{i18n.t(\"data.previewTitle\")}" }
                    if let Some(item) = pending.as_ref() {
                        p { class: "mt-1 truncate text-sm text-muted", "{item.file_name}" }
                    }
                    if let Some(text) = current_text.as_ref() {
                        p { class: "mt-3 text-sm leading-6", "{text}" }
                    }
                    if let Some(text) = new_text.as_ref() {
                        p { class: "mt-1 text-sm leading-6", "{text}" }
                    }
                    p { class: "mt-3 text-sm leading-6 text-muted", "{i18n.t(\"data.previewDeletes\")}" }
                    div { class: "mt-5 flex gap-2",
                        button {
                            class: "h-11 flex-1 rounded-xl bg-white/10 text-sm font-medium",
                            onclick: move |_| close_asking.call(()),
                            "{i18n.t(\"data.cancel\")}"
                        }
                        button {
                            class: "h-11 flex-1 rounded-xl bg-red-500 px-2 text-sm font-semibold text-black",
                            onclick: move |_| do_replace.call(()),
                            "{i18n.t(\"data.replace\")}"
                        }
                    }
                }
            }
        }
    }
}

fn counts_text(i18n: &I18n, key: &str, counts: Counts) -> String {
    i18n.ta(
        key,
        &[
            ("plans", &counts.plans.to_string()),
            ("workouts", &counts.workouts.to_string()),
            ("measurements", &counts.measurements.to_string()),
        ],
    )
}

#[allow(clippy::future_not_send)]
async fn read_file() -> Result<(String, String), String> {
    let mut eval = document::eval(
        r"
        (async () => {
          const result = await new Promise((resolve) => {
            const started = Date.now();
            const check = () => {
              if (window.__jefeetRead) {
                const read = window.__jefeetRead;
                window.__jefeetRead = null;
                resolve(read);
                return;
              }
              if (Date.now() - started > 120000) {
                resolve({ name: '', text: '', size: 0, error: 'timed out waiting for the file' });
                return;
              }
              setTimeout(check, 20);
            };
            check();
          });
          const sizes = String(result.size) + '/' + String(result.bytes || 0);
          if (result.error) {
            dioxus.send(result.name);
            dioxus.send('__read_error__: ' + result.error);
            dioxus.send(sizes);
            return;
          }
          if (result.size > 8388608) {
            dioxus.send(result.name);
            dioxus.send('__too_big__');
            dioxus.send(sizes);
            return;
          }
          dioxus.send(result.name);
          dioxus.send(result.text);
          dioxus.send(sizes);
        })();
        ",
    );
    let name = eval
        .recv::<String>()
        .await
        .map_err(|error| error.to_string())?;
    let text = eval
        .recv::<String>()
        .await
        .map_err(|error| error.to_string())?;
    let size = eval
        .recv::<String>()
        .await
        .map_err(|error| error.to_string())?;
    dioxus::logger::tracing::info!(
        "import picked {name:?}, size={size}, {} chars, first {:?}",
        text.chars().count(),
        text.chars().take(40).collect::<String>()
    );
    Ok((name, text))
}
