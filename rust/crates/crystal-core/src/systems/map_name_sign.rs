use crate::state::MapNameSignMemory;

pub const SHOWN_MAP_NAME_SIGN_MASK: u8 = 1 << 1;
pub const MAP_NAME_SIGN_FRAMES: u8 = 60;
pub const GATE_LANDMARK: u8 = 0xff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapNameSignOutcome {
    Hidden { landmark: u8 },
    Shown { landmark: u8, frames: u8 },
}

/// Execute Crystal's `InitMapNameSign.inefficient_farcall` state transition.
/// Map metadata supplies the source-derived landmark byte and the exact set of
/// landmarks suppressed by `.CheckSpecialMap`.
pub fn init_map_name_sign(
    memory: &mut MapNameSignMemory,
    map_landmark: u8,
    gate_environment: bool,
    national_park_gate: bool,
    special_landmark: u8,
    suppressed_landmarks: &[u8],
) -> MapNameSignOutcome {
    let current = if gate_environment || national_park_gate {
        GATE_LANDMARK
    } else {
        map_landmark
    };
    memory.current_landmark = current;

    let was_forced_hidden = memory.flags & SHOWN_MAP_NAME_SIGN_MASK != 0;
    memory.flags &= !SHOWN_MAP_NAME_SIGN_MASK;
    let moving_within_landmark =
        memory.previous_landmark == current || memory.previous_landmark == special_landmark;
    memory.previous_landmark = current;

    if was_forced_hidden
        || moving_within_landmark
        || current == GATE_LANDMARK
        || current == special_landmark
        || suppressed_landmarks.contains(&current)
    {
        return MapNameSignOutcome::Hidden { landmark: current };
    }

    memory.timer = MAP_NAME_SIGN_FRAMES;
    MapNameSignOutcome::Shown {
        landmark: current,
        frames: MAP_NAME_SIGN_FRAMES,
    }
}

pub fn force_hide_next_map_name_sign(memory: &mut MapNameSignMemory) {
    memory.flags |= SHOWN_MAP_NAME_SIGN_MASK;
}

/// Execute the WRAM timer portion of one `PlaceMapNameSign` overworld pass.
/// The returned byte is A's pre-decrement value, which owns the source's
/// 60-frame setup hold and 59-frame text initialization branches.
pub fn place_map_name_sign(memory: &mut MapNameSignMemory) -> u8 {
    let previous = memory.timer;
    if previous != 0 {
        memory.timer = memory.timer.wrapping_sub(1);
    }
    previous
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_map_name_sign_matches_previous_special_gate_and_suppression_branches() {
        let mut memory = MapNameSignMemory {
            previous_landmark: 1,
            ..MapNameSignMemory::default()
        };
        assert_eq!(
            init_map_name_sign(&mut memory, 2, false, false, 0, &[7, 8, 9, 10, 11]),
            MapNameSignOutcome::Shown {
                landmark: 2,
                frames: 60,
            }
        );
        assert_eq!(memory.timer, 60);
        assert_eq!(
            init_map_name_sign(&mut memory, 2, false, false, 0, &[]),
            MapNameSignOutcome::Hidden { landmark: 2 }
        );

        memory.previous_landmark = 0;
        assert_eq!(
            init_map_name_sign(&mut memory, 3, false, false, 0, &[]),
            MapNameSignOutcome::Hidden { landmark: 3 }
        );
        memory.previous_landmark = 3;
        assert_eq!(
            init_map_name_sign(&mut memory, 7, false, false, 0, &[7]),
            MapNameSignOutcome::Hidden { landmark: 7 }
        );
        assert_eq!(
            init_map_name_sign(&mut memory, 4, true, false, 0, &[]),
            MapNameSignOutcome::Hidden { landmark: 0xff }
        );
    }

    #[test]
    fn shown_flag_is_one_shot_and_does_not_clear_an_existing_timer() {
        let mut memory = MapNameSignMemory {
            previous_landmark: 1,
            timer: 23,
            ..MapNameSignMemory::default()
        };
        force_hide_next_map_name_sign(&mut memory);
        assert_eq!(
            init_map_name_sign(&mut memory, 2, false, false, 0, &[]),
            MapNameSignOutcome::Hidden { landmark: 2 }
        );
        assert_eq!(memory.flags, 0);
        assert_eq!(memory.previous_landmark, 2);
        assert_eq!(memory.timer, 23);
    }

    #[test]
    fn place_map_name_sign_uses_the_source_predecrement_timer_value() {
        let mut memory = MapNameSignMemory {
            timer: 60,
            ..MapNameSignMemory::default()
        };
        assert_eq!(place_map_name_sign(&mut memory), 60);
        assert_eq!(memory.timer, 59);
        assert_eq!(place_map_name_sign(&mut memory), 59);
        assert_eq!(memory.timer, 58);
        for expected in (1..=58).rev() {
            assert_eq!(place_map_name_sign(&mut memory), expected);
        }
        assert_eq!(place_map_name_sign(&mut memory), 0);
        assert_eq!(memory.timer, 0);
    }
}
