use std::collections::HashMap;

#[cfg(not(target_os = "android"))]
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone};
use rusqlite::{params, Connection, OptionalExtension};

use crate::db;

const VERSION_LINE: &str = "# jefeet-csv 1";
const MAX_PROBLEMS: usize = 20;

struct SectionSpec {
    name: &'static str,
    columns: &'static [&'static str],
    notes: &'static [&'static str],
    example: &'static str,
}

const SECTIONS: [SectionSpec; 9] = [
    SectionSpec {
        name: "settings",
        columns: &[
            "gender",
            "dob",
            "unit",
            "rest_seconds",
            "default_sets",
            "target_reps",
            "first_week_day",
            "language",
        ],
        notes: &[
            "A single row. Blank cells keep the current value.",
            "gender=male|female, dob=YYYY-MM-DD, unit=kg|lb, first_week_day=0 (Mon) .. 6 (Sun), language=en|ar|fr.",
        ],
        example: "male,1971-10-19,kg,60,3,12,0,en",
    },
    SectionSpec {
        name: "measurements",
        columns: &[
            "recorded_at",
            "weight_kg",
            "fat_percent",
            "height_cm",
            "chest_cm",
            "arms_cm",
            "waist_cm",
            "calves_cm",
            "hips_cm",
            "thighs_cm",
            "shoulders_cm",
            "neck_cm",
            "forearms_cm",
        ],
        notes: &[
            "One row per entry. recorded_at accepts a date or a time; a date means noon local.",
            "weight_kg must be above 0, fat_percent is 0..100. Blank cells mean not measured.",
        ],
        example: "2024-11-02,80,23.13,170,,,92,,,,39,",
    },
    SectionSpec {
        name: "plans",
        columns: &[
            "name",
            "level",
            "goal",
            "split",
            "block_weeks",
            "block_enabled",
            "block_started",
            "active",
            "created_at",
        ],
        notes: &[
            "One row per routine. split=numbered|weekday, level=beginner|intermediate|advanced,",
            "goal=cutting|maintaining|bulking|building_strength, active=1 marks the current routine.",
            "Blank block_started and created_at are filled in automatically.",
        ],
        example: "Full Body Program,intermediate,bulking,weekday,4,1,,1,",
    },
    SectionSpec {
        name: "plan_days",
        columns: &["plan", "day", "weekday", "order"],
        notes: &[
            "plan refers to the plans section. order is 1-based; blank counts row order.",
            "weekday=0 (Mon) .. 6 (Sun), only used by weekday routines.",
        ],
        example: "Full Body Program,Lower focused,0,1",
    },
    SectionSpec {
        name: "plan_exercises",
        columns: &[
            "plan",
            "day",
            "order",
            "exercise",
            "exercise_id",
            "rest_seconds",
            "superset",
        ],
        notes: &[
            "order is 1-based inside the day. exercise_id is optional and shown under the exercise name in the app.",
            "superset is a number shared by the exercises in one superset. Blank rest uses the app default.",
        ],
        example: "Full Body Program,Lower focused,1,Barbell Squat,12,120,",
    },
    SectionSpec {
        name: "plan_sets",
        columns: &[
            "plan",
            "day",
            "exercise_order",
            "set",
            "weight",
            "reps",
            "seconds",
            "distance_m",
        ],
        notes: &[
            "exercise_order points at the order column above. set is 1-based; blank counts row order.",
            "weight is a target in kilograms. Blank target cells are allowed.",
        ],
        example: "Full Body Program,Lower focused,1,1,40,8,,",
    },
    SectionSpec {
        name: "workouts",
        columns: &["started_at", "ended_at", "name", "plan", "day"],
        notes: &[
            "started_at and ended_at are required and unique per workout.",
            "plan and day are optional names from the sections above; leave them blank for a standalone workout.",
        ],
        example: "2026-10-29 18:30,2026-10-29 19:30,Lower focused,Full Body Program,Lower focused",
    },
    SectionSpec {
        name: "workout_exercises",
        columns: &["started_at", "order", "exercise", "exercise_id", "superset"],
        notes: &[
            "started_at links to a workouts row. order is 1-based inside the workout.",
            "superset is the group number saved with the workout.",
        ],
        example: "2026-10-29 18:30,1,Barbell Squat,12,",
    },
    SectionSpec {
        name: "workout_sets",
        columns: &[
            "started_at",
            "exercise_order",
            "set",
            "weight",
            "reps",
            "seconds",
            "distance_m",
            "completed_at",
        ],
        notes: &[
            "exercise_order points at the order column above. set is 1-based; blank counts row order.",
            "At least one of weight, reps, seconds or distance_m is required. completed_at defaults to ended_at.",
        ],
        example: "2026-10-29 18:30,1,1,40,8,,,2026-10-29 18:35",
    },
];

const GLOBAL_NOTES: &[&str] = &[
    "One CSV file. Sections may appear in any order; delete the ones you do not need.",
    "Lines starting with # are ignored. Blank cells are allowed unless a note says otherwise.",
    "Times are local, with an optional offset: 2026-10-29T18:30:00+01:00 or 2026-10-29 18:30.",
    "Importing replaces every routine, workout and body measurement already in the app.",
];

#[derive(Clone, Default, PartialEq)]
pub struct Backup {
    pub settings: Option<Settings>,
    pub measurements: Vec<Measurement>,
    pub plans: Vec<Plan>,
    pub workouts: Vec<Workout>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct Settings {
    pub gender: Option<String>,
    pub dob: Option<i64>,
    pub unit: Option<String>,
    pub rest_seconds: Option<i64>,
    pub default_sets: Option<i64>,
    pub target_reps: Option<i64>,
    pub first_week_day: Option<i64>,
    pub language: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct Measurement {
    pub recorded_at: i64,
    pub weight_kg: Option<f64>,
    pub fat_percent: Option<f64>,
    pub height_cm: Option<f64>,
    pub chest_cm: Option<f64>,
    pub arms_cm: Option<f64>,
    pub waist_cm: Option<f64>,
    pub calves_cm: Option<f64>,
    pub hips_cm: Option<f64>,
    pub thighs_cm: Option<f64>,
    pub shoulders_cm: Option<f64>,
    pub neck_cm: Option<f64>,
    pub forearms_cm: Option<f64>,
}

#[derive(Clone, PartialEq)]
pub struct Plan {
    pub name: String,
    pub level: Option<String>,
    pub goal: Option<String>,
    pub split: String,
    pub block_weeks: i64,
    pub block_enabled: bool,
    pub block_started_at: Option<i64>,
    pub active: bool,
    pub created_at: i64,
    pub days: Vec<PlanDay>,
}

#[derive(Clone, PartialEq)]
pub struct PlanDay {
    pub name: String,
    pub weekday: Option<i64>,
    pub order: i64,
    pub exercises: Vec<PlanExercise>,
}

#[derive(Clone, PartialEq)]
pub struct PlanExercise {
    pub order: i64,
    pub exercise_id: i64,
    pub exercise_name: String,
    pub rest_seconds: i64,
    pub superset: Option<i64>,
    pub sets: Vec<SetTargets>,
}

#[derive(Clone, PartialEq)]
pub struct SetTargets {
    pub set_number: i64,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub seconds: Option<i64>,
    pub distance_m: Option<f64>,
}

#[derive(Clone, PartialEq)]
pub struct Workout {
    pub started_at: i64,
    pub ended_at: i64,
    pub name: String,
    pub plan: Option<String>,
    pub day: Option<String>,
    pub exercises: Vec<WorkoutExercise>,
}

#[derive(Clone, PartialEq)]
pub struct WorkoutExercise {
    pub order: i64,
    pub exercise_id: i64,
    pub exercise_name: String,
    pub superset: Option<i64>,
    pub sets: Vec<LoggedSet>,
}

#[derive(Clone, PartialEq)]
pub struct LoggedSet {
    pub set_number: i64,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub seconds: Option<i64>,
    pub distance_m: Option<f64>,
    pub completed_at: i64,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub plans: i64,
    pub workouts: i64,
    pub measurements: i64,
}

impl Backup {
    #[must_use]
    pub fn counts(&self) -> Counts {
        Counts {
            plans: i64::try_from(self.plans.len()).unwrap_or(i64::MAX),
            workouts: i64::try_from(self.workouts.len()).unwrap_or(i64::MAX),
            measurements: i64::try_from(self.measurements.len()).unwrap_or(i64::MAX),
        }
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.settings.is_none()
            && self.measurements.is_empty()
            && self.plans.is_empty()
            && self.workouts.is_empty()
    }
}

#[must_use]
pub fn template() -> String {
    let mut out = String::new();
    out.push_str(VERSION_LINE);
    out.push('\n');
    for note in GLOBAL_NOTES {
        out.push_str("# ");
        out.push_str(note);
        out.push('\n');
    }
    for spec in &SECTIONS {
        out.push('\n');
        out.push_str("### ");
        out.push_str(spec.name);
        out.push_str(" ###\n");
        for note in spec.notes {
            out.push_str("# ");
            out.push_str(note);
            out.push('\n');
        }
        out.push_str("# example: ");
        out.push_str(spec.example);
        out.push('\n');
        out.push_str(&spec.columns.join(","));
        out.push('\n');
    }
    out
}

pub fn export() -> Result<String> {
    db::with_connection(|conn| to_csv(&load(conn)?))
}

pub fn current_counts() -> Result<Counts> {
    db::with_connection(|conn| {
        let plans = conn.query_row("SELECT COUNT(*) FROM plan", [], |row| row.get(0))?;
        let workouts = conn.query_row(
            "SELECT COUNT(*) FROM session WHERE ended_at IS NOT NULL",
            [],
            |row| row.get(0),
        )?;
        let measurements = conn.query_row("SELECT COUNT(*) FROM measurement", [], |row| row.get(0))?;
        Ok(Counts {
            plans,
            workouts,
            measurements,
        })
    })
}

fn to_csv(backup: &Backup) -> Result<String> {
    let mut writer = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(Vec::new());

    writer.write_record([VERSION_LINE])?;

    header(&mut writer, "settings")?;
    if let Some(settings) = &backup.settings {
        writer.write_record(settings_row(settings))?;
    }

    header(&mut writer, "measurements")?;
    for measurement in &backup.measurements {
        writer.write_record(measurement_row(measurement))?;
    }

    header(&mut writer, "plans")?;
    for plan in &backup.plans {
        writer.write_record(plan_row(plan))?;
    }

    header(&mut writer, "plan_days")?;
    for plan in &backup.plans {
        for day in &plan.days {
            writer.write_record(plan_day_row(&plan.name, day))?;
        }
    }

    header(&mut writer, "plan_exercises")?;
    for plan in &backup.plans {
        for day in &plan.days {
            for exercise in &day.exercises {
                writer.write_record(plan_exercise_row(&plan.name, &day.name, exercise))?;
            }
        }
    }

    header(&mut writer, "plan_sets")?;
    for plan in &backup.plans {
        for day in &plan.days {
            for exercise in &day.exercises {
                for set in &exercise.sets {
                    writer.write_record(plan_set_row(&plan.name, &day.name, exercise, set))?;
                }
            }
        }
    }

    header(&mut writer, "workouts")?;
    for workout in &backup.workouts {
        writer.write_record(workout_row(workout))?;
    }

    header(&mut writer, "workout_exercises")?;
    for workout in &backup.workouts {
        for exercise in &workout.exercises {
            writer.write_record(workout_exercise_row(workout, exercise))?;
        }
    }

    header(&mut writer, "workout_sets")?;
    for workout in &backup.workouts {
        for exercise in &workout.exercises {
            for set in &exercise.sets {
                writer.write_record(workout_set_row(workout, exercise, set))?;
            }
        }
    }

    let bytes = writer
        .into_inner()
        .map_err(csv::IntoInnerError::into_error)?;
    let text = String::from_utf8(bytes).context("CSV is not valid UTF-8")?;
    Ok(space_sections(&text))
}

fn space_sections(text: &str) -> String {
    let mut out = String::new();
    for line in text.lines() {
        if line.starts_with("### ") && !out.is_empty() {
            out.push('\n');
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn header(writer: &mut csv::Writer<Vec<u8>>, name: &str) -> Result<()> {
    let spec = SECTIONS
        .iter()
        .find(|spec| spec.name == name)
        .with_context(|| format!("unknown section {name}"))?;
    writer.write_record([format!("### {} ###", spec.name)])?;
    writer.write_record(spec.columns)?;
    Ok(())
}

fn settings_row(settings: &Settings) -> Vec<String> {
    vec![
        optional(settings.gender.as_deref()),
        date_or_empty(settings.dob),
        optional(settings.unit.as_deref()),
        integer_or_empty(settings.rest_seconds),
        integer_or_empty(settings.default_sets),
        integer_or_empty(settings.target_reps),
        integer_or_empty(settings.first_week_day),
        optional(settings.language.as_deref()),
    ]
}

fn measurement_row(measurement: &Measurement) -> Vec<String> {
    vec![
        time_or_empty(Some(measurement.recorded_at)),
        number_or_empty(measurement.weight_kg),
        number_or_empty(measurement.fat_percent),
        number_or_empty(measurement.height_cm),
        number_or_empty(measurement.chest_cm),
        number_or_empty(measurement.arms_cm),
        number_or_empty(measurement.waist_cm),
        number_or_empty(measurement.calves_cm),
        number_or_empty(measurement.hips_cm),
        number_or_empty(measurement.thighs_cm),
        number_or_empty(measurement.shoulders_cm),
        number_or_empty(measurement.neck_cm),
        number_or_empty(measurement.forearms_cm),
    ]
}

fn plan_row(plan: &Plan) -> Vec<String> {
    vec![
        plan.name.clone(),
        optional(plan.level.as_deref()),
        optional(plan.goal.as_deref()),
        plan.split.clone(),
        plan.block_weeks.to_string(),
        boolean(plan.block_enabled),
        time_or_empty(plan.block_started_at),
        boolean(plan.active),
        time_or_empty(Some(plan.created_at)),
    ]
}

fn plan_day_row(plan: &str, day: &PlanDay) -> Vec<String> {
    vec![
        plan.to_string(),
        day.name.clone(),
        integer_or_empty(day.weekday),
        day.order.to_string(),
    ]
}

fn plan_exercise_row(plan: &str, day: &str, exercise: &PlanExercise) -> Vec<String> {
    vec![
        plan.to_string(),
        day.to_string(),
        exercise.order.to_string(),
        exercise.exercise_name.clone(),
        exercise.exercise_id.to_string(),
        exercise.rest_seconds.to_string(),
        integer_or_empty(exercise.superset),
    ]
}

fn plan_set_row(plan: &str, day: &str, exercise: &PlanExercise, set: &SetTargets) -> Vec<String> {
    vec![
        plan.to_string(),
        day.to_string(),
        exercise.order.to_string(),
        set.set_number.to_string(),
        number_or_empty(set.weight),
        integer_or_empty(set.reps),
        integer_or_empty(set.seconds),
        number_or_empty(set.distance_m),
    ]
}

fn workout_row(workout: &Workout) -> Vec<String> {
    vec![
        time_or_empty(Some(workout.started_at)),
        time_or_empty(Some(workout.ended_at)),
        workout.name.clone(),
        optional(workout.plan.as_deref()),
        optional(workout.day.as_deref()),
    ]
}

fn workout_exercise_row(workout: &Workout, exercise: &WorkoutExercise) -> Vec<String> {
    vec![
        time_or_empty(Some(workout.started_at)),
        exercise.order.to_string(),
        exercise.exercise_name.clone(),
        exercise.exercise_id.to_string(),
        integer_or_empty(exercise.superset),
    ]
}

fn workout_set_row(workout: &Workout, exercise: &WorkoutExercise, set: &LoggedSet) -> Vec<String> {
    vec![
        time_or_empty(Some(workout.started_at)),
        exercise.order.to_string(),
        set.set_number.to_string(),
        number_or_empty(set.weight),
        integer_or_empty(set.reps),
        integer_or_empty(set.seconds),
        number_or_empty(set.distance_m),
        time_or_empty(Some(set.completed_at)),
    ]
}

fn optional(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

fn boolean(value: bool) -> String {
    if value { "1" } else { "0" }.to_string()
}

fn number_or_empty(value: Option<f64>) -> String {
    value.map_or_else(String::new, |number| number.to_string())
}

fn integer_or_empty(value: Option<i64>) -> String {
    value.map_or_else(String::new, |number| number.to_string())
}

fn time_or_empty(value: Option<i64>) -> String {
    value.map_or_else(String::new, format_time)
}

fn date_or_empty(value: Option<i64>) -> String {
    value.map_or_else(String::new, format_date)
}

fn format_time(seconds: i64) -> String {
    Local.timestamp_opt(seconds, 0).single().map_or_else(String::new, |moment| {
        moment.to_rfc3339_opts(SecondsFormat::Secs, false)
    })
}

fn format_date(seconds: i64) -> String {
    Local.timestamp_opt(seconds, 0).single().map_or_else(String::new, |moment| {
        moment.format("%Y-%m-%d").to_string()
    })
}

fn load(conn: &Connection) -> Result<Backup> {
    Ok(Backup {
        settings: load_settings(conn)?,
        measurements: load_measurements(conn)?,
        plans: load_plans(conn)?,
        workouts: load_workouts(conn)?,
    })
}

fn load_settings(conn: &Connection) -> Result<Option<Settings>> {
    Ok(conn
        .query_row(
            "SELECT gender, dob, unit, rest_seconds, default_sets, default_target_reps,
                    first_week_day, language
             FROM user_setting WHERE id = 1",
            [],
            |row| {
                Ok(Settings {
                    gender: row.get(0)?,
                    dob: row.get(1)?,
                    unit: Some(row.get(2)?),
                    rest_seconds: Some(row.get(3)?),
                    default_sets: Some(row.get(4)?),
                    target_reps: Some(row.get(5)?),
                    first_week_day: Some(row.get(6)?),
                    language: Some(row.get(7)?),
                })
            },
        )
        .optional()?)
}

fn load_measurements(conn: &Connection) -> Result<Vec<Measurement>> {
    let mut stmt = conn.prepare(
        "SELECT recorded_at, weight_kg, fat_percent, height_cm, chest_cm, arms_cm, waist_cm,
                calves_cm, hips_cm, thighs_cm, shoulders_cm, neck_cm, forearms_cm
         FROM measurement ORDER BY recorded_at, id",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Measurement {
            recorded_at: row.get(0)?,
            weight_kg: row.get(1)?,
            fat_percent: row.get(2)?,
            height_cm: row.get(3)?,
            chest_cm: row.get(4)?,
            arms_cm: row.get(5)?,
            waist_cm: row.get(6)?,
            calves_cm: row.get(7)?,
            hips_cm: row.get(8)?,
            thighs_cm: row.get(9)?,
            shoulders_cm: row.get(10)?,
            neck_cm: row.get(11)?,
            forearms_cm: row.get(12)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_plans(conn: &Connection) -> Result<Vec<Plan>> {
    let base = {
        let mut stmt = conn.prepare(
            "SELECT id, name, level, goal, split, block_weeks, block_enabled,
                    block_started_at, is_active, created_at
             FROM plan ORDER BY created_at, id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                Plan {
                    name: row.get(1)?,
                    level: row.get(2)?,
                    goal: row.get(3)?,
                    split: row.get(4)?,
                    block_weeks: row.get(5)?,
                    block_enabled: row.get(6)?,
                    block_started_at: row.get(7)?,
                    active: row.get(8)?,
                    created_at: row.get(9)?,
                    days: Vec::new(),
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut plans = Vec::new();
    for (id, mut plan) in base {
        plan.days = load_plan_days(conn, id)?;
        plans.push(plan);
    }
    Ok(plans)
}

fn load_plan_days(conn: &Connection, plan_id: i64) -> Result<Vec<PlanDay>> {
    let base = {
        let mut stmt = conn.prepare(
            "SELECT id, name, weekday, order_index FROM plan_day
             WHERE plan_id = ?1 ORDER BY order_index",
        )?;
        let rows = stmt.query_map([plan_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PlanDay {
                    name: row.get(1)?,
                    weekday: row.get(2)?,
                    order: row.get::<_, i64>(3)?.saturating_add(1),
                    exercises: Vec::new(),
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut days = Vec::new();
    for (id, mut day) in base {
        day.exercises = load_plan_exercises(conn, id)?;
        days.push(day);
    }
    Ok(days)
}

fn load_plan_exercises(conn: &Connection, day_id: i64) -> Result<Vec<PlanExercise>> {
    let base = {
        let mut stmt = conn.prepare(
            "SELECT pe.id, pe.exercise_id, e.name, pe.order_index, pe.rest_seconds, pe.superset_group
             FROM plan_exercise pe JOIN exercise e ON e.id = pe.exercise_id
             WHERE pe.plan_day_id = ?1 ORDER BY pe.order_index",
        )?;
        let rows = stmt.query_map([day_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                PlanExercise {
                    order: row.get::<_, i64>(3)?.saturating_add(1),
                    exercise_id: row.get(1)?,
                    exercise_name: row.get(2)?,
                    rest_seconds: row.get(4)?,
                    superset: row.get(5)?,
                    sets: Vec::new(),
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut exercises = Vec::new();
    for (id, mut exercise) in base {
        exercise.sets = load_plan_sets(conn, id)?;
        exercises.push(exercise);
    }
    Ok(exercises)
}

fn load_plan_sets(conn: &Connection, exercise_id: i64) -> Result<Vec<SetTargets>> {
    let mut stmt = conn.prepare(
        "SELECT set_number, target_weight, target_reps, target_seconds, target_distance_m
         FROM plan_set WHERE plan_exercise_id = ?1 ORDER BY set_number",
    )?;
    let rows = stmt.query_map([exercise_id], |row| {
        Ok(SetTargets {
            set_number: row.get(0)?,
            weight: row.get(1)?,
            reps: row.get(2)?,
            seconds: row.get(3)?,
            distance_m: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn load_workouts(conn: &Connection) -> Result<Vec<Workout>> {
    let base = {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.started_at, s.ended_at, COALESCE(s.name, ''), p.name, d.name
             FROM session s
             LEFT JOIN plan p ON p.id = s.plan_id
             LEFT JOIN plan_day d ON d.id = s.plan_day_id
             WHERE s.ended_at IS NOT NULL
             ORDER BY s.started_at, s.id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                Workout {
                    started_at: row.get(1)?,
                    ended_at: row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                    name: row.get(3)?,
                    plan: row.get(4)?,
                    day: row.get(5)?,
                    exercises: Vec::new(),
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut workouts = Vec::new();
    for (id, mut workout) in base {
        workout.exercises = load_workout_exercises(conn, id)?;
        workouts.push(workout);
    }
    Ok(workouts)
}

fn load_workout_exercises(conn: &Connection, session_id: i64) -> Result<Vec<WorkoutExercise>> {
    let base = {
        let mut stmt = conn.prepare(
            "SELECT se.id, se.exercise_id, e.name, se.order_index, se.superset_group
             FROM session_exercise se JOIN exercise e ON e.id = se.exercise_id
             WHERE se.session_id = ?1 ORDER BY se.order_index",
        )?;
        let rows = stmt.query_map([session_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                WorkoutExercise {
                    order: row.get::<_, i64>(3)?.saturating_add(1),
                    exercise_id: row.get(1)?,
                    exercise_name: row.get(2)?,
                    superset: row.get(4)?,
                    sets: Vec::new(),
                },
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    let mut exercises = Vec::new();
    for (id, mut exercise) in base {
        exercise.sets = load_workout_sets(conn, id)?;
        exercises.push(exercise);
    }
    Ok(exercises)
}

fn load_workout_sets(conn: &Connection, exercise_id: i64) -> Result<Vec<LoggedSet>> {
    let mut stmt = conn.prepare(
        "SELECT set_number, weight, reps, seconds, distance_m, completed_at
         FROM session_set WHERE session_exercise_id = ?1 ORDER BY set_number",
    )?;
    let rows = stmt.query_map([exercise_id], |row| {
        Ok(LoggedSet {
            set_number: row.get(0)?,
            weight: row.get(1)?,
            reps: row.get(2)?,
            seconds: row.get(3)?,
            distance_m: row.get(4)?,
            completed_at: row.get(5)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

struct Table {
    spec: &'static SectionSpec,
    start_line: usize,
    header: Option<(usize, Vec<String>)>,
    rows: Vec<(usize, Vec<String>)>,
}

impl Table {
    fn cell<'a>(&self, row: &'a [String], column: &str) -> Option<&'a str> {
        let index = self.spec.columns.iter().position(|name| *name == column)?;
        row.get(index)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

struct ExerciseNames {
    base: String,
    ar: Option<String>,
    fr: Option<String>,
}

struct Catalog {
    names: HashMap<i64, ExerciseNames>,
    by_key: HashMap<String, Vec<i64>>,
}

pub fn parse(text: &str) -> Result<Backup, String> {
    let (catalog, default_rest) = db::with_connection(|conn| load_catalog(conn))
        .map_err(|error| format!("Could not read the exercise list: {error:#}"))?;
    let tables = collect(text).map_err(|errors| errors.join("\n"))?;
    let mut problems = Vec::new();

    let settings = parse_settings(tables.get("settings"), &mut problems);
    let measurements = parse_measurements(tables.get("measurements"), &mut problems);
    let mut plans = parse_plans(tables.get("plans"), &mut problems);
    parse_plan_days(tables.get("plan_days"), &mut plans, &mut problems);
    parse_plan_exercises(
        tables.get("plan_exercises"),
        &catalog,
        default_rest,
        &mut plans,
        &mut problems,
    );
    parse_plan_sets(tables.get("plan_sets"), &mut plans, &mut problems);
    let mut workouts = parse_workouts(tables.get("workouts"), &plans, &mut problems);
    parse_workout_exercises(
        tables.get("workout_exercises"),
        &catalog,
        &mut workouts,
        &mut problems,
    );
    parse_workout_sets(tables.get("workout_sets"), &mut workouts, &mut problems);

    finish_orders(&mut plans, &mut workouts);

    if problems.is_empty() {
        Ok(Backup {
            settings,
            measurements,
            plans,
            workouts,
        })
    } else {
        if problems.len() >= MAX_PROBLEMS {
            problems.push("More errors follow; fix these first.".to_string());
        }
        Err(problems.join("\n"))
    }
}

#[allow(clippy::too_many_lines)]
fn collect(text: &str) -> Result<HashMap<&'static str, Table>, Vec<String>> {
    let mut errors = Vec::new();
    let mut tables: HashMap<&'static str, Table> = HashMap::new();
    let mut current: Option<&'static str> = None;
    let mut version_seen = false;
    let mut first = true;

    for (index, raw) in text.lines().enumerate() {
        let line = index.saturating_add(1);
        let raw = raw.strip_prefix('\u{feff}').unwrap_or(raw);
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if first {
            first = false;
            if trimmed != VERSION_LINE {
                return Err(vec![format!(
                    "line {line}: this is not a LibreSteel CSV. The first line must be {VERSION_LINE}."
                )]);
            }
            version_seen = true;
            continue;
        }
        if trimmed.starts_with("### ") && trimmed.ends_with(" ###") {
            let name = trimmed
                .strip_prefix("### ")
                .and_then(|rest| rest.strip_suffix(" ###"))
                .map(str::trim)
                .unwrap_or_default();
            let Some(spec) = SECTIONS.iter().find(|spec| spec.name == name) else {
                push_problem(&mut errors, line, format!("unknown section '### {name} ###'"));
                current = None;
                continue;
            };
            if tables.contains_key(spec.name) {
                push_problem(
                    &mut errors,
                    line,
                    format!("section '{}' appears twice", spec.name),
                );
                current = None;
                continue;
            }
            tables.insert(
                spec.name,
                Table {
                    spec,
                    start_line: line,
                    header: None,
                    rows: Vec::new(),
                },
            );
            current = Some(spec.name);
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        let Some(section) = current else {
            push_problem(&mut errors, line, "data appears before any section");
            continue;
        };
        let Some(table) = tables.get_mut(section) else {
            continue;
        };
        match fields(raw, line) {
            Ok(values) => {
                if table.header.is_none() {
                    table.header = Some((line, values));
                } else if let Some((_, header)) = &table.header {
                    if values.len() == header.len() {
                        table.rows.push((line, values));
                    } else {
                        push_problem(
                            &mut errors,
                            line,
                            format!("expected {} columns, found {}", header.len(), values.len()),
                        );
                    }
                }
            }
            Err(error) => errors.push(error),
        }
    }

    if !version_seen {
        return Err(vec!["The file is empty.".to_string()]);
    }

    for spec in &SECTIONS {
        let Some(table) = tables.get(spec.name) else {
            continue;
        };
        let Some((line, header)) = &table.header else {
            push_problem(
                &mut errors,
                table.start_line,
                format!("section '{}' has no header row", spec.name),
            );
            continue;
        };
        let expected = spec.columns;
        if header.len() != expected.len()
            || header
                .iter()
                .zip(expected.iter())
                .any(|(actual, wanted)| actual.as_str() != *wanted)
        {
            push_problem(
                &mut errors,
                *line,
                format!("header for '{}' must be: {}", spec.name, expected.join(",")),
            );
        }
    }

    if errors.is_empty() {
        Ok(tables)
    } else {
        Err(errors)
    }
}

fn fields(raw: &str, line: usize) -> Result<Vec<String>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .from_reader(raw.as_bytes());
    let Some(record) = reader.records().next() else {
        return Err(format!("line {line}: empty row"));
    };
    let record = record.map_err(|error| format!("line {line}: {error}"))?;
    Ok(record.iter().map(|value| value.trim().to_string()).collect())
}

fn push_problem(problems: &mut Vec<String>, line: usize, message: impl std::fmt::Display) {
    if problems.len() < MAX_PROBLEMS {
        problems.push(format!("line {line}: {message}"));
    }
}

fn parse_settings(table: Option<&Table>, problems: &mut Vec<String>) -> Option<Settings> {
    let table = table?;
    if table.rows.len() > 1 {
        push_problem(
            problems,
            table.rows.get(1).map_or(0, |(line, _)| *line),
            "settings must have at most one row",
        );
        return None;
    }
    let (line, row) = table.rows.first()?;
    match parse_settings_row(table, row) {
        Ok(settings) => Some(settings),
        Err(error) => {
            push_problem(problems, *line, error);
            None
        }
    }
}

fn parse_settings_row(table: &Table, row: &[String]) -> Result<Settings> {
    let gender = match table.cell(row, "gender") {
        None => None,
        Some(value) if value.eq_ignore_ascii_case("male") => Some("male".to_string()),
        Some(value) if value.eq_ignore_ascii_case("female") => Some("female".to_string()),
        Some(value) => bail!("gender '{value}' must be male or female"),
    };
    let dob = date(table, row, "dob")?;
    let unit = choice(table, row, "unit", &["kg", "lb"])?;
    let rest_seconds = integer(table, row, "rest_seconds")?;
    if rest_seconds.is_some_and(|value| value < 0) {
        bail!("rest_seconds must be zero or more");
    }
    let default_sets = integer(table, row, "default_sets")?;
    if default_sets.is_some_and(|value| value < 1) {
        bail!("default_sets must be at least 1");
    }
    let target_reps = integer(table, row, "target_reps")?;
    if target_reps.is_some_and(|value| value < 1) {
        bail!("target_reps must be at least 1");
    }
    let first_week_day = integer(table, row, "first_week_day")?;
    if first_week_day.is_some_and(|value| value != 0 && value != 6) {
        bail!("first_week_day must be 0 (Mon) or 6 (Sun)");
    }
    let language = choice(table, row, "language", &["en", "ar", "fr"])?;
    Ok(Settings {
        gender,
        dob,
        unit,
        rest_seconds,
        default_sets,
        target_reps,
        first_week_day,
        language,
    })
}

fn parse_measurements(table: Option<&Table>, problems: &mut Vec<String>) -> Vec<Measurement> {
    let Some(table) = table else {
        return Vec::new();
    };
    let mut measurements = Vec::new();
    for (line, row) in &table.rows {
        match parse_measurement_row(table, row) {
            Ok(measurement) => measurements.push(measurement),
            Err(error) => push_problem(problems, *line, error),
        }
    }
    measurements
}

fn parse_measurement_row(table: &Table, row: &[String]) -> Result<Measurement> {
    let recorded_at = required_time(table, row, "recorded_at")?;
    let weight_kg = number(table, row, "weight_kg")?;
    if weight_kg.is_some_and(|value| value <= 0.0) {
        bail!("weight_kg must be above 0");
    }
    let fat_percent = number(table, row, "fat_percent")?;
    if fat_percent.is_some_and(|value| !(0.0..=100.0).contains(&value)) {
        bail!("fat_percent must be between 0 and 100");
    }
    let height_cm = non_negative(number(table, row, "height_cm")?, "height_cm")?;
    let chest_cm = non_negative(number(table, row, "chest_cm")?, "chest_cm")?;
    let arms_cm = non_negative(number(table, row, "arms_cm")?, "arms_cm")?;
    let waist_cm = non_negative(number(table, row, "waist_cm")?, "waist_cm")?;
    let calves_cm = non_negative(number(table, row, "calves_cm")?, "calves_cm")?;
    let hips_cm = non_negative(number(table, row, "hips_cm")?, "hips_cm")?;
    let thighs_cm = non_negative(number(table, row, "thighs_cm")?, "thighs_cm")?;
    let shoulders_cm = non_negative(number(table, row, "shoulders_cm")?, "shoulders_cm")?;
    let neck_cm = non_negative(number(table, row, "neck_cm")?, "neck_cm")?;
    let forearms_cm = non_negative(number(table, row, "forearms_cm")?, "forearms_cm")?;
    Ok(Measurement {
        recorded_at,
        weight_kg,
        fat_percent,
        height_cm,
        chest_cm,
        arms_cm,
        waist_cm,
        calves_cm,
        hips_cm,
        thighs_cm,
        shoulders_cm,
        neck_cm,
        forearms_cm,
    })
}

fn parse_plans(table: Option<&Table>, problems: &mut Vec<String>) -> Vec<Plan> {
    let Some(table) = table else {
        return Vec::new();
    };
    let mut plans = Vec::new();
    let mut active_line = None;
    for (line, row) in &table.rows {
        match parse_plan_row(table, row) {
            Ok(plan) => {
                if plans.iter().any(|existing: &Plan| existing.name == plan.name) {
                    push_problem(
                        problems,
                        *line,
                        format!("plan '{}' appears twice", plan.name),
                    );
                } else {
                    if plan.active {
                        if active_line.is_some() {
                            push_problem(problems, *line, "only one plan can be active");
                        } else {
                            active_line = Some(*line);
                        }
                    }
                    plans.push(plan);
                }
            }
            Err(error) => push_problem(problems, *line, error),
        }
    }
    plans
}

fn parse_plan_row(table: &Table, row: &[String]) -> Result<Plan> {
    let name = required(table, row, "name")?.to_string();
    let level = choice(
        table,
        row,
        "level",
        &["beginner", "intermediate", "advanced"],
    )?;
    let goal = choice(
        table,
        row,
        "goal",
        &["cutting", "maintaining", "bulking", "building_strength"],
    )?;
    let split = choice(table, row, "split", &["numbered", "weekday"])?
        .unwrap_or_else(|| "numbered".to_string());
    let block_weeks = integer(table, row, "block_weeks")?.unwrap_or(4);
    if !(1..=52).contains(&block_weeks) {
        bail!("block_weeks must be between 1 and 52");
    }
    let block_enabled = flag(table, row, "block_enabled")?.unwrap_or(true);
    let block_started_at = time(table, row, "block_started")?;
    let active = flag(table, row, "active")?.unwrap_or(false);
    let created_at = time(table, row, "created_at")?.unwrap_or_else(db::now);
    Ok(Plan {
        name,
        level,
        goal,
        split,
        block_weeks,
        block_enabled,
        block_started_at,
        active,
        created_at,
        days: Vec::new(),
    })
}

fn parse_plan_days(table: Option<&Table>, plans: &mut [Plan], problems: &mut Vec<String>) {
    let Some(table) = table else {
        return;
    };
    for (line, row) in &table.rows {
        if let Err(error) = parse_plan_day_row(table, row, plans) {
            push_problem(problems, *line, error);
        }
    }
}

fn parse_plan_day_row(table: &Table, row: &[String], plans: &mut [Plan]) -> Result<()> {
    let plan_name = required(table, row, "plan")?;
    let day_name = required(table, row, "day")?;
    let Some(plan) = plans.iter_mut().find(|plan| plan.name == plan_name) else {
        bail!("plan '{plan_name}' is not in the plans section");
    };
    if plan.days.iter().any(|day| day.name == day_name) {
        bail!("day '{day_name}' appears twice under plan '{plan_name}'");
    }
    let weekday = integer(table, row, "weekday")?;
    if weekday.is_some_and(|value| !(0..=6).contains(&value)) {
        bail!("weekday must be between 0 (Mon) and 6 (Sun)");
    }
    let next = plan
        .days
        .iter()
        .map(|day| day.order)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let order = integer(table, row, "order")?.unwrap_or(next);
    if order < 1 {
        bail!("order must be at least 1");
    }
    if plan.days.iter().any(|day| day.order == order) {
        bail!("order {order} is used twice in plan '{plan_name}'");
    }
    plan.days.push(PlanDay {
        name: day_name.to_string(),
        weekday,
        order,
        exercises: Vec::new(),
    });
    Ok(())
}

fn parse_plan_exercises(
    table: Option<&Table>,
    catalog: &Catalog,
    default_rest: i64,
    plans: &mut [Plan],
    problems: &mut Vec<String>,
) {
    let Some(table) = table else {
        return;
    };
    for (line, row) in &table.rows {
        if let Err(error) = parse_plan_exercise_row(table, row, catalog, default_rest, plans) {
            push_problem(problems, *line, error);
        }
    }
}

fn parse_plan_exercise_row(
    table: &Table,
    row: &[String],
    catalog: &Catalog,
    default_rest: i64,
    plans: &mut [Plan],
) -> Result<()> {
    let plan_name = required(table, row, "plan")?;
    let day_name = required(table, row, "day")?;
    let Some(plan) = plans.iter_mut().find(|plan| plan.name == plan_name) else {
        bail!("plan '{plan_name}' is not in the plans section");
    };
    let Some(day) = plan.days.iter_mut().find(|day| day.name == day_name) else {
        bail!("day '{day_name}' is not in plan '{plan_name}'");
    };
    let next = day
        .exercises
        .iter()
        .map(|exercise| exercise.order)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let order = integer(table, row, "order")?.unwrap_or(next);
    if order < 1 {
        bail!("order must be at least 1");
    }
    if day.exercises.iter().any(|exercise| exercise.order == order) {
        bail!("order {order} is used twice in '{plan_name} / {day_name}'");
    }
    let rest_seconds = integer(table, row, "rest_seconds")?.unwrap_or(default_rest);
    if rest_seconds < 0 {
        bail!("rest_seconds must be zero or more");
    }
    let superset = integer(table, row, "superset")?;
    let exercise_id = integer(table, row, "exercise_id")?;
    let exercise = table.cell(row, "exercise").map(str::to_string);
    let (exercise_id, exercise_name) = resolve(catalog, exercise_id, exercise.as_deref())?;
    day.exercises.push(PlanExercise {
        order,
        exercise_id,
        exercise_name,
        rest_seconds,
        superset,
        sets: Vec::new(),
    });
    Ok(())
}

fn parse_plan_sets(table: Option<&Table>, plans: &mut [Plan], problems: &mut Vec<String>) {
    let Some(table) = table else {
        return;
    };
    for (line, row) in &table.rows {
        if let Err(error) = parse_plan_set_row(table, row, plans) {
            push_problem(problems, *line, error);
        }
    }
}

fn parse_plan_set_row(table: &Table, row: &[String], plans: &mut [Plan]) -> Result<()> {
    let plan_name = required(table, row, "plan")?;
    let day_name = required(table, row, "day")?;
    let exercise_order = required_integer(table, row, "exercise_order")?;
    let Some(plan) = plans.iter_mut().find(|plan| plan.name == plan_name) else {
        bail!("plan '{plan_name}' is not in the plans section");
    };
    let Some(day) = plan.days.iter_mut().find(|day| day.name == day_name) else {
        bail!("day '{day_name}' is not in plan '{plan_name}'");
    };
    let Some(exercise) = day
        .exercises
        .iter_mut()
        .find(|exercise| exercise.order == exercise_order)
    else {
        bail!(
            "exercise_order {exercise_order} is not in '{plan_name} / {day_name}'"
        );
    };
    let next = exercise
        .sets
        .iter()
        .map(|set| set.set_number)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let set_number = integer(table, row, "set")?.unwrap_or(next);
    if set_number < 1 {
        bail!("set must be at least 1");
    }
    if exercise.sets.iter().any(|set| set.set_number == set_number) {
        bail!(
            "set {set_number} is used twice for exercise {exercise_order} in '{plan_name} / {day_name}'"
        );
    }
    let weight = non_negative(number(table, row, "weight")?, "weight")?;
    let reps = integer(table, row, "reps")?;
    if reps.is_some_and(|value| value < 0) {
        bail!("reps must be zero or more");
    }
    let seconds = integer(table, row, "seconds")?;
    if seconds.is_some_and(|value| value < 0) {
        bail!("seconds must be zero or more");
    }
    let distance_m = non_negative(number(table, row, "distance_m")?, "distance_m")?;
    exercise.sets.push(SetTargets {
        set_number,
        weight,
        reps,
        seconds,
        distance_m,
    });
    Ok(())
}

fn parse_workouts(
    table: Option<&Table>,
    plans: &[Plan],
    problems: &mut Vec<String>,
) -> Vec<Workout> {
    let Some(table) = table else {
        return Vec::new();
    };
    let mut workouts = Vec::new();
    for (line, row) in &table.rows {
        match parse_workout_row(table, row, plans) {
            Ok(workout) => {
                if workouts
                    .iter()
                    .any(|existing: &Workout| existing.started_at == workout.started_at)
                {
                    push_problem(
                        problems,
                        *line,
                        format!(
                            "a workout starting at {} appears twice",
                            format_time(workout.started_at)
                        ),
                    );
                } else {
                    workouts.push(workout);
                }
            }
            Err(error) => push_problem(problems, *line, error),
        }
    }
    workouts
}

fn parse_workout_row(table: &Table, row: &[String], plans: &[Plan]) -> Result<Workout> {
    let started_at = required_time(table, row, "started_at")?;
    let ended_at = required_time(table, row, "ended_at")?;
    if ended_at < started_at {
        bail!("ended_at is before started_at");
    }
    let plan = table.cell(row, "plan").map(str::to_string);
    let day = table.cell(row, "day").map(str::to_string);
    if let Some(plan_name) = plan.as_deref() {
        if !plans.iter().any(|plan| plan.name == plan_name) {
            bail!("plan '{plan_name}' is not in the plans section");
        }
    }
    if let Some(day_name) = day.as_deref() {
        let Some(plan_name) = plan.as_deref() else {
            bail!("day needs a plan");
        };
        let Some(plan) = plans.iter().find(|plan| plan.name == plan_name) else {
            bail!("plan '{plan_name}' is not in the plans section");
        };
        if !plan.days.iter().any(|day| day.name == day_name) {
            bail!("day '{day_name}' is not in plan '{plan_name}'");
        }
    }
    let name = table
        .cell(row, "name")
        .map(str::to_string)
        .or_else(|| day.clone())
        .unwrap_or_else(|| "Workout".to_string());
    Ok(Workout {
        started_at,
        ended_at,
        name,
        plan,
        day,
        exercises: Vec::new(),
    })
}

fn parse_workout_exercises(
    table: Option<&Table>,
    catalog: &Catalog,
    workouts: &mut [Workout],
    problems: &mut Vec<String>,
) {
    let Some(table) = table else {
        return;
    };
    for (line, row) in &table.rows {
        if let Err(error) = parse_workout_exercise_row(table, row, catalog, workouts) {
            push_problem(problems, *line, error);
        }
    }
}

fn parse_workout_exercise_row(
    table: &Table,
    row: &[String],
    catalog: &Catalog,
    workouts: &mut [Workout],
) -> Result<()> {
    let started_at = required_time(table, row, "started_at")?;
    let Some(workout) = workouts
        .iter_mut()
        .find(|workout| workout.started_at == started_at)
    else {
        bail!(
            "no workout starts at {}",
            format_time(started_at)
        );
    };
    let next = workout
        .exercises
        .iter()
        .map(|exercise| exercise.order)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let order = integer(table, row, "order")?.unwrap_or(next);
    if order < 1 {
        bail!("order must be at least 1");
    }
    if workout.exercises.iter().any(|exercise| exercise.order == order) {
        bail!("order {order} is used twice in this workout");
    }
    let superset = integer(table, row, "superset")?;
    let exercise_id = integer(table, row, "exercise_id")?;
    let exercise = table.cell(row, "exercise").map(str::to_string);
    let (exercise_id, exercise_name) = resolve(catalog, exercise_id, exercise.as_deref())?;
    workout.exercises.push(WorkoutExercise {
        order,
        exercise_id,
        exercise_name,
        superset,
        sets: Vec::new(),
    });
    Ok(())
}

fn parse_workout_sets(table: Option<&Table>, workouts: &mut [Workout], problems: &mut Vec<String>) {
    let Some(table) = table else {
        return;
    };
    for (line, row) in &table.rows {
        if let Err(error) = parse_workout_set_row(table, row, workouts) {
            push_problem(problems, *line, error);
        }
    }
}

fn parse_workout_set_row(table: &Table, row: &[String], workouts: &mut [Workout]) -> Result<()> {
    let started_at = required_time(table, row, "started_at")?;
    let exercise_order = required_integer(table, row, "exercise_order")?;
    let Some(workout) = workouts
        .iter_mut()
        .find(|workout| workout.started_at == started_at)
    else {
        bail!(
            "no workout starts at {}",
            format_time(started_at)
        );
    };
    let ended_at = workout.ended_at;
    let Some(exercise) = workout
        .exercises
        .iter_mut()
        .find(|exercise| exercise.order == exercise_order)
    else {
        bail!("exercise_order {exercise_order} is not in this workout");
    };
    let next = exercise
        .sets
        .iter()
        .map(|set| set.set_number)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let set_number = integer(table, row, "set")?.unwrap_or(next);
    if set_number < 1 {
        bail!("set must be at least 1");
    }
    if exercise.sets.iter().any(|set| set.set_number == set_number) {
        bail!("set {set_number} is used twice for exercise {exercise_order}");
    }
    let weight = non_negative(number(table, row, "weight")?, "weight")?;
    let reps = integer(table, row, "reps")?;
    if reps.is_some_and(|value| value < 0) {
        bail!("reps must be zero or more");
    }
    let seconds = integer(table, row, "seconds")?;
    if seconds.is_some_and(|value| value < 0) {
        bail!("seconds must be zero or more");
    }
    let distance_m = non_negative(number(table, row, "distance_m")?, "distance_m")?;
    let completed_at = time(table, row, "completed_at")?.unwrap_or(ended_at);
    exercise.sets.push(LoggedSet {
        set_number,
        weight,
        reps,
        seconds,
        distance_m,
        completed_at,
    });
    Ok(())
}

fn finish_orders(plans: &mut [Plan], workouts: &mut [Workout]) {
    for plan in plans {
        plan.days.sort_by_key(|day| day.order);
        for day in &mut plan.days {
            day.exercises.sort_by_key(|exercise| exercise.order);
            for exercise in &mut day.exercises {
                exercise.sets.sort_by_key(|set| set.set_number);
            }
        }
    }
    for workout in workouts {
        workout.exercises.sort_by_key(|exercise| exercise.order);
        for exercise in &mut workout.exercises {
            exercise.sets.sort_by_key(|set| set.set_number);
        }
    }
}

fn required<'a>(table: &Table, row: &'a [String], column: &str) -> Result<&'a str> {
    table
        .cell(row, column)
        .with_context(|| format!("{column} is required"))
}

fn required_integer(table: &Table, row: &[String], column: &str) -> Result<i64> {
    let raw = required(table, row, column)?;
    raw.parse::<i64>()
        .with_context(|| format!("{column} '{raw}' must be a whole number"))
}

fn required_time(table: &Table, row: &[String], column: &str) -> Result<i64> {
    let raw = required(table, row, column)?;
    parse_time(raw).with_context(|| format!("{column} '{raw}' is not a date or time"))
}

fn time(table: &Table, row: &[String], column: &str) -> Result<Option<i64>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    parse_time(raw)
        .map(Some)
        .with_context(|| format!("{column} '{raw}' is not a date or time"))
}

fn date(table: &Table, row: &[String], column: &str) -> Result<Option<i64>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    parse_date(raw)
        .map(Some)
        .with_context(|| format!("{column} '{raw}' is not a date"))
}

fn number(table: &Table, row: &[String], column: &str) -> Result<Option<f64>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    parse_number(raw)
        .map(Some)
        .with_context(|| format!("{column} '{raw}' is not a number"))
}

fn integer(table: &Table, row: &[String], column: &str) -> Result<Option<i64>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    raw.parse::<i64>()
        .map(Some)
        .with_context(|| format!("{column} '{raw}' must be a whole number"))
}

fn flag(table: &Table, row: &[String], column: &str) -> Result<Option<bool>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    match raw.to_lowercase().as_str() {
        "1" | "true" | "yes" => Ok(Some(true)),
        "0" | "false" | "no" => Ok(Some(false)),
        _ => bail!("{column} '{raw}' must be 1 or 0"),
    }
}

fn choice(table: &Table, row: &[String], column: &str, allowed: &[&str]) -> Result<Option<String>> {
    let Some(raw) = table.cell(row, column) else {
        return Ok(None);
    };
    let value = raw.to_lowercase();
    if allowed.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        bail!("{column} '{raw}' must be one of {}", allowed.join(", "));
    }
}

fn non_negative(value: Option<f64>, column: &str) -> Result<Option<f64>> {
    if value.is_some_and(|number| number < 0.0) {
        bail!("{column} must be zero or more");
    }
    Ok(value)
}

fn parse_number(raw: &str) -> Option<f64> {
    raw.replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn parse_time(raw: &str) -> Option<i64> {
    if let Ok(value) = DateTime::parse_from_rfc3339(raw) {
        return Some(value.timestamp());
    }
    for format in [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M",
    ] {
        if let Ok(value) = NaiveDateTime::parse_from_str(raw, format) {
            return local(&value);
        }
    }
    if let Ok(value) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return value
            .and_hms_opt(12, 0, 0)
            .and_then(|value| local(&value));
    }
    None
}

fn parse_date(raw: &str) -> Option<i64> {
    if let Ok(value) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return value
            .and_hms_opt(0, 0, 0)
            .and_then(|value| local(&value));
    }
    parse_time(raw)
}

fn local(value: &NaiveDateTime) -> Option<i64> {
    Local.from_local_datetime(value).earliest().map(|value| value.timestamp())
}

fn load_catalog(conn: &Connection) -> Result<(Catalog, i64)> {
    let mut stmt = conn.prepare("SELECT id, name, name_ar, name_fr FROM exercise")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut names = HashMap::new();
    let mut by_key: HashMap<String, Vec<i64>> = HashMap::new();
    for row in rows {
        let (id, base, ar, fr) = row?;
        let entry = ExerciseNames { base, ar, fr };
        for candidate in [
            Some(entry.base.clone()),
            entry.ar.clone(),
            entry.fr.clone(),
        ]
        .into_iter()
        .flatten()
        {
            by_key.entry(normalize(&candidate)).or_default().push(id);
        }
        names.insert(id, entry);
    }
    for ids in by_key.values_mut() {
        ids.sort_unstable();
        ids.dedup();
    }
    let default_rest = conn
        .query_row(
            "SELECT rest_seconds FROM user_setting WHERE id = 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(60);
    Ok((Catalog { names, by_key }, default_rest))
}

fn resolve(catalog: &Catalog, id: Option<i64>, name: Option<&str>) -> Result<(i64, String)> {
    if let Some(id) = id {
        let entry = catalog
            .names
            .get(&id)
            .with_context(|| format!("exercise_id {id} does not exist"))?;
        if let Some(name) = name {
            let wanted = normalize(name);
            let matches = [
                Some(entry.base.as_str()),
                entry.ar.as_deref(),
                entry.fr.as_deref(),
            ]
            .into_iter()
            .flatten()
            .any(|candidate| normalize(candidate) == wanted);
            if !matches {
                bail!("exercise_id {id} is '{}', not '{name}'", entry.base);
            }
        }
        return Ok((id, entry.base.clone()));
    }
    let name = name.context("exercise or exercise_id is required")?;
    let Some(ids) = catalog.by_key.get(&normalize(name)) else {
        bail!("exercise '{name}' was not found");
    };
    if let [id] = ids.as_slice() {
        let entry = catalog
            .names
            .get(id)
            .with_context(|| format!("exercise id {id} is missing"))?;
        Ok((*id, entry.base.clone()))
    } else {
        let list = ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        bail!("exercise '{name}' is ambiguous; use one of ids {list}")
    }
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

#[allow(clippy::too_many_lines)]
pub fn replace(backup: &Backup) -> Result<()> {
    db::with_connection(|conn| {
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM session", [])?;
        tx.execute("DELETE FROM plan", [])?;
        tx.execute("DELETE FROM measurement", [])?;

        if let Some(settings) = &backup.settings {
            tx.execute(
                "UPDATE user_setting SET
                     gender = COALESCE(?1, gender),
                     dob = COALESCE(?2, dob),
                     unit = COALESCE(?3, unit),
                     rest_seconds = COALESCE(?4, rest_seconds),
                     default_sets = COALESCE(?5, default_sets),
                     default_target_reps = COALESCE(?6, default_target_reps),
                     first_week_day = COALESCE(?7, first_week_day),
                     language = COALESCE(?8, language)
                 WHERE id = 1",
                params![
                    &settings.gender,
                    settings.dob,
                    &settings.unit,
                    settings.rest_seconds,
                    settings.default_sets,
                    settings.target_reps,
                    settings.first_week_day,
                    &settings.language,
                ],
            )?;
        }

        for measurement in &backup.measurements {
            tx.execute(
                "INSERT INTO measurement
                     (recorded_at, weight_kg, fat_percent, height_cm, chest_cm, arms_cm, waist_cm,
                      calves_cm, hips_cm, thighs_cm, shoulders_cm, neck_cm, forearms_cm)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    measurement.recorded_at,
                    measurement.weight_kg,
                    measurement.fat_percent,
                    measurement.height_cm,
                    measurement.chest_cm,
                    measurement.arms_cm,
                    measurement.waist_cm,
                    measurement.calves_cm,
                    measurement.hips_cm,
                    measurement.thighs_cm,
                    measurement.shoulders_cm,
                    measurement.neck_cm,
                    measurement.forearms_cm,
                ],
            )?;
        }

        let mut plan_ids = HashMap::new();
        let mut day_ids = HashMap::new();
        let mut active_id = None;
        for plan in &backup.plans {
            tx.execute(
                "INSERT INTO plan
                     (name, level, goal, split, block_weeks, block_enabled, block_started_at,
                      is_active, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)",
                params![
                    &plan.name,
                    &plan.level,
                    &plan.goal,
                    &plan.split,
                    plan.block_weeks,
                    plan.block_enabled,
                    plan.block_started_at,
                    plan.created_at,
                ],
            )?;
            let plan_id = tx.last_insert_rowid();
            plan_ids.insert(plan.name.clone(), plan_id);
            if plan.active {
                active_id = Some(plan_id);
            }
            for (day_index, day) in plan.days.iter().enumerate() {
                tx.execute(
                    "INSERT INTO plan_day (plan_id, name, weekday, order_index)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        plan_id,
                        &day.name,
                        day.weekday,
                        i64::try_from(day_index).unwrap_or(i64::MAX),
                    ],
                )?;
                let day_id = tx.last_insert_rowid();
                day_ids.insert((plan.name.clone(), day.name.clone()), day_id);
                for (exercise_index, exercise) in day.exercises.iter().enumerate() {
                    tx.execute(
                        "INSERT INTO plan_exercise
                             (plan_day_id, exercise_id, order_index, rest_seconds, superset_group)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            day_id,
                            exercise.exercise_id,
                            i64::try_from(exercise_index).unwrap_or(i64::MAX),
                            exercise.rest_seconds,
                            exercise.superset,
                        ],
                    )?;
                    let exercise_id = tx.last_insert_rowid();
                    for set in &exercise.sets {
                        tx.execute(
                            "INSERT INTO plan_set
                                 (plan_exercise_id, set_number, target_weight, target_reps,
                                  target_seconds, target_distance_m)
                             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                            params![
                                exercise_id,
                                set.set_number,
                                set.weight,
                                set.reps,
                                set.seconds,
                                set.distance_m,
                            ],
                        )?;
                    }
                }
            }
        }
        if let Some(id) = active_id {
            tx.execute("UPDATE plan SET is_active = 1 WHERE id = ?1", [id])?;
        }

        for workout in &backup.workouts {
            let plan_id = workout
                .plan
                .as_ref()
                .and_then(|name| plan_ids.get(name).copied());
            let day_id = match (workout.plan.as_ref(), workout.day.as_ref()) {
                (Some(plan), Some(day)) => {
                    day_ids.get(&(plan.clone(), day.clone())).copied()
                }
                _ => None,
            };
            tx.execute(
                "INSERT INTO session (plan_id, plan_day_id, name, started_at, ended_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    plan_id,
                    day_id,
                    &workout.name,
                    workout.started_at,
                    workout.ended_at,
                ],
            )?;
            let session_id = tx.last_insert_rowid();
            for (exercise_index, exercise) in workout.exercises.iter().enumerate() {
                tx.execute(
                    "INSERT INTO session_exercise
                         (session_id, exercise_id, name, order_index, superset_group)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        session_id,
                        exercise.exercise_id,
                        &exercise.exercise_name,
                        i64::try_from(exercise_index).unwrap_or(i64::MAX),
                        exercise.superset,
                    ],
                )?;
                let exercise_id = tx.last_insert_rowid();
                for set in &exercise.sets {
                    tx.execute(
                        "INSERT INTO session_set
                             (session_exercise_id, set_number, weight, reps, seconds, distance_m,
                              completed_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            exercise_id,
                            set.set_number,
                            set.weight,
                            set.reps,
                            set.seconds,
                            set.distance_m,
                            set.completed_at,
                        ],
                    )?;
                }
            }
        }

        tx.commit()?;
        Ok(())
    })
}

pub fn save_file(name: &str, contents: &str) -> Result<String> {
    save(name, contents)
}

#[cfg(not(target_os = "android"))]
fn save(name: &str, contents: &str) -> Result<String> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join("Downloads");
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join(name);
    std::fs::write(&path, contents).with_context(|| format!("write {}", path.display()))?;
    Ok(path.display().to_string())
}

#[cfg(target_os = "android")]
fn save(name: &str, contents: &str) -> Result<String> {
    android::save(name, contents)
}

#[cfg(target_os = "android")]
mod android {
    use std::path::PathBuf;

    use anyhow::{bail, Context, Result};
    use jni::{
        objects::{JObject, JString, JValue},
        JNIEnv, JavaVM,
    };

    pub fn save(name: &str, contents: &str) -> Result<String> {
        let ctx = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
        let mut env = vm.attach_current_thread()?;
        let activity = unsafe { JObject::from_raw(ctx.context().cast()) };
        let sdk = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")?
            .i()?;
        if sdk >= 29 {
            downloads(&mut env, &activity, name, contents)
        } else {
            external(&mut env, &activity, name, contents)
        }
    }

    fn downloads(
        env: &mut JNIEnv,
        activity: &JObject,
        name: &str,
        contents: &str,
    ) -> Result<String> {
        let resolver = env
            .call_method(
                activity,
                "getContentResolver",
                "()Landroid/content/ContentResolver;",
                &[],
            )?
            .l()?;
        let values = env.new_object("android/content/ContentValues", "()V", &[])?;
        put(env, &values, "DISPLAY_NAME", name)?;
        put(env, &values, "MIME_TYPE", "text/csv")?;
        put(env, &values, "RELATIVE_PATH", "Download/")?;
        let collection = env
            .get_static_field(
                "android/provider/MediaStore$Downloads",
                "EXTERNAL_CONTENT_URI",
                "Landroid/net/Uri;",
            )?
            .l()?;
        let uri = env
            .call_method(
                &resolver,
                "insert",
                "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
                &[JValue::Object(&collection), JValue::Object(&values)],
            )?
            .l()?;
        if uri.is_null() {
            bail!("Downloads refused the file");
        }
        if let Err(error) = write_stream(env, &resolver, &uri, contents) {
            delete(env, &resolver, &uri);
            return Err(error);
        }
        Ok(format!("Download/{name}"))
    }

    fn write_stream(
        env: &mut JNIEnv,
        resolver: &JObject,
        uri: &JObject,
        contents: &str,
    ) -> Result<()> {
        let stream = env
            .call_method(
                resolver,
                "openOutputStream",
                "(Landroid/net/Uri;)Ljava/io/OutputStream;",
                &[JValue::Object(uri)],
            )?
            .l()?;
        let bytes = env.byte_array_from_slice(contents.as_bytes())?;
        env.call_method(
            &stream,
            "write",
            "([B)V",
            &[JValue::Object(&bytes)],
        )?;
        env.call_method(&stream, "close", "()V", &[])?;
        Ok(())
    }

    fn delete(env: &mut JNIEnv, resolver: &JObject, uri: &JObject) {
        let _ = env.call_method(
            resolver,
            "delete",
            "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
            &[
                JValue::Object(uri),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        );
    }

    fn put(env: &mut JNIEnv, values: &JObject, key: &str, value: &str) -> Result<()> {
        let key = env.new_string(key)?;
        let value = env.new_string(value)?;
        env.call_method(
            values,
            "put",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&key), JValue::Object(&value)],
        )?;
        Ok(())
    }

    fn external(
        env: &mut JNIEnv,
        activity: &JObject,
        name: &str,
        contents: &str,
    ) -> Result<String> {
        let dir = env
            .call_method(
                activity,
                "getExternalFilesDir",
                "(Ljava/lang/String;)Ljava/io/File;",
                &[JValue::Object(&JObject::null())],
            )?
            .l()?;
        if dir.is_null() {
            bail!("External storage is unavailable");
        }
        let raw = env
            .call_method(&dir, "getAbsolutePath", "()Ljava/lang/String;", &[])?
            .l()?;
        let dir_path = java_string(env, &raw)?;
        let path = PathBuf::from(dir_path).join(name);
        std::fs::write(&path, contents).with_context(|| format!("write {}", path.display()))?;
        Ok(path.display().to_string())
    }

    fn java_string(env: &mut JNIEnv, value: &JObject) -> Result<String> {
        let value = <&JString>::from(value);
        Ok(env.get_string(value)?.into())
    }
}
