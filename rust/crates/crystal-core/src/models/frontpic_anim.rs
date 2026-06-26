use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const FRONTPIC_ANIM_FRAME_COMMAND: &str = "frame";
pub const FRONTPIC_ANIM_SET_REPEAT_COMMAND: &str = "setrepeat";
pub const FRONTPIC_ANIM_DO_REPEAT_COMMAND: &str = "dorepeat";
pub const FRONTPIC_ANIM_END_COMMAND: &str = "endanim";
pub const FRONTPIC_ANIM_COMMANDS: &[&str] = &[
    FRONTPIC_ANIM_FRAME_COMMAND,
    FRONTPIC_ANIM_SET_REPEAT_COMMAND,
    FRONTPIC_ANIM_DO_REPEAT_COMMAND,
    FRONTPIC_ANIM_END_COMMAND,
];

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontpicAnimCommand {
    pub kind: String,
    pub frame: Option<u16>,
    pub duration: Option<u16>,
    pub count: Option<u16>,
    pub target: Option<u16>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontpicAnimProgram {
    pub commands: Vec<FrontpicAnimCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontpicAnimCommandIssue {
    MissingFrame,
    MissingSetRepeatCount,
    MissingDoRepeatTarget,
    UnknownCommand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontpicAnimCatalogIssue {
    InvalidSpeciesId {
        species_id: String,
    },
    UnknownSpecies {
        species_id: String,
    },
    EmptyProgram {
        species_id: String,
    },
    Command {
        species_id: String,
        index: usize,
        command: String,
        issue: FrontpicAnimCommandIssue,
    },
    MissingSpeciesProgram {
        species_id: String,
    },
}

pub fn is_known_frontpic_anim_command(kind: &str) -> bool {
    FRONTPIC_ANIM_COMMANDS.contains(&kind)
}

pub fn frontpic_anim_command_issue(
    command: &FrontpicAnimCommand,
) -> Option<FrontpicAnimCommandIssue> {
    match command.kind.as_str() {
        FRONTPIC_ANIM_FRAME_COMMAND => {
            if command.frame.is_none() || command.duration.is_none() {
                Some(FrontpicAnimCommandIssue::MissingFrame)
            } else {
                None
            }
        }
        FRONTPIC_ANIM_SET_REPEAT_COMMAND => {
            if command.count.is_none() {
                Some(FrontpicAnimCommandIssue::MissingSetRepeatCount)
            } else {
                None
            }
        }
        FRONTPIC_ANIM_DO_REPEAT_COMMAND => {
            if command.target.is_none() {
                Some(FrontpicAnimCommandIssue::MissingDoRepeatTarget)
            } else {
                None
            }
        }
        FRONTPIC_ANIM_END_COMMAND => None,
        _ => Some(FrontpicAnimCommandIssue::UnknownCommand),
    }
}

pub fn frontpic_anim_catalog_issues(
    programs: &BTreeMap<String, FrontpicAnimProgram>,
    species_ids: &BTreeSet<String>,
) -> Vec<FrontpicAnimCatalogIssue> {
    let mut issues = Vec::new();
    for (species_id, program) in programs {
        if !is_exact_nonempty_frontpic_token(species_id) {
            issues.push(FrontpicAnimCatalogIssue::InvalidSpeciesId {
                species_id: species_id.clone(),
            });
        } else if !is_frontpic_animation_asset_key(species_id, species_ids) {
            issues.push(FrontpicAnimCatalogIssue::UnknownSpecies {
                species_id: species_id.clone(),
            });
        }
        if program.commands.is_empty() {
            issues.push(FrontpicAnimCatalogIssue::EmptyProgram {
                species_id: species_id.clone(),
            });
        }
        for (index, command) in program.commands.iter().enumerate() {
            if let Some(issue) = frontpic_anim_command_issue(command) {
                issues.push(FrontpicAnimCatalogIssue::Command {
                    species_id: species_id.clone(),
                    index,
                    command: command.kind.clone(),
                    issue,
                });
            }
        }
    }
    for species_id in species_ids {
        if !programs.contains_key(species_id) {
            issues.push(FrontpicAnimCatalogIssue::MissingSpeciesProgram {
                species_id: species_id.clone(),
            });
        }
    }
    issues
}

fn is_exact_nonempty_frontpic_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn is_frontpic_animation_asset_key(species_id: &str, species_ids: &BTreeSet<String>) -> bool {
    species_ids.contains(species_id)
        || species_id == "EGG"
        || species_id
            .strip_prefix("UNOWN_")
            .and_then(|suffix| {
                suffix
                    .as_bytes()
                    .first()
                    .copied()
                    .filter(|_| suffix.len() == 1)
            })
            .is_some_and(|byte| byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontpic_anim_command_set_is_exact() {
        assert_eq!(
            FRONTPIC_ANIM_COMMANDS,
            &[
                FRONTPIC_ANIM_FRAME_COMMAND,
                FRONTPIC_ANIM_SET_REPEAT_COMMAND,
                FRONTPIC_ANIM_DO_REPEAT_COMMAND,
                FRONTPIC_ANIM_END_COMMAND
            ]
        );
        assert!(is_known_frontpic_anim_command("frame"));
        assert!(!is_known_frontpic_anim_command("FRAME"));
    }

    #[test]
    fn frontpic_anim_command_shape_is_exact_without_fallbacks() {
        let frame = FrontpicAnimCommand {
            kind: FRONTPIC_ANIM_FRAME_COMMAND.to_string(),
            frame: Some(0),
            duration: Some(8),
            ..FrontpicAnimCommand::default()
        };
        assert_eq!(frontpic_anim_command_issue(&frame), None);

        assert_eq!(
            frontpic_anim_command_issue(&FrontpicAnimCommand {
                kind: FRONTPIC_ANIM_FRAME_COMMAND.to_string(),
                frame: Some(0),
                ..FrontpicAnimCommand::default()
            }),
            Some(FrontpicAnimCommandIssue::MissingFrame)
        );
        assert_eq!(
            frontpic_anim_command_issue(&FrontpicAnimCommand {
                kind: FRONTPIC_ANIM_SET_REPEAT_COMMAND.to_string(),
                ..FrontpicAnimCommand::default()
            }),
            Some(FrontpicAnimCommandIssue::MissingSetRepeatCount)
        );
        assert_eq!(
            frontpic_anim_command_issue(&FrontpicAnimCommand {
                kind: FRONTPIC_ANIM_DO_REPEAT_COMMAND.to_string(),
                ..FrontpicAnimCommand::default()
            }),
            Some(FrontpicAnimCommandIssue::MissingDoRepeatTarget)
        );
        assert_eq!(
            frontpic_anim_command_issue(&FrontpicAnimCommand {
                kind: "FRAME".to_string(),
                ..FrontpicAnimCommand::default()
            }),
            Some(FrontpicAnimCommandIssue::UnknownCommand)
        );
    }

    #[test]
    fn frontpic_anim_catalog_issues_validate_exact_asset_keys_and_programs() {
        let species_ids = BTreeSet::from(["CHIKORITA".to_string(), "BAYLEEF".to_string()]);
        let programs = BTreeMap::from([
            (
                " BAYLEEF".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: FRONTPIC_ANIM_END_COMMAND.to_string(),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            ),
            (
                "CHIKORITA".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: FRONTPIC_ANIM_FRAME_COMMAND.to_string(),
                        frame: Some(0),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            ),
            (
                "chikorita".to_string(),
                FrontpicAnimProgram {
                    commands: Vec::new(),
                },
            ),
            (
                "EGG".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: FRONTPIC_ANIM_END_COMMAND.to_string(),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            ),
            (
                "UNOWN_A".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: FRONTPIC_ANIM_END_COMMAND.to_string(),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            ),
            (
                "UNOWN_aa".to_string(),
                FrontpicAnimProgram {
                    commands: vec![FrontpicAnimCommand {
                        kind: "ENDANIM".to_string(),
                        ..FrontpicAnimCommand::default()
                    }],
                },
            ),
        ]);

        assert_eq!(
            frontpic_anim_catalog_issues(&programs, &species_ids),
            vec![
                FrontpicAnimCatalogIssue::InvalidSpeciesId {
                    species_id: " BAYLEEF".to_string(),
                },
                FrontpicAnimCatalogIssue::Command {
                    species_id: "CHIKORITA".to_string(),
                    index: 0,
                    command: FRONTPIC_ANIM_FRAME_COMMAND.to_string(),
                    issue: FrontpicAnimCommandIssue::MissingFrame,
                },
                FrontpicAnimCatalogIssue::UnknownSpecies {
                    species_id: "UNOWN_aa".to_string(),
                },
                FrontpicAnimCatalogIssue::Command {
                    species_id: "UNOWN_aa".to_string(),
                    index: 0,
                    command: "ENDANIM".to_string(),
                    issue: FrontpicAnimCommandIssue::UnknownCommand,
                },
                FrontpicAnimCatalogIssue::UnknownSpecies {
                    species_id: "chikorita".to_string(),
                },
                FrontpicAnimCatalogIssue::EmptyProgram {
                    species_id: "chikorita".to_string(),
                },
                FrontpicAnimCatalogIssue::MissingSpeciesProgram {
                    species_id: "BAYLEEF".to_string(),
                },
            ]
        );
    }

    #[test]
    fn frontpic_anim_json_requires_explicit_program_and_command_kind() {
        let missing_commands = serde_json::from_str::<FrontpicAnimProgram>(r#"{}"#)
            .expect_err("frontpic animation programs must declare command lists")
            .to_string();
        assert!(
            missing_commands.contains("missing field `commands`"),
            "{missing_commands}"
        );

        let missing_kind =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"frame":0}]}"#)
                .expect_err("frontpic animation commands must declare their opcode kind")
                .to_string();
        assert!(
            missing_kind.contains("missing field `kind`"),
            "{missing_kind}"
        );

        let explicit_command =
            serde_json::from_str::<FrontpicAnimProgram>(r#"{"commands":[{"kind":"endanim"}]}"#)
                .expect("optional command operands may be absent when opcode does not use them");
        assert_eq!(explicit_command.commands[0].kind, FRONTPIC_ANIM_END_COMMAND);

        let unknown_program_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim"}],"fallback":[]}"#,
        )
        .expect_err("frontpic animation programs must not accept unknown fields")
        .to_string();
        assert!(
            unknown_program_field.contains("unknown field `fallback`"),
            "{unknown_program_field}"
        );

        let unknown_command_field = serde_json::from_str::<FrontpicAnimProgram>(
            r#"{"commands":[{"kind":"endanim","legacyOpcode":"end"}]}"#,
        )
        .expect_err("frontpic animation commands must not accept unknown fields")
        .to_string();
        assert!(
            unknown_command_field.contains("unknown field `legacyOpcode`"),
            "{unknown_command_field}"
        );
    }
}
