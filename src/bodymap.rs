use std::collections::HashMap;

use dioxus::prelude::*;

const BODY: &str = include_str!("../assets/muscles/body.svg");

#[rustfmt::skip]
const BODY_GROUPS: [(&str, &str); 12] = [
    ("abs", "Abs"),
    ("upper_legs", "Upper Legs"),
    ("back", "Back"),
    ("shoulders", "Shoulders"),
    ("chest", "Chest"),
    ("biceps", "Biceps"),
    ("triceps", "Triceps"),
    ("forearms", "Forearms"),
    ("lower_legs", "Lower Legs"),
    ("glutes", "Glutes"),
    ("hamstrings", "Upper Legs"),
    ("calves", "Lower Legs"),
];

#[component]
pub fn BodyMap(counts: Vec<(String, i64)>) -> Element {
    let mut regions = HashMap::<&'static str, i64>::new();
    for (label, count) in &counts {
        for (key, name) in BODY_GROUPS {
            if name == label {
                let region = regions.entry(key).or_insert(0);
                *region = region.saturating_add(*count);
            }
        }
    }
    rsx! {
        div { class: "flex justify-center", dangerous_inner_html: tinted(BODY, &regions) }
    }
}

fn tinted(svg: &str, counts: &HashMap<&'static str, i64>) -> String {
    let max = counts.values().copied().max().unwrap_or(0).max(1);
    let mut out = svg.replace("<title>Full body, muscles</title>", "");
    for (key, count) in counts {
        let fraction = f64::from(i32::try_from(*count).unwrap_or(1))
            / f64::from(i32::try_from(max).unwrap_or(1));
        let alpha = fraction.mul_add(0.7, 0.3);
        out = out.replace(
            &format!("id=\"muscle-{key}\" fill=\"#C7C7C7\""),
            &format!("id=\"muscle-{key}\" fill=\"#ec834b\" fill-opacity=\"{alpha:.2}\""),
        );
    }
    out
}
