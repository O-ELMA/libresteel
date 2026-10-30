use std::sync::OnceLock;

use chrono::Weekday;
use dioxus::prelude::*;
use serde_json::Value;

pub const WEEKDAYS: [Weekday; 7] = [
    Weekday::Mon,
    Weekday::Tue,
    Weekday::Wed,
    Weekday::Thu,
    Weekday::Fri,
    Weekday::Sat,
    Weekday::Sun,
];

fn parse(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn map(lang: &str) -> &'static Value {
    static EN: OnceLock<Value> = OnceLock::new();
    static FR: OnceLock<Value> = OnceLock::new();
    static AR: OnceLock<Value> = OnceLock::new();
    match lang {
        "fr" => FR.get_or_init(|| parse(include_str!("i18n/fr.json"))),
        "ar" => AR.get_or_init(|| parse(include_str!("i18n/ar.json"))),
        _ => EN.get_or_init(|| parse(include_str!("i18n/en.json"))),
    }
}

fn walk<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(root, |current, segment| current.get(segment))
}

fn lookup(lang: &str, key: &str) -> Option<String> {
    let value = walk(map(lang), key).or_else(|| walk(map("en"), key))?;
    value.as_str().map(str::to_string)
}

fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    args.iter()
        .fold(template.to_string(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

#[derive(Clone, Copy)]
pub struct I18n {
    pub language: Signal<String>,
}

impl I18n {
    #[must_use]
    pub fn lang(&self) -> String {
        self.language.read().clone()
    }

    #[must_use]
    pub fn t(&self, key: &str) -> String {
        lookup(&self.lang(), key).unwrap_or_else(|| key.to_string())
    }

    #[must_use]
    pub fn ta(&self, key: &str, args: &[(&str, &str)]) -> String {
        interpolate(&self.t(key), args)
    }

    #[must_use]
    pub fn tn(&self, key: &str, count: i64) -> String {
        let suffix = if count == 1 { "_one" } else { "_other" };
        let template =
            lookup(&self.lang(), &format!("{key}{suffix}")).unwrap_or_else(|| self.t(key));
        interpolate(&template, &[("count", &count.to_string())])
    }

    #[must_use]
    pub fn value(&self, group: &str, raw: &str) -> String {
        let key = format!("{group}.{}", raw.trim().to_lowercase().replace(' ', "_"));
        lookup(&self.lang(), &key).unwrap_or_else(|| raw.to_string())
    }

    #[must_use]
    pub fn unit(&self, lb: bool) -> String {
        self.t(if lb {
            "common.labels.lb"
        } else {
            "common.labels.kg"
        })
    }

    #[must_use]
    pub fn month_long(&self, month: u32) -> String {
        self.t(&format!("common.month.{}", month_key(month)))
    }

    #[must_use]
    pub fn month_short(&self, month: u32) -> String {
        self.t(&format!("common.month.{}Short", month_key(month)))
    }

    #[must_use]
    pub fn weekday_short(&self, weekday: Weekday) -> String {
        self.t(&format!("common.weekday.{}Short", weekday_key(weekday)))
    }

    #[must_use]
    pub fn weekday_long(&self, weekday: Weekday) -> String {
        self.t(&format!("common.weekday.{}", weekday_key(weekday)))
    }

    #[must_use]
    pub fn weekdays_short(&self) -> Vec<String> {
        WEEKDAYS
            .into_iter()
            .map(|weekday| self.weekday_short(weekday))
            .collect()
    }
}

#[must_use]
pub fn use_i18n() -> I18n {
    use_context::<I18n>()
}

const fn month_key(month: u32) -> &'static str {
    match month {
        1 => "january",
        2 => "february",
        3 => "march",
        4 => "april",
        5 => "may",
        6 => "june",
        7 => "july",
        8 => "august",
        9 => "september",
        10 => "october",
        11 => "november",
        _ => "december",
    }
}

const fn weekday_key(weekday: Weekday) -> &'static str {
    use chrono::Weekday::{Fri, Mon, Sat, Sun, Thu, Tue, Wed};
    match weekday {
        Mon => "monday",
        Tue => "tuesday",
        Wed => "wednesday",
        Thu => "thursday",
        Fri => "friday",
        Sat => "saturday",
        Sun => "sunday",
    }
}
