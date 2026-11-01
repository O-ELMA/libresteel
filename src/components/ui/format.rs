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

pub fn option_text(value: Option<impl std::fmt::Display>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

pub fn minutes_text(seconds: Option<i64>) -> String {
    seconds.map_or_else(String::new, |seconds| (seconds / 60).to_string())
}

pub fn km_text(distance_m: Option<f64>) -> String {
    distance_m.map_or_else(String::new, |distance| (distance / 1000.0).to_string())
}

pub fn to_f64(value: i64) -> f64 {
    f64::from(i32::try_from(value).unwrap_or(i32::MAX))
}

pub fn set_orm(weight: Option<f64>, reps: Option<i64>) -> Option<f64> {
    let weight = weight?;
    let reps = reps?;
    if weight <= 0.0 || !(1..=30).contains(&reps) {
        return None;
    }
    Some(weight * (1.0 + f64::from(i32::try_from(reps).unwrap_or(0)) / 30.0))
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

pub fn quiet<T: Default>(result: anyhow::Result<T>) -> T {
    result.unwrap_or_else(|err| {
        dioxus::logger::tracing::error!("query failed: {err:#}");
        T::default()
    })
}

pub fn long_clock(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = seconds % 3600 / 60;
    let rest = seconds % 60;
    format!("{hours}:{minutes:02}:{rest:02}")
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
