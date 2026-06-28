use serde::{Deserialize, Serialize};

use crate::world::map::Direction;

pub const B_PAD_RIGHT: u8 = 1 << 0;
pub const B_PAD_LEFT: u8 = 1 << 1;
pub const B_PAD_UP: u8 = 1 << 2;
pub const B_PAD_DOWN: u8 = 1 << 3;
pub const B_PAD_A: u8 = 1 << 4;
pub const B_PAD_B: u8 = 1 << 5;
pub const B_PAD_SELECT: u8 = 1 << 6;
pub const B_PAD_START: u8 = 1 << 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GameButton {
    A,
    B,
    Start,
    Select,
    Right,
    Left,
    Up,
    Down,
}

impl GameButton {
    pub const fn pad_bit(self) -> u8 {
        match self {
            Self::Right => B_PAD_RIGHT,
            Self::Left => B_PAD_LEFT,
            Self::Up => B_PAD_UP,
            Self::Down => B_PAD_DOWN,
            Self::A => B_PAD_A,
            Self::B => B_PAD_B,
            Self::Select => B_PAD_SELECT,
            Self::Start => B_PAD_START,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct JoypadState {
    previous_mask: u8,
}

impl JoypadState {
    pub const fn new() -> Self {
        Self { previous_mask: 0 }
    }

    pub const fn from_previous_mask(previous_mask: u8) -> Self {
        Self { previous_mask }
    }

    pub const fn previous_mask(self) -> u8 {
        self.previous_mask
    }

    pub fn compute_mask(buttons: impl IntoIterator<Item = GameButton>) -> u8 {
        buttons
            .into_iter()
            .fold(0, |mask, button| mask | button.pad_bit())
    }

    pub fn update(
        &mut self,
        buttons: impl IntoIterator<Item = GameButton>,
        filter_mask: u8,
    ) -> JoypadUpdate {
        let current_mask = Self::compute_mask(buttons);
        let filtered_mask = current_mask & filter_mask;
        let h_joy_pressed = (filtered_mask ^ self.previous_mask) & filtered_mask;
        self.previous_mask = filtered_mask;
        JoypadUpdate {
            h_joy_pressed,
            h_joy_down: filtered_mask,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoypadUpdate {
    pub h_joy_pressed: u8,
    pub h_joy_down: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[serde(deny_unknown_fields)]
pub enum JoypadDirectionError {
    #[error("joypad mask {mask:#010b} presses multiple directions")]
    ConflictingDirections { mask: u8 },
}

pub fn direction_from_pad_mask(mask: u8) -> Result<Option<Direction>, JoypadDirectionError> {
    let directions = [
        (B_PAD_DOWN, Direction::Down),
        (B_PAD_UP, Direction::Up),
        (B_PAD_LEFT, Direction::Left),
        (B_PAD_RIGHT, Direction::Right),
    ];
    let mut pressed = directions
        .into_iter()
        .filter(|(bit, _)| mask & *bit != 0)
        .map(|(_, direction)| direction);
    let first = pressed.next();
    if pressed.next().is_some() {
        return Err(JoypadDirectionError::ConflictingDirections { mask });
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_bits_match_hardware_layout() {
        assert_eq!(GameButton::Right.pad_bit(), 0b0000_0001);
        assert_eq!(GameButton::Left.pad_bit(), 0b0000_0010);
        assert_eq!(GameButton::Up.pad_bit(), 0b0000_0100);
        assert_eq!(GameButton::Down.pad_bit(), 0b0000_1000);
        assert_eq!(GameButton::A.pad_bit(), 0b0001_0000);
        assert_eq!(GameButton::B.pad_bit(), 0b0010_0000);
        assert_eq!(GameButton::Select.pad_bit(), 0b0100_0000);
        assert_eq!(GameButton::Start.pad_bit(), 0b1000_0000);
    }

    #[test]
    fn joypad_update_reports_new_presses_only() {
        let mut joypad = JoypadState::new();
        let first = joypad.update([GameButton::A, GameButton::Right], 0xff);
        assert_eq!(
            first,
            JoypadUpdate {
                h_joy_pressed: B_PAD_A | B_PAD_RIGHT,
                h_joy_down: B_PAD_A | B_PAD_RIGHT
            }
        );

        let held = joypad.update([GameButton::A, GameButton::Right], 0xff);
        assert_eq!(held.h_joy_pressed, 0);
        assert_eq!(held.h_joy_down, B_PAD_A | B_PAD_RIGHT);

        let added = joypad.update([GameButton::A, GameButton::B], 0xff);
        assert_eq!(added.h_joy_pressed, B_PAD_B);
        assert_eq!(added.h_joy_down, B_PAD_A | B_PAD_B);
    }

    #[test]
    fn joypad_filter_masks_ineligible_buttons() {
        let mut joypad = JoypadState::new();
        let update = joypad.update([GameButton::A, GameButton::Start], B_PAD_A);
        assert_eq!(update.h_joy_pressed, B_PAD_A);
        assert_eq!(update.h_joy_down, B_PAD_A);
    }

    #[test]
    fn joypad_can_resume_previous_down_mask_for_save_load_edges() {
        let mut joypad = JoypadState::from_previous_mask(B_PAD_RIGHT);

        let held = joypad.update([GameButton::Right], 0xff);
        assert_eq!(held.h_joy_pressed, 0);
        assert_eq!(held.h_joy_down, B_PAD_RIGHT);

        let changed = joypad.update([GameButton::A, GameButton::Right], 0xff);
        assert_eq!(changed.h_joy_pressed, B_PAD_A);
        assert_eq!(changed.h_joy_down, B_PAD_A | B_PAD_RIGHT);
    }

    #[test]
    fn game_button_json_rejects_legacy_alias_payloads() {
        let error = serde_json::from_value::<GameButton>(serde_json::json!({
            "a": {
                "legacy_button": "A_BUTTON"
            }
        }))
        .expect_err("buttons must not accept legacy object payloads")
        .to_string();
        assert!(
            error.contains("invalid type") || error.contains("unknown variant"),
            "{error}"
        );
    }
}
