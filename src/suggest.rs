use crate::db::{HistorySession, RoutineSet};

const INCREMENT_KG: f64 = 2.5;
const JUMP_RATIO: f64 = 0.10;
const BRIDGE_REPS: i64 = 4;
const DELOAD_RATIO: f64 = 0.6;
const MUSCLE_SETS_MIN: i64 = 10;
const MUSCLE_SETS_MAX: i64 = 20;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub week: i64,
    pub weeks: i64,
    pub pass_start: i64,
}

impl Block {
    #[must_use]
    pub const fn deload(self) -> bool {
        self.week > self.weeks
    }

    #[must_use]
    pub const fn rir(self) -> i64 {
        let rir = self.weeks.saturating_add(1).saturating_sub(self.week);
        if rir < 1 {
            1
        } else if rir > 4 {
            4
        } else {
            rir
        }
    }

    #[must_use]
    pub const fn muscle_sets(self) -> i64 {
        match self
            .week
            .saturating_sub(1)
            .saturating_mul(MUSCLE_SETS_MAX - MUSCLE_SETS_MIN)
            .checked_div(self.weeks.saturating_sub(1))
        {
            Some(step) => MUSCLE_SETS_MIN.saturating_add(step),
            None => MUSCLE_SETS_MIN,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Goal {
    Cutting,
    Maintaining,
    Bulking,
    BuildingStrength,
}

impl Goal {
    #[must_use]
    pub fn from_routine(goal: Option<&str>) -> Self {
        match goal {
            Some("cutting") => Self::Cutting,
            Some("bulking") => Self::Bulking,
            Some("building_strength") => Self::BuildingStrength,
            _ => Self::Maintaining,
        }
    }

    const fn clean_sessions(self) -> usize {
        match self {
            Self::Cutting => 2,
            Self::Maintaining | Self::Bulking | Self::BuildingStrength => 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Target {
    pub weight: Option<f64>,
    pub reps: Option<i64>,
}

#[must_use]
pub fn targets(
    numbers: &[i64],
    targets: &[RoutineSet],
    history: &[HistorySession],
    goal: Goal,
    block: Option<Block>,
) -> Vec<Option<Target>> {
    let Some(reference) = history.first() else {
        return numbers.iter().map(|_| None).collect();
    };
    if block.is_some_and(Block::deload) {
        return numbers
            .iter()
            .map(|number| {
                let reps = planned(targets, *number)?.target_reps?;
                let last = reference
                    .sets
                    .iter()
                    .find(|set| set.set_number == *number)
                    .or_else(|| reference.sets.last())?;
                Some(Target {
                    weight: last
                        .weight
                        .map(|weight| (weight * DELOAD_RATIO * 100.0).round() / 100.0),
                    reps: Some(reps),
                })
            })
            .collect();
    }
    let streak = history
        .iter()
        .take_while(|session| clean(session, targets))
        .count();
    let earned = streak >= goal.clean_sessions();
    let top = reference
        .sets
        .iter()
        .filter_map(|set| set.weight)
        .reduce(f64::max);
    let bridged = targets.iter().all(|target| {
        let Some(reps) = target.target_reps else {
            return true;
        };
        reference.sets.iter().any(|set| {
            set.set_number == target.set_number
                && set
                    .reps
                    .is_some_and(|done| done >= reps.saturating_add(BRIDGE_REPS))
        })
    });
    let jump = top.is_some_and(|top| top > 0.0 && (INCREMENT_KG <= top * JUMP_RATIO || bridged));

    numbers
        .iter()
        .map(|number| {
            let target = planned(targets, *number)?.target_reps?;
            let last = reference
                .sets
                .iter()
                .find(|set| set.set_number == *number)
                .or_else(|| reference.sets.last())?;
            let reps = last.reps?;
            if earned && jump {
                return Some(Target {
                    weight: last.weight.map(|weight| weight + INCREMENT_KG),
                    reps: Some(target),
                });
            }
            let cap = if earned {
                target.saturating_add(BRIDGE_REPS)
            } else {
                target
            };
            Some(Target {
                weight: last.weight,
                reps: Some(reps.saturating_add(1).min(cap).max(reps)),
            })
        })
        .collect()
}

fn planned(targets: &[RoutineSet], number: i64) -> Option<&RoutineSet> {
    targets
        .iter()
        .find(|set| set.set_number == number)
        .or_else(|| targets.last())
}

fn clean(session: &HistorySession, targets: &[RoutineSet]) -> bool {
    !targets.is_empty()
        && targets.iter().all(|target| {
            let Some(reps) = target.target_reps else {
                return true;
            };
            session.sets.iter().any(|set| {
                set.set_number == target.set_number && set.reps.is_some_and(|done| done >= reps)
            })
        })
}
