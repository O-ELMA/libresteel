use std::{
    fs,
    path::PathBuf,
    sync::{Mutex, OnceLock, PoisonError},
};

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::suggest::Block;

// Embedded so launch copies a database instead of parsing the 2.3 MB JSON.
const PREBUILT: &[u8] = include_bytes!("../assets/jefeet.db");

static DB: OnceLock<Mutex<Connection>> = OnceLock::new();

pub fn init() -> Result<()> {
    let dir = app_data_dir()?;
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join("jefeet.db");
    if !path.exists() {
        // A kill mid-copy must not leave a truncated database that a later launch trusts.
        let tmp = dir.join("jefeet.db.tmp");
        fs::write(&tmp, PREBUILT).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
    }
    let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
    conn.pragma_update(None, "foreign_keys", true)?;
    migrate(&conn, &dir)?;
    // Connection is Send + !Sync, and every later write goes through this one handle.
    DB.get_or_init(|| Mutex::new(conn));
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn migrate(conn: &Connection, dir: &std::path::Path) -> Result<()> {
    conn.execute("INSERT OR IGNORE INTO user_setting (id) VALUES (1)", [])?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < 2 {
        if !column_exists(conn, "plan", "split")? {
            conn.execute(
                "ALTER TABLE plan ADD COLUMN split TEXT NOT NULL DEFAULT 'numbered'",
                [],
            )?;
        }
        conn.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        if !column_exists(conn, "exercise", "body_parts_json")? {
            conn.execute("ALTER TABLE exercise ADD COLUMN body_parts_json TEXT", [])?;
        }
        let missing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM exercise WHERE body_parts_json IS NULL",
            [],
            |row| row.get(0),
        )?;
        if missing > 0 {
            backfill_body_parts(conn, dir)?;
        }
        conn.pragma_update(None, "user_version", 3)?;
    }
    if version < 4 {
        if !column_exists(conn, "user_setting", "first_week_day")? {
            conn.execute(
                "ALTER TABLE user_setting ADD COLUMN first_week_day INTEGER NOT NULL DEFAULT 0",
                [],
            )?;
        }
        conn.pragma_update(None, "user_version", 4)?;
    }
    if version < 5 {
        if !column_exists(conn, "user_setting", "language")? {
            conn.execute(
                "ALTER TABLE user_setting ADD COLUMN language TEXT NOT NULL DEFAULT 'en'",
                [],
            )?;
        }
        conn.pragma_update(None, "user_version", 5)?;
    }
    if version < 6 {
        if !column_exists(conn, "session", "rest_until")? {
            conn.execute("ALTER TABLE session ADD COLUMN rest_until INTEGER", [])?;
        }
        conn.pragma_update(None, "user_version", 6)?;
    }
    if version < 7 {
        conn.execute(
            "UPDATE user_setting SET rest_seconds = 60 WHERE id = 1 AND rest_seconds = 150",
            [],
        )?;
        conn.execute(
            "UPDATE user_setting SET default_target_reps = 12 WHERE id = 1 AND default_target_reps = 10",
            [],
        )?;
        conn.pragma_update(None, "user_version", 7)?;
    }
    if version < 8 {
        for column in ["name_ar", "description_ar", "name_fr", "description_fr"] {
            if !column_exists(conn, "exercise", column)? {
                conn.execute(
                    &format!("ALTER TABLE exercise ADD COLUMN {column} TEXT"),
                    [],
                )?;
            }
        }
        let missing: i64 = conn.query_row(
            "SELECT COUNT(*) FROM exercise WHERE COALESCE(name_ar, '') = ''",
            [],
            |row| row.get(0),
        )?;
        if missing > 0 {
            backfill_translations(conn, dir)?;
        }
        conn.pragma_update(None, "user_version", 8)?;
    }
    if version < 9 {
        add_column(conn, "plan", "block_weeks", "INTEGER NOT NULL DEFAULT 4")?;
        add_column(conn, "plan", "block_started_at", "INTEGER")?;
        conn.pragma_update(None, "user_version", 9)?;
    }
    if version < 10 {
        add_column(
            conn,
            "user_setting",
            "block_enabled",
            "INTEGER NOT NULL DEFAULT 1",
        )?;
        conn.pragma_update(None, "user_version", 10)?;
    }
    if version < 12 {
        migrate_routine_blocks(conn)?;
        conn.pragma_update(None, "user_version", 12)?;
    }
    if version < 13 {
        add_column(conn, "plan_exercise", "superset_group", "INTEGER")?;
        add_column(conn, "session_exercise", "superset_group", "INTEGER")?;
        conn.pragma_update(None, "user_version", 13)?;
    }
    if version < 14 {
        add_column(
            conn,
            "user_setting",
            "onboarded",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        conn.pragma_update(None, "user_version", 14)?;
    }
    Ok(())
}

fn migrate_routine_blocks(conn: &Connection) -> Result<()> {
    let added = !column_exists(conn, "plan", "block_enabled")?;
    add_column(conn, "plan", "block_enabled", "INTEGER NOT NULL DEFAULT 1")?;
    if added {
        let enabled: i64 = conn
            .query_row(
                "SELECT block_enabled FROM user_setting WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(1);
        conn.execute("UPDATE plan SET block_enabled = ?1", [enabled])?;
    }
    Ok(())
}

fn add_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    if !column_exists(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    Ok(conn
        .prepare(&format!(
            "SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1"
        ))?
        .exists([column])?)
}

fn backfill_body_parts(conn: &Connection, dir: &std::path::Path) -> Result<()> {
    with_seed(conn, dir, |conn| {
        conn.execute(
            "UPDATE exercise SET body_parts_json = COALESCE(
                 (SELECT s.body_parts_json FROM seed.exercise s WHERE s.id = exercise.id), '[]')",
            [],
        )?;
        Ok(())
    })
}

fn backfill_translations(conn: &Connection, dir: &std::path::Path) -> Result<()> {
    with_seed(conn, dir, |conn| {
        conn.execute(
            "UPDATE exercise SET
                 name_ar = (SELECT s.name_ar FROM seed.exercise s WHERE s.id = exercise.id),
                 description_ar = (SELECT s.description_ar FROM seed.exercise s WHERE s.id = exercise.id),
                 name_fr = (SELECT s.name_fr FROM seed.exercise s WHERE s.id = exercise.id),
                 description_fr = (SELECT s.description_fr FROM seed.exercise s WHERE s.id = exercise.id)",
            [],
        )?;
        Ok(())
    })
}

fn with_seed<T>(
    conn: &Connection,
    dir: &std::path::Path,
    copy: impl FnOnce(&Connection) -> Result<T>,
) -> Result<T> {
    let seed = dir.join("jefeet.seed.tmp");
    fs::write(&seed, PREBUILT).with_context(|| format!("write {}", seed.display()))?;
    let seed_path = seed
        .to_str()
        .with_context(|| format!("seed path {} is not UTF-8", seed.display()))?;
    conn.execute("ATTACH DATABASE ?1 AS seed", [seed_path])?;
    let result = copy(conn);
    conn.execute("DETACH DATABASE seed", [])?;
    let _ = fs::remove_file(&seed);
    result
}

fn localized(conn: &Connection, column: &str) -> String {
    let language = conn
        .query_row(
            "SELECT language FROM user_setting WHERE id = 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "en".to_string());
    match language.as_str() {
        "ar" => format!("COALESCE({column}_ar, {column})"),
        "fr" => format!("COALESCE({column}_fr, {column})"),
        _ => column.to_string(),
    }
}

pub fn group_counts() -> Result<Vec<(String, i64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT muscle_main, COUNT(*) FROM exercise
         WHERE muscle_main IS NOT NULL
         GROUP BY 1 ORDER BY 2 DESC",
    )?;
    let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let counts = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(counts)
}

#[derive(Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: i64,
    pub name: String,
    pub muscles: Vec<String>,
    pub equipment_json: String,
    pub body_parts_json: String,
    pub difficulty: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ExerciseDetail {
    pub name: String,
    pub description: String,
    pub difficulty: Option<String>,
    pub exercise_type: String,
    pub log_type: String,
    pub muscles_main: Vec<String>,
    pub muscles_secondary: Vec<String>,
    pub equipment_json: String,
}

pub fn exercises() -> Result<Vec<Exercise>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name = localized(&conn, "name");
    let mut stmt = conn.prepare(&format!(
        "SELECT id, {name},
                json_extract(muscles_json, '$.main'),
                json_extract(muscles_json, '$.secondary'),
                equipment_json, COALESCE(body_parts_json, '[]'), difficulty
         FROM exercise ORDER BY popularity DESC"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok(Exercise {
            id: row.get(0)?,
            name: row.get(1)?,
            muscles: muscles(row.get(2)?, row.get(3)?),
            equipment_json: row.get(4)?,
            body_parts_json: row.get(5)?,
            difficulty: row.get(6)?,
        })
    })?;
    let exercises = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(exercises)
}

pub fn exercise(id: i64) -> Result<Option<ExerciseDetail>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name = localized(&conn, "name");
    let description = localized(&conn, "description");
    let mut stmt = conn.prepare(&format!(
        "SELECT {name}, {description}, difficulty, exercise_type, log_type,
                json_extract(muscles_json, '$.main'),
                json_extract(muscles_json, '$.secondary'),
                equipment_json
         FROM exercise WHERE id = ?1"
    ))?;
    let mut rows = stmt.query_map([id], |row| {
        Ok(ExerciseDetail {
            name: row.get(0)?,
            description: row.get(1)?,
            difficulty: row.get(2)?,
            exercise_type: row.get(3)?,
            log_type: row.get(4)?,
            muscles_main: owned_list(row.get(5)?),
            muscles_secondary: owned_list(row.get(6)?),
            equipment_json: row.get(7)?,
        })
    })?;
    let detail = rows.next().transpose()?;
    drop(rows);
    drop(stmt);
    drop(conn);
    Ok(detail)
}

fn owned_list(raw: Option<String>) -> Vec<String> {
    let mut list = Vec::new();
    if let Some(json) = raw {
        list.extend(json_list(&json).into_iter().map(str::to_string));
    }
    list
}

fn muscles(main: Option<String>, secondary: Option<String>) -> Vec<String> {
    let mut muscles = owned_list(main);
    muscles.extend(owned_list(secondary));
    muscles
}

// shortcut: values in the shipped dataset hold no commas or escapes, revisit if the seed ever changes.
pub fn json_list(raw: &str) -> Vec<&str> {
    raw.trim_matches(['[', ']'])
        .split(',')
        .map(|item| item.trim().trim_matches('"'))
        .filter(|item| !item.is_empty())
        .collect()
}

pub fn with_connection<T>(f: impl FnOnce(&mut Connection) -> Result<T>) -> Result<T> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    f(&mut conn)
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| {
            i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX)
        })
}

pub fn unit_lb() -> bool {
    let Some(db) = DB.get() else {
        return false;
    };
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row("SELECT unit FROM user_setting WHERE id = 1", [], |row| {
        row.get::<_, String>(0)
    })
    .is_ok_and(|unit| unit == "lb")
}

pub fn set_unit(lb: bool) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE user_setting SET unit = ?1 WHERE id = 1",
        [if lb { "lb" } else { "kg" }],
    )?;
    drop(conn);
    Ok(())
}

pub fn set_display_name(value: &str) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE user_setting SET display_name = NULLIF(TRIM(?1), '') WHERE id = 1",
        [value],
    )?;
    drop(conn);
    Ok(())
}

pub fn onboarded() -> bool {
    let Some(db) = DB.get() else {
        return false;
    };
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row(
        "SELECT onboarded FROM user_setting WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .is_ok_and(|value| value != 0)
}

pub fn finish_onboarding() -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("UPDATE user_setting SET onboarded = 1 WHERE id = 1", [])?;
    drop(conn);
    Ok(())
}

pub fn first_week_day() -> u32 {
    let Some(db) = DB.get() else {
        return 0;
    };
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row(
        "SELECT first_week_day FROM user_setting WHERE id = 1",
        [],
        |row| row.get::<_, i64>(0),
    )
    .map_or(0, |value| u32::try_from(value).unwrap_or(0) % 7)
}

pub fn set_first_week_day(value: u32) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE user_setting SET first_week_day = ?1 WHERE id = 1",
        [i64::from(value % 7)],
    )?;
    drop(conn);
    Ok(())
}

const DEFAULT_REST: i64 = 60;
const DEFAULT_SETS: i64 = 3;
const DEFAULT_REPS: i64 = 12;

pub fn default_rest() -> i64 {
    let Some(db) = DB.get() else {
        return DEFAULT_REST;
    };
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row(
        "SELECT rest_seconds FROM user_setting WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .unwrap_or(DEFAULT_REST)
}

pub fn set_default_rest(seconds: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE user_setting SET rest_seconds = ?1 WHERE id = 1",
        [seconds.max(0)],
    )?;
    drop(conn);
    Ok(())
}

pub fn language() -> String {
    let Some(db) = DB.get() else {
        return "en".to_string();
    };
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row(
        "SELECT language FROM user_setting WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .unwrap_or_else(|_| "en".to_string())
}

pub fn set_language(language: &str) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE user_setting SET language = ?1 WHERE id = 1",
        [language],
    )?;
    drop(conn);
    Ok(())
}

pub fn gender() -> Option<String> {
    let db = DB.get()?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row("SELECT gender FROM user_setting WHERE id = 1", [], |row| {
        row.get::<_, Option<String>>(0)
    })
    .ok()
    .flatten()
    .filter(|value| !value.is_empty())
}

pub fn set_gender(value: &str) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("UPDATE user_setting SET gender = ?1 WHERE id = 1", [value])?;
    drop(conn);
    Ok(())
}

pub fn dob() -> Option<i64> {
    let db = DB.get()?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.query_row("SELECT dob FROM user_setting WHERE id = 1", [], |row| {
        row.get::<_, Option<i64>>(0)
    })
    .ok()
    .flatten()
}

pub fn set_dob(value: Option<i64>) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("UPDATE user_setting SET dob = ?1 WHERE id = 1", [value])?;
    drop(conn);
    Ok(())
}

fn body_column(field: &str) -> Result<&'static str> {
    Ok(match field {
        "weight" => "weight_kg",
        "fat" => "fat_percent",
        "waist" => "waist_cm",
        "chest" => "chest_cm",
        "arms" => "arms_cm",
        "forearms" => "forearms_cm",
        "shoulders" => "shoulders_cm",
        "hips" => "hips_cm",
        "upper_leg" => "thighs_cm",
        "lower_leg" => "calves_cm",
        "neck" => "neck_cm",
        "height" => "height_cm",
        _ => return Err(anyhow::anyhow!("unknown body field {field}")),
    })
}

pub fn body_latest(field: &str) -> Result<Option<(i64, f64)>> {
    let column = body_column(field)?;
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(conn
        .query_row(
            &format!(
                "SELECT recorded_at, {column} FROM measurement
                 WHERE {column} IS NOT NULL
                 ORDER BY recorded_at DESC
                 LIMIT 1"
            ),
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?)
}

pub fn save_body(field: &str, value: f64) -> Result<()> {
    let column = body_column(field)?;
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        &format!("INSERT INTO measurement (recorded_at, {column}) VALUES (?1, ?2)"),
        params![now(), value],
    )?;
    drop(conn);
    Ok(())
}

pub fn save_measurement(
    weight_kg: Option<f64>,
    fat_percent: Option<f64>,
    height_cm: Option<f64>,
) -> Result<()> {
    if weight_kg.is_none() && fat_percent.is_none() && height_cm.is_none() {
        return Ok(());
    }
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "INSERT INTO measurement (recorded_at, weight_kg, fat_percent, height_cm)
         VALUES (?1, ?2, ?3, ?4)",
        params![now(), weight_kg, fat_percent, height_cm],
    )?;
    drop(conn);
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct Routine {
    pub id: i64,
    pub name: String,
    pub level: Option<String>,
    pub goal: Option<String>,
    pub split: String,
    pub block_weeks: i64,
    pub block_enabled: bool,
    pub is_active: bool,
    pub day_count: i64,
}

pub fn routines() -> Result<Vec<Routine>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.level, p.goal, p.split, p.block_weeks, p.block_enabled,
                p.is_active, (SELECT COUNT(*) FROM plan_day d WHERE d.plan_id = p.id)
         FROM plan p ORDER BY p.is_active DESC, p.created_at",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Routine {
            id: row.get(0)?,
            name: row.get(1)?,
            level: row.get(2)?,
            goal: row.get(3)?,
            split: row.get(4)?,
            block_weeks: row.get(5)?,
            block_enabled: row.get(6)?,
            is_active: row.get(7)?,
            day_count: row.get(8)?,
        })
    })?;
    let routines = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(routines)
}

pub fn routine_goal(routine_id: Option<i64>) -> Result<Option<String>> {
    let Some(routine_id) = routine_id else {
        return Ok(None);
    };
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let goal = conn
        .query_row("SELECT goal FROM plan WHERE id = ?1", [routine_id], |row| {
            row.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten();
    drop(conn);
    Ok(goal)
}

pub fn routine_block(routine_id: Option<i64>, session_start: i64) -> Result<Option<Block>> {
    let Some(routine_id) = routine_id else {
        return Ok(None);
    };
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let Some((weeks, started, enabled, days)) = conn
        .query_row(
            "SELECT p.block_weeks, p.block_started_at, p.block_enabled,
                    (SELECT COUNT(*) FROM plan_day d WHERE d.plan_id = p.id)
             FROM plan p WHERE p.id = ?1",
            [routine_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    if enabled == 0 {
        return Ok(None);
    }
    let weeks = weeks.max(1);
    let days = usize::try_from(days.max(1)).unwrap_or(1);
    let Some(started) = started else {
        conn.execute(
            "UPDATE plan SET block_started_at = ?1 WHERE id = ?2",
            params![session_start, routine_id],
        )?;
        drop(conn);
        return Ok(Some(Block {
            week: 1,
            weeks,
            pass_start: session_start,
        }));
    };
    let mut stmt = conn.prepare(
        "SELECT started_at FROM session
         WHERE plan_id = ?1 AND ended_at IS NOT NULL AND started_at >= ?2
         ORDER BY started_at",
    )?;
    let rows = stmt.query_map(params![routine_id, started], |row| row.get::<_, i64>(0))?;
    let sessions = rows.collect::<rusqlite::Result<Vec<i64>>>()?;
    drop(stmt);
    let passes = sessions.len().checked_div(days).unwrap_or(0);
    let next = i64::try_from(passes).unwrap_or(i64::MAX).saturating_add(1);
    let next_start = sessions
        .get(passes.saturating_mul(days))
        .copied()
        .unwrap_or(session_start);
    let (week, pass_start) = if next > weeks.saturating_add(1) {
        conn.execute(
            "UPDATE plan SET block_started_at = ?1 WHERE id = ?2",
            params![session_start, routine_id],
        )?;
        (1, session_start)
    } else {
        (next, next_start)
    };
    drop(conn);
    Ok(Some(Block {
        week,
        weeks,
        pass_start,
    }))
}

pub const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

pub struct DaySeed {
    pub name: String,
    pub weekday: Option<i64>,
}

pub fn create_routine(
    name: &str,
    level: Option<&str>,
    goal: Option<&str>,
    split: &str,
    block_weeks: i64,
    block_enabled: bool,
    seeds: &[DaySeed],
) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO plan (name, level, goal, split, block_weeks, block_enabled, is_active, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NOT EXISTS(SELECT 1 FROM plan WHERE is_active = 1), ?7)",
        params![
            name,
            level,
            goal,
            split,
            block_weeks.max(1),
            block_enabled,
            now()
        ],
    )?;
    let id = tx.last_insert_rowid();
    for (index, seed) in seeds.iter().enumerate() {
        tx.execute(
            "INSERT INTO plan_day (plan_id, name, weekday, order_index)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                id,
                &seed.name,
                seed.weekday,
                i64::try_from(index).unwrap_or(i64::MAX)
            ],
        )?;
    }
    tx.commit()?;
    drop(conn);
    Ok(id)
}

pub fn update_routine(
    id: i64,
    name: &str,
    level: Option<&str>,
    goal: Option<&str>,
    split: &str,
    block_weeks: i64,
    block_enabled: bool,
) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE plan SET name = ?1, level = ?2, goal = ?3, split = ?4, block_weeks = ?5,
                block_enabled = ?6 WHERE id = ?7",
        params![
            name,
            level,
            goal,
            split,
            block_weeks.max(1),
            block_enabled,
            id
        ],
    )?;
    drop(conn);
    Ok(())
}

pub fn select_routine(id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    tx.execute("UPDATE plan SET is_active = 0", [])?;
    tx.execute("UPDATE plan SET is_active = 1 WHERE id = ?1", [id])?;
    tx.commit()?;
    drop(conn);
    Ok(())
}

pub fn delete_routine(id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("DELETE FROM plan WHERE id = ?1", [id])?;
    drop(conn);
    Ok(())
}

pub fn clone_routine(id: i64, name: &str) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO plan
             (name, level, goal, split, block_weeks, block_enabled, block_started_at, is_active, created_at)
         SELECT ?2, level, goal, split, block_weeks, block_enabled, NULL, 0, ?3
         FROM plan WHERE id = ?1",
        params![id, name, now()],
    )?;
    let new_id = tx.last_insert_rowid();
    let days = {
        let mut stmt =
            tx.prepare("SELECT id FROM plan_day WHERE plan_id = ?1 ORDER BY order_index")?;
        let rows = stmt.query_map([id], |row| row.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for day in days {
        tx.execute(
            "INSERT INTO plan_day (plan_id, name, weekday, order_index)
             SELECT ?2, name, weekday, order_index FROM plan_day WHERE id = ?1",
            params![day, new_id],
        )?;
        let new_day = tx.last_insert_rowid();
        copy_day_exercises(&tx, day, new_day)?;
    }
    tx.commit()?;
    drop(conn);
    Ok(new_id)
}

#[derive(Clone, PartialEq, Eq)]
pub struct RoutineDay {
    pub id: i64,
    pub routine_id: i64,
    pub name: String,
    pub weekday: Option<i64>,
    pub muscle_main: Option<String>,
    pub exercise_count: i64,
    pub est_minutes: i64,
    pub last_done: Option<i64>,
}

fn select_days(filter: &str, key: i64) -> Result<Vec<RoutineDay>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let sql = format!(
        "SELECT d.id, d.plan_id, d.name, d.weekday,
                (SELECT COUNT(*) FROM plan_exercise pe WHERE pe.plan_day_id = d.id),
                COALESCE((SELECT CAST(ROUND(SUM(
                     (SELECT COUNT(*) FROM plan_set ps WHERE ps.plan_exercise_id = pe.id)
                     * (45 + pe.rest_seconds)) / 60.0) AS INTEGER)
                  FROM plan_exercise pe WHERE pe.plan_day_id = d.id), 0),
                (SELECT MAX(s.ended_at) FROM session s
                 WHERE s.plan_day_id = d.id AND s.ended_at IS NOT NULL),
                (SELECT e.muscle_main FROM plan_exercise pe
                 JOIN exercise e ON e.id = pe.exercise_id
                 WHERE pe.plan_day_id = d.id ORDER BY pe.order_index LIMIT 1)
         FROM plan_day d WHERE {filter} ORDER BY d.order_index"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([key], |row| {
        Ok(RoutineDay {
            id: row.get(0)?,
            routine_id: row.get(1)?,
            name: row.get(2)?,
            weekday: row.get(3)?,
            exercise_count: row.get(4)?,
            est_minutes: row.get(5)?,
            last_done: row.get(6)?,
            muscle_main: row.get(7)?,
        })
    })?;
    let days = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(days)
}

pub fn days(routine_id: i64) -> Result<Vec<RoutineDay>> {
    select_days("d.plan_id = ?1", routine_id)
}

pub fn add_day(routine_id: i64, name: &str, weekday: Option<i64>) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "INSERT INTO plan_day (plan_id, name, weekday, order_index)
         VALUES (?1, ?2, ?3,
                 COALESCE((SELECT MAX(order_index) + 1 FROM plan_day WHERE plan_id = ?1), 0))",
        params![routine_id, name, weekday],
    )?;
    let id = conn.last_insert_rowid();
    drop(conn);
    Ok(id)
}

pub fn clone_day(day_id: i64, routine_id: i64, name: &str) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    let weekday = tx
        .query_row(
            "SELECT weekday FROM plan_day WHERE id = ?1",
            [day_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?
        .with_context(|| format!("routine day {day_id} does not exist"))?;
    tx.execute(
        "INSERT INTO plan_day (plan_id, name, weekday, order_index)
         VALUES (?1, ?2, ?3,
                 COALESCE((SELECT MAX(order_index) + 1 FROM plan_day WHERE plan_id = ?1), 0))",
        params![routine_id, name, weekday],
    )?;
    let new_id = tx.last_insert_rowid();
    copy_day_exercises(&tx, day_id, new_id)?;
    tx.commit()?;
    drop(conn);
    Ok(new_id)
}

fn copy_day_exercises(conn: &Connection, source: i64, target: i64) -> Result<()> {
    let ids = {
        let mut stmt = conn
            .prepare("SELECT id FROM plan_exercise WHERE plan_day_id = ?1 ORDER BY order_index")?;
        let rows = stmt.query_map([source], |row| row.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for id in ids {
        conn.execute(
            "INSERT INTO plan_exercise (plan_day_id, exercise_id, order_index, rest_seconds, superset_group)
             SELECT ?2, exercise_id, order_index, rest_seconds, superset_group FROM plan_exercise WHERE id = ?1",
            params![id, target],
        )?;
        let new_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO plan_set
                 (plan_exercise_id, set_number, target_weight, target_reps, target_seconds, target_distance_m)
             SELECT ?2, set_number, target_weight, target_reps, target_seconds, target_distance_m
             FROM plan_set WHERE plan_exercise_id = ?1",
            params![id, new_id],
        )?;
    }
    Ok(())
}

pub fn update_day(id: i64, name: &str, weekday: Option<i64>) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE plan_day SET name = ?1, weekday = ?2 WHERE id = ?3",
        params![name, weekday, id],
    )?;
    drop(conn);
    Ok(())
}

pub fn delete_day(id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("DELETE FROM plan_day WHERE id = ?1", [id])?;
    drop(conn);
    Ok(())
}

pub fn set_day_order(routine_id: i64, ids: &[i64], split: &str) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    write_day_order(&tx, routine_id, ids, split, split)?;
    tx.commit()?;
    drop(conn);
    Ok(())
}

pub fn renumber_days(routine_id: i64, old_split: &str, new_split: &str) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    let mut stmt = tx.prepare("SELECT id FROM plan_day WHERE plan_id = ?1 ORDER BY order_index")?;
    let rows = stmt.query_map([routine_id], |row| row.get::<_, i64>(0))?;
    let ids = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    write_day_order(&tx, routine_id, &ids, old_split, new_split)?;
    tx.commit()?;
    drop(conn);
    Ok(())
}

fn write_day_order(
    conn: &Connection,
    routine_id: i64,
    ids: &[i64],
    old_split: &str,
    new_split: &str,
) -> Result<()> {
    let mut stmt = conn.prepare("SELECT id, name, order_index FROM plan_day WHERE plan_id = ?1")?;
    let rows = stmt
        .query_map([routine_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    for (index, id) in ids.iter().enumerate() {
        let Some((_, old_name, old_order)) = rows.iter().find(|(row_id, _, _)| row_id == id) else {
            continue;
        };
        let old_index = usize::try_from(*old_order).unwrap_or(usize::MAX);
        let (default_name, weekday) = day_label(new_split, index);
        let was_default = *old_name == day_label(old_split, old_index).0;
        let name = if was_default {
            default_name
        } else {
            old_name.clone()
        };
        conn.execute(
            "UPDATE plan_day
             SET order_index = ?1, name = ?2, weekday = ?3
             WHERE id = ?4 AND plan_id = ?5",
            params![
                i64::try_from(index).unwrap_or(i64::MAX),
                name,
                weekday,
                id,
                routine_id
            ],
        )?;
    }
    Ok(())
}

fn day_label(split: &str, index: usize) -> (String, Option<i64>) {
    if split == "weekday" {
        if let Some(name) = WEEKDAY_NAMES.get(index) {
            return ((*name).to_string(), i64::try_from(index).ok());
        }
    }
    (format!("Day {}", index.saturating_add(1)), None)
}

#[derive(Clone, PartialEq)]
pub struct RoutineSet {
    pub set_number: i64,
    pub target_weight: Option<f64>,
    pub target_reps: Option<i64>,
    pub target_seconds: Option<i64>,
    pub target_distance_m: Option<f64>,
}

#[derive(Clone, PartialEq)]
pub struct RoutineExercise {
    pub id: i64,
    pub exercise_id: i64,
    pub name: String,
    pub muscle_main: Option<String>,
    pub log_type: String,
    pub rest_seconds: i64,
    pub superset: Option<i64>,
    pub sets: Vec<RoutineSet>,
}

pub fn day_exercises(day_id: i64) -> Result<Vec<RoutineExercise>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name = localized(&conn, "e.name");
    let mut stmt = conn.prepare(&format!(
        "SELECT pe.id, e.id, {name}, e.muscle_main, e.log_type, pe.rest_seconds, pe.superset_group
         FROM plan_exercise pe JOIN exercise e ON e.id = pe.exercise_id
         WHERE pe.plan_day_id = ?1 ORDER BY pe.order_index"
    ))?;
    let rows = stmt.query_map([day_id], |row| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
        ))
    })?;
    let base = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut sets_stmt = conn.prepare(
        "SELECT set_number, target_weight, target_reps, target_seconds, target_distance_m
         FROM plan_set WHERE plan_exercise_id = ?1 ORDER BY set_number",
    )?;
    let mut exercises = Vec::new();
    for (id, exercise_id, name, muscle_main, log_type, rest_seconds, superset) in base {
        let rows = sets_stmt.query_map([id], |row| {
            Ok(RoutineSet {
                set_number: row.get(0)?,
                target_weight: row.get(1)?,
                target_reps: row.get(2)?,
                target_seconds: row.get(3)?,
                target_distance_m: row.get(4)?,
            })
        })?;
        let sets = rows.collect::<Result<Vec<_>, _>>()?;
        exercises.push(RoutineExercise {
            id,
            exercise_id,
            name,
            muscle_main,
            log_type,
            rest_seconds,
            superset,
            sets,
        });
    }
    drop(sets_stmt);
    drop(conn);
    Ok(exercises)
}

pub fn add_exercises(day_id: i64, exercise_ids: &[i64]) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    for exercise_id in exercise_ids {
        insert_exercise(&tx, day_id, *exercise_id)?;
    }
    tx.commit()?;
    drop(conn);
    Ok(())
}

fn insert_exercise(conn: &Connection, day_id: i64, exercise_id: i64) -> Result<i64> {
    conn.execute(
        "INSERT INTO plan_exercise (plan_day_id, exercise_id, order_index, rest_seconds)
         VALUES (?1, ?2,
                 COALESCE((SELECT MAX(order_index) + 1 FROM plan_exercise WHERE plan_day_id = ?1), 0),
                 COALESCE((SELECT rest_seconds FROM user_setting WHERE id = 1), 60))",
        params![day_id, exercise_id],
    )?;
    let id = conn.last_insert_rowid();
    let log_type: String = conn.query_row(
        "SELECT log_type FROM exercise WHERE id = ?1",
        [exercise_id],
        |row| row.get(0),
    )?;
    let (default_sets, default_reps): (i64, i64) = conn
        .query_row(
            "SELECT default_sets, default_target_reps FROM user_setting WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .unwrap_or((DEFAULT_SETS, DEFAULT_REPS));
    let (reps, seconds) = match log_type.as_str() {
        "duration" => (None, Some(45)),
        "cardio" => (None, None),
        "reps_and_duration" => (Some(default_reps), Some(45)),
        _ => (Some(default_reps), None),
    };
    conn.execute(
        "INSERT INTO plan_set (plan_exercise_id, set_number, target_reps, target_seconds)
         WITH RECURSIVE seq(n) AS (SELECT 1 UNION ALL SELECT n + 1 FROM seq WHERE n + 1 <= ?2)
         SELECT ?1, n, ?3, ?4 FROM seq",
        params![id, default_sets, reps, seconds],
    )?;
    Ok(id)
}

pub fn clone_exercise(plan_exercise_id: i64) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO plan_exercise (plan_day_id, exercise_id, order_index, rest_seconds)
         SELECT plan_day_id, exercise_id,
                COALESCE((SELECT MAX(order_index) + 1 FROM plan_exercise
                          WHERE plan_day_id = source.plan_day_id), 0),
                rest_seconds
         FROM plan_exercise source WHERE id = ?1",
        [plan_exercise_id],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO plan_set
             (plan_exercise_id, set_number, target_weight, target_reps, target_seconds, target_distance_m)
         SELECT ?2, set_number, target_weight, target_reps, target_seconds, target_distance_m
         FROM plan_set WHERE plan_exercise_id = ?1",
        params![plan_exercise_id, id],
    )?;
    tx.commit()?;
    drop(conn);
    Ok(id)
}

pub fn replace_exercise(plan_exercise_id: i64, exercise_id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE plan_exercise SET exercise_id = ?2 WHERE id = ?1",
        params![plan_exercise_id, exercise_id],
    )?;
    drop(conn);
    Ok(())
}

pub fn day_muscle_counts(day_id: i64) -> Result<Vec<(String, i64)>> {
    muscle_counts("d.id = ?1", day_id)
}

pub fn routine_muscle_counts(routine_id: i64) -> Result<Vec<(String, i64)>> {
    muscle_counts("d.plan_id = ?1", routine_id)
}

fn muscle_counts(filter: &str, key: i64) -> Result<Vec<(String, i64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(&format!(
        "SELECT e.muscle_main, COUNT(*)
         FROM plan_day d
         JOIN plan_exercise pe ON pe.plan_day_id = d.id
         JOIN exercise e ON e.id = pe.exercise_id
         WHERE e.muscle_main IS NOT NULL AND {filter}
         GROUP BY 1 ORDER BY 2 DESC"
    ))?;
    let rows = stmt.query_map([key], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let counts = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(counts)
}

pub fn set_exercise_order(day_id: i64, ids: &[i64]) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    for (index, id) in ids.iter().enumerate() {
        tx.execute(
            "UPDATE plan_exercise SET order_index = ?1 WHERE id = ?2 AND plan_day_id = ?3",
            params![i64::try_from(index).unwrap_or(i64::MAX), id, day_id],
        )?;
    }
    tx.commit()?;
    drop(conn);
    Ok(())
}

pub fn remove_exercise(plan_exercise_id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let day_id = conn
        .query_row(
            "SELECT plan_day_id FROM plan_exercise WHERE id = ?1",
            [plan_exercise_id],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    conn.execute(
        "DELETE FROM plan_exercise WHERE id = ?1",
        [plan_exercise_id],
    )?;
    if let Some(day_id) = day_id {
        clear_singleton_supersets(&conn, day_id)?;
    }
    drop(conn);
    Ok(())
}

pub fn set_day_supersets(day_id: i64, groups: &[Vec<i64>]) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE plan_exercise SET superset_group = NULL WHERE plan_day_id = ?1",
        [day_id],
    )?;
    let mut number = 1i64;
    for group in groups {
        if group.len() < 2 {
            continue;
        }
        for id in group {
            tx.execute(
                "UPDATE plan_exercise SET superset_group = ?1 WHERE id = ?2 AND plan_day_id = ?3",
                params![number, id, day_id],
            )?;
        }
        number = number.saturating_add(1);
    }
    tx.commit()?;
    drop(conn);
    Ok(())
}

fn clear_singleton_supersets(conn: &Connection, day_id: i64) -> Result<()> {
    conn.execute(
        "UPDATE plan_exercise SET superset_group = NULL
         WHERE plan_day_id = ?1 AND superset_group IN (
             SELECT superset_group FROM plan_exercise
             WHERE plan_day_id = ?1 AND superset_group IS NOT NULL
             GROUP BY superset_group HAVING COUNT(*) < 2)",
        [day_id],
    )?;
    Ok(())
}

pub fn set_rest(plan_exercise_id: i64, seconds: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE plan_exercise SET rest_seconds = ?1 WHERE id = ?2",
        params![seconds.max(0), plan_exercise_id],
    )?;
    drop(conn);
    Ok(())
}

pub fn add_set(plan_exercise_id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let copied = conn.execute(
        "INSERT INTO plan_set
             (plan_exercise_id, set_number, target_weight, target_reps, target_seconds, target_distance_m)
         SELECT plan_exercise_id, set_number + 1, target_weight, target_reps, target_seconds, target_distance_m
         FROM plan_set WHERE plan_exercise_id = ?1 ORDER BY set_number DESC LIMIT 1",
        [plan_exercise_id],
    )?;
    if copied == 0 {
        conn.execute(
            "INSERT INTO plan_set (plan_exercise_id, set_number) VALUES (?1, 1)",
            [plan_exercise_id],
        )?;
    }
    drop(conn);
    Ok(())
}

pub fn remove_set(plan_exercise_id: i64, set_number: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let tx = conn.transaction()?;
    let deleted = tx.execute(
        "DELETE FROM plan_set WHERE plan_exercise_id = ?1 AND set_number = ?2",
        params![plan_exercise_id, set_number],
    )?;
    // The UNIQUE(plan_exercise_id, set_number) index rejects a plain set_number - 1 sweep, so shift the
    // survivors out of range first. Capping a plan at 1000 sets is not a real limit.
    if deleted == 1 {
        tx.execute(
            "UPDATE plan_set SET set_number = set_number + 1000 WHERE plan_exercise_id = ?1",
            [plan_exercise_id],
        )?;
        tx.execute(
            "UPDATE plan_set SET set_number = set_number - 1000
             WHERE plan_exercise_id = ?1 AND set_number <= ?2",
            params![plan_exercise_id, set_number.saturating_add(1000)],
        )?;
        tx.execute(
            "UPDATE plan_set SET set_number = set_number - 1001
             WHERE plan_exercise_id = ?1 AND set_number > ?2",
            params![plan_exercise_id, set_number.saturating_add(1000)],
        )?;
    }
    tx.commit()?;
    drop(conn);
    Ok(())
}

pub fn update_set(
    plan_exercise_id: i64,
    set_number: i64,
    weight: Option<f64>,
    reps: Option<i64>,
    seconds: Option<i64>,
    distance_m: Option<f64>,
) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE plan_set
         SET target_weight = ?3, target_reps = ?4, target_seconds = ?5, target_distance_m = ?6
         WHERE plan_exercise_id = ?1 AND set_number = ?2",
        params![
            plan_exercise_id,
            set_number,
            weight,
            reps,
            seconds,
            distance_m
        ],
    )?;
    drop(conn);
    Ok(())
}

#[derive(Clone, PartialEq, Eq)]
pub struct ActiveSession {
    pub id: i64,
    pub routine_id: Option<i64>,
    pub routine_day_id: Option<i64>,
    pub name: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub rest_until: Option<i64>,
}

#[derive(Clone, PartialEq)]
pub struct SessionSet {
    pub set_number: i64,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub seconds: Option<i64>,
    pub distance_m: Option<f64>,
}

#[derive(Clone, PartialEq)]
pub struct HistorySet {
    pub set_number: i64,
    pub weight: Option<f64>,
    pub reps: Option<i64>,
    pub seconds: Option<i64>,
    pub distance_m: Option<f64>,
}

#[derive(Clone, PartialEq)]
pub struct HistorySession {
    pub started_at: i64,
    pub sets: Vec<HistorySet>,
}

#[derive(Clone, PartialEq)]
pub struct SessionExercise {
    pub id: i64,
    pub exercise_id: i64,
    pub name: String,
    pub muscle_main: Option<String>,
    pub log_type: String,
    pub rest_seconds: i64,
    pub superset: Option<i64>,
    pub best_1rm: Option<f64>,
    pub prior_1rm: Option<f64>,
    pub targets: Vec<RoutineSet>,
    pub logged: Vec<SessionSet>,
}

fn session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActiveSession> {
    Ok(ActiveSession {
        id: row.get(0)?,
        routine_id: row.get(1)?,
        routine_day_id: row.get(2)?,
        name: row.get(3)?,
        started_at: row.get(4)?,
        ended_at: row.get(5)?,
        rest_until: row.get(6)?,
    })
}

pub fn active_session() -> Result<Option<ActiveSession>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let session = conn
        .query_row(
            "SELECT id, plan_id, plan_day_id, COALESCE(name, ''), started_at, NULL, rest_until
             FROM session WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
            [],
            session_row,
        )
        .optional()?;
    drop(conn);
    Ok(session)
}

pub fn finished_sessions() -> Result<Vec<ActiveSession>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT id, plan_id, plan_day_id, COALESCE(name, ''), started_at, ended_at, rest_until
         FROM session WHERE ended_at IS NOT NULL ORDER BY started_at DESC",
    )?;
    let rows = stmt.query_map([], session_row)?;
    let sessions = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(sessions)
}

pub fn volume_between(start: i64, end: i64) -> Result<f64> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let volume = conn
        .query_row(
            "SELECT SUM(s.weight * s.reps)
             FROM session_set s
             JOIN session_exercise se ON se.id = s.session_exercise_id
             JOIN session sn ON sn.id = se.session_id
             WHERE s.weight IS NOT NULL AND s.reps IS NOT NULL
               AND sn.started_at >= ?1 AND sn.started_at < ?2",
            params![start, end],
            |row| row.get::<_, Option<f64>>(0),
        )?
        .unwrap_or(0.0);
    drop(conn);
    Ok(volume)
}

pub fn workout_seconds_between(start: i64, end: i64) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let seconds = conn.query_row(
        "SELECT COALESCE(SUM(ended_at - started_at), 0)
         FROM session
         WHERE ended_at IS NOT NULL
           AND started_at >= ?1 AND started_at < ?2
           AND ended_at - started_at <= 14400",
        params![start, end],
        |row| row.get::<_, i64>(0),
    )?;
    drop(conn);
    Ok(seconds)
}

pub fn muscle_sets_between(start: i64, end: i64) -> Result<Vec<(String, i64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT e.muscle_main, COUNT(*)
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN exercise e ON e.id = se.exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE e.muscle_main IS NOT NULL
           AND (s.weight > 0 OR s.reps > 0 OR s.seconds > 0 OR s.distance_m > 0)
           AND sn.started_at >= ?1 AND sn.started_at < ?2
         GROUP BY 1",
    )?;
    let rows = stmt.query_map(params![start, end], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let counts = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(counts)
}

pub fn best_1rm_since(since: i64) -> Result<Vec<(i64, String, f64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name = localized(&conn, "e.name");
    let mut stmt = conn.prepare(&format!(
        "SELECT se.exercise_id, {name}, MAX(s.weight * (1.0 + s.reps / 30.0))
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN exercise e ON e.id = se.exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE sn.started_at >= ?1 AND s.weight > 0 AND s.reps BETWEEN 1 AND 30
           AND e.exercise_type = 'strength'
         GROUP BY se.exercise_id
         ORDER BY 3 DESC"
    ))?;
    let rows = stmt.query_map([since], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    let lifts = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(lifts)
}

pub fn first_session_at() -> Result<Option<i64>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let first = conn.query_row("SELECT MIN(started_at) FROM session", [], |row| row.get(0))?;
    drop(conn);
    Ok(first)
}

pub fn volume_sessions_between(start: i64, end: i64) -> Result<Vec<(i64, f64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT sn.started_at, SUM(s.weight * s.reps)
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE s.weight IS NOT NULL AND s.reps IS NOT NULL
           AND sn.started_at >= ?1 AND sn.started_at < ?2
         GROUP BY sn.id",
    )?;
    let rows = stmt.query_map(params![start, end], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let volumes = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(volumes)
}

pub fn workout_sessions_between(start: i64, end: i64) -> Result<Vec<(i64, i64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT started_at, ended_at - started_at
         FROM session
         WHERE ended_at IS NOT NULL
           AND started_at >= ?1 AND started_at < ?2
           AND ended_at - started_at <= 14400",
    )?;
    let rows = stmt.query_map(params![start, end], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let sessions = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(sessions)
}

pub fn exercise_name(id: i64) -> Result<Option<String>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name_col = localized(&conn, "name");
    let name = conn
        .query_row(
            &format!("SELECT {name_col} FROM exercise WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    drop(conn);
    Ok(name)
}

pub fn exercise_history(
    exercise_id: i64,
    skip_session: i64,
    limit: i64,
) -> Result<Vec<HistorySession>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT sn.id, sn.started_at, s.set_number, s.weight, s.reps, s.seconds, s.distance_m
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE se.exercise_id = ?1 AND sn.ended_at IS NOT NULL AND sn.id <> ?2
           AND (s.weight > 0 OR s.reps > 0 OR s.seconds > 0 OR s.distance_m > 0)
           AND sn.id IN (
               SELECT sn2.id FROM session sn2
               JOIN session_exercise se2 ON se2.session_id = sn2.id
               JOIN session_set s2 ON s2.session_exercise_id = se2.id
               WHERE se2.exercise_id = ?1 AND sn2.ended_at IS NOT NULL AND sn2.id <> ?2
                 AND (s2.weight > 0 OR s2.reps > 0 OR s2.seconds > 0 OR s2.distance_m > 0)
               GROUP BY sn2.id
               ORDER BY MAX(sn2.started_at) DESC LIMIT ?3
           )
         ORDER BY sn.started_at DESC, s.set_number",
    )?;
    let rows = stmt.query_map(params![exercise_id, skip_session, limit], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            HistorySet {
                set_number: row.get(2)?,
                weight: row.get(3)?,
                reps: row.get(4)?,
                seconds: row.get(5)?,
                distance_m: row.get(6)?,
            },
        ))
    })?;
    let mut sessions: Vec<(i64, HistorySession)> = Vec::new();
    for row in rows {
        let (id, started_at, set) = row?;
        match sessions.last_mut() {
            Some((last_id, session)) if *last_id == id => session.sets.push(set),
            _ => sessions.push((
                id,
                HistorySession {
                    started_at,
                    sets: vec![set],
                },
            )),
        }
    }
    drop(stmt);
    drop(conn);
    Ok(sessions.into_iter().map(|(_, session)| session).collect())
}

pub fn exercise_1rm_sessions(exercise_id: i64, start: i64, end: i64) -> Result<Vec<(i64, f64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT sn.started_at, MAX(s.weight * (1.0 + s.reps / 30.0))
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE se.exercise_id = ?1 AND s.weight > 0 AND s.reps BETWEEN 1 AND 30
           AND sn.started_at >= ?2 AND sn.started_at < ?3
         GROUP BY sn.id ORDER BY sn.started_at",
    )?;
    let rows = stmt.query_map(params![exercise_id, start, end], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    let points = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(points)
}

pub fn exercise_volume_sessions(exercise_id: i64, start: i64, end: i64) -> Result<Vec<(i64, f64)>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let mut stmt = conn.prepare(
        "SELECT sn.started_at, SUM(s.weight * s.reps)
         FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         JOIN session sn ON sn.id = se.session_id
         WHERE se.exercise_id = ?1 AND s.weight IS NOT NULL AND s.reps IS NOT NULL
           AND sn.started_at >= ?2 AND sn.started_at < ?3
         GROUP BY sn.id ORDER BY sn.started_at",
    )?;
    let rows = stmt.query_map(params![exercise_id, start, end], |row| {
        Ok((row.get(0)?, row.get(1)?))
    })?;
    let points = rows.collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    drop(conn);
    Ok(points)
}

pub fn start_session(day_id: i64) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let mut conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(id) = active_id(&conn)? {
        return Ok(id);
    }
    let tx = conn.transaction()?;
    let (routine_id, name) = tx
        .query_row(
            "SELECT plan_id, name FROM plan_day WHERE id = ?1",
            [day_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .with_context(|| format!("plan day {day_id} does not exist"))?;
    tx.execute(
        "INSERT INTO session (plan_id, plan_day_id, name, started_at) VALUES (?1, ?2, ?3, ?4)",
        params![routine_id, day_id, name, now()],
    )?;
    let id = tx.last_insert_rowid();
    tx.execute(
        "INSERT INTO session_exercise (session_id, exercise_id, name, order_index, superset_group)
         SELECT ?1, pe.exercise_id, e.name, pe.order_index, pe.superset_group
         FROM plan_exercise pe JOIN exercise e ON e.id = pe.exercise_id
         WHERE pe.plan_day_id = ?2 ORDER BY pe.order_index",
        params![id, day_id],
    )?;
    tx.commit()?;
    drop(conn);
    Ok(id)
}

fn active_id(conn: &Connection) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM session WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn session_exercises(session: &ActiveSession) -> Result<Vec<SessionExercise>> {
    session_exercises_inner(session, false)
}

pub fn finished_session_exercises(session: &ActiveSession) -> Result<Vec<SessionExercise>> {
    session_exercises_inner(session, true)
}

fn session_exercises_inner(
    session: &ActiveSession,
    finished_only: bool,
) -> Result<Vec<SessionExercise>> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let name = localized(&conn, "e.name");
    let mut stmt = conn.prepare(&format!(
        "SELECT se.id, se.exercise_id, COALESCE({name}, se.name), e.muscle_main, e.log_type,
                COALESCE(pe.rest_seconds,
                         (SELECT rest_seconds FROM user_setting WHERE id = 1), 60),
                pe.id, se.superset_group
         FROM session_exercise se
         JOIN exercise e ON e.id = se.exercise_id
         LEFT JOIN plan_exercise pe
                ON pe.plan_day_id = ?2 AND pe.order_index = se.order_index
         WHERE se.session_id = ?1 ORDER BY se.order_index"
    ))?;
    let base = stmt
        .query_map(params![session.id, session.routine_day_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<i64>>(7)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    let mut logged_stmt = conn.prepare(
        "SELECT s.session_exercise_id, s.set_number, s.weight, s.reps, s.seconds, s.distance_m
         FROM session_set s JOIN session_exercise se ON se.id = s.session_exercise_id
         WHERE se.session_id = ?1 ORDER BY s.set_number",
    )?;
    let logged = logged_stmt
        .query_map([session.id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                SessionSet {
                    set_number: row.get(1)?,
                    weight: row.get(2)?,
                    reps: row.get(3)?,
                    seconds: row.get(4)?,
                    distance_m: row.get(5)?,
                },
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(logged_stmt);
    let mut sets_stmt = conn.prepare(
        "SELECT set_number, target_weight, target_reps, target_seconds, target_distance_m
         FROM plan_set WHERE plan_exercise_id = ?1 ORDER BY set_number",
    )?;
    let mut exercises = Vec::new();
    for (id, exercise_id, name, muscle_main, log_type, rest_seconds, plan_exercise, superset) in
        base
    {
        let targets = match plan_exercise {
            Some(plan_exercise) => sets_stmt
                .query_map([plan_exercise], |row| {
                    Ok(RoutineSet {
                        set_number: row.get(0)?,
                        target_weight: row.get(1)?,
                        target_reps: row.get(2)?,
                        target_seconds: row.get(3)?,
                        target_distance_m: row.get(4)?,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        exercises.push(SessionExercise {
            id,
            exercise_id,
            name,
            muscle_main,
            log_type,
            rest_seconds,
            superset,
            best_1rm: best_1rm_with(&conn, exercise_id, None, finished_only)?,
            prior_1rm: best_1rm_with(&conn, exercise_id, Some(session.id), finished_only)?,
            targets,
            logged: logged
                .iter()
                .filter(|(owner, _)| *owner == id)
                .map(|(_, set)| set.clone())
                .collect(),
        });
    }
    drop(sets_stmt);
    drop(conn);
    Ok(exercises)
}

pub fn current_position(rows: &[SessionExercise]) -> usize {
    let position = rows.iter().position(|row| {
        if row.targets.is_empty() {
            row.logged.is_empty()
        } else {
            row.logged.len() < row.targets.len()
        }
    });
    position.unwrap_or_else(|| rows.len().saturating_sub(1))
}

pub fn session_set_count(session_id: i64) -> Result<i64> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    let count = conn.query_row(
        "SELECT COUNT(*) FROM session_set s
         JOIN session_exercise se ON se.id = s.session_exercise_id
         WHERE se.session_id = ?1",
        [session_id],
        |row| row.get(0),
    )?;
    drop(conn);
    Ok(count)
}

pub fn log_set(
    session_exercise_id: i64,
    set_number: i64,
    weight: Option<f64>,
    reps: Option<i64>,
    seconds: Option<i64>,
    distance_m: Option<f64>,
) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "INSERT INTO session_set
             (session_exercise_id, set_number, weight, reps, seconds, distance_m, completed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(session_exercise_id, set_number) DO UPDATE SET
             weight = excluded.weight, reps = excluded.reps,
             seconds = excluded.seconds, distance_m = excluded.distance_m",
        params![
            session_exercise_id,
            set_number,
            weight,
            reps,
            seconds,
            distance_m,
            now()
        ],
    )?;
    drop(conn);
    Ok(())
}

pub fn finish_session(session_id: i64) -> Result<()> {
    if session_set_count(session_id)? == 0 {
        return discard_session(session_id);
    }
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE session SET ended_at = ?2, rest_until = NULL WHERE id = ?1 AND ended_at IS NULL",
        params![session_id, now()],
    )?;
    drop(conn);
    Ok(())
}

pub fn set_rest_until(session_id: i64, deadline: Option<i64>) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute(
        "UPDATE session SET rest_until = ?2 WHERE id = ?1 AND ended_at IS NULL",
        params![session_id, deadline],
    )?;
    drop(conn);
    Ok(())
}

pub fn discard_session(session_id: i64) -> Result<()> {
    let db = DB.get().context("database not initialised")?;
    let conn = db.lock().unwrap_or_else(PoisonError::into_inner);
    conn.execute("DELETE FROM session WHERE id = ?1", [session_id])?;
    drop(conn);
    Ok(())
}

fn best_1rm_with(
    conn: &Connection,
    exercise_id: i64,
    skip_session: Option<i64>,
    finished_only: bool,
) -> Result<Option<f64>> {
    Ok(conn
        .query_row(
            "SELECT MAX(s.weight * (1.0 + s.reps / 30.0))
             FROM session_set s
             JOIN session_exercise se ON se.id = s.session_exercise_id
             JOIN session sn ON sn.id = se.session_id
             WHERE se.exercise_id = ?1 AND s.weight > 0 AND s.reps BETWEEN 1 AND 30
               AND (?2 IS NULL OR se.session_id <> ?2)
               AND (?3 = 0 OR sn.ended_at IS NOT NULL)",
            params![
                exercise_id,
                skip_session,
                i64::from(u8::from(finished_only))
            ],
            |row| row.get(0),
        )
        .optional()?
        .flatten())
}

#[cfg(target_os = "android")]
fn app_data_dir() -> Result<PathBuf> {
    use jni::{objects::JString, JavaVM};

    let ctx = ndk_context::android_context();
    // SAFETY: tao initialises ndk-context with the live JVM and application Context before launch.
    let vm = unsafe { JavaVM::from_raw(ctx.vm().cast()) }?;
    let mut env = vm.attach_current_thread()?;
    // SAFETY: same source as above; the reference is not kept past this function.
    let context = unsafe { jni::objects::JObject::from_raw(ctx.context().cast()) };
    let file = env
        .call_method(&context, "getFilesDir", "()Ljava/io/File;", &[])?
        .l()?;
    let raw = env
        .call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])?
        .l()?;
    let path = JString::from(raw);
    let dir = env.get_string(&path).map(String::from)?;
    Ok(PathBuf::from(dir))
}

#[cfg(not(target_os = "android"))]
fn app_data_dir() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .context("neither XDG_DATA_HOME nor HOME is set")?;
    Ok(base.join("je-feet"))
}
