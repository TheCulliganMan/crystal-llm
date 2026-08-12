use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize, de::Error as _};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleAnimationCatalogIssue {
    InvalidAnimation {
        label: String,
    },
    InvalidCommand {
        label: String,
        index: usize,
        command: String,
    },
    UnknownCommandTarget {
        label: String,
        index: usize,
        target: String,
    },
    InvalidTableEntry {
        index: usize,
    },
    UnknownTableAnimation {
        index: usize,
        label: String,
    },
    TableCountMismatch {
        actual_count: usize,
        expected_count: usize,
    },
}

pub fn battle_animation_catalog_issues(
    animations: &BTreeMap<String, Vec<String>>,
    animation_table: &[String],
    move_count: usize,
) -> Vec<BattleAnimationCatalogIssue> {
    let mut issues = Vec::new();
    let global_labels = animations
        .values()
        .flatten()
        .filter_map(|command| global_battle_animation_label(command))
        .collect::<BTreeSet<_>>();
    for (label, commands) in animations {
        if !is_exact_nonempty_battle_animation_token(label) || commands.is_empty() {
            issues.push(BattleAnimationCatalogIssue::InvalidAnimation {
                label: label.clone(),
            });
        }
        for (index, command) in commands.iter().enumerate() {
            if !is_canonical_battle_animation_command(command) {
                issues.push(BattleAnimationCatalogIssue::InvalidCommand {
                    label: label.clone(),
                    index,
                    command: command.clone(),
                });
            }
        }
        let local_labels = commands
            .iter()
            .filter_map(|command| local_battle_animation_label(command))
            .collect::<BTreeSet<_>>();
        for (index, command) in commands.iter().enumerate() {
            let Some(target) = battle_animation_command_target(command) else {
                continue;
            };
            let resolved = if target.starts_with('.') {
                local_labels.contains(target)
            } else {
                animations.contains_key(target) || global_labels.contains(target)
            };
            if !resolved {
                issues.push(BattleAnimationCatalogIssue::UnknownCommandTarget {
                    label: label.clone(),
                    index,
                    target: target.to_string(),
                });
            }
        }
    }
    for (index, label) in animation_table.iter().enumerate() {
        if !is_exact_nonempty_battle_animation_token(label) {
            issues.push(BattleAnimationCatalogIssue::InvalidTableEntry { index });
        } else if !animations.contains_key(label) {
            issues.push(BattleAnimationCatalogIssue::UnknownTableAnimation {
                index,
                label: label.clone(),
            });
        }
    }
    if !animation_table.is_empty() && animation_table.len() != move_count + 1 {
        issues.push(BattleAnimationCatalogIssue::TableCountMismatch {
            actual_count: animation_table.len(),
            expected_count: move_count + 1,
        });
    }

    issues
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct BattleAnimationCommandTable(pub BTreeMap<String, Vec<String>>);

impl<'de> Deserialize<'de> for BattleAnimationCommandTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let animations = BTreeMap::<String, Vec<String>>::deserialize(deserializer)?;
        if animations.is_empty() {
            return Err(D::Error::custom(
                "battle animation command table must not be empty",
            ));
        }
        for (label, commands) in &animations {
            if !is_exact_nonempty_battle_animation_token(label) {
                return Err(D::Error::custom(format!(
                    "battle animation label must be an exact animation token, found {label:?}"
                )));
            }
            if commands.is_empty() {
                return Err(D::Error::custom(format!(
                    "battle animation {label:?} must declare at least one command"
                )));
            }
            for command in commands {
                if !is_canonical_battle_animation_command(command) {
                    return Err(D::Error::custom(format!(
                        "battle animation command for {label:?} is not a canonical ASM command, found {command:?}"
                    )));
                }
            }
        }
        if let Some(issue) = battle_animation_catalog_issues(&animations, &[], 0)
            .into_iter()
            .find(|issue| {
                matches!(
                    issue,
                    BattleAnimationCatalogIssue::UnknownCommandTarget { .. }
                )
            })
        {
            return Err(D::Error::custom(format!(
                "battle animation command table has an unresolved ASM target: {issue:?}"
            )));
        }
        Ok(Self(animations))
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct BattleAnimationTable(pub Vec<String>);

impl<'de> Deserialize<'de> for BattleAnimationTable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let entries = Vec::<String>::deserialize(deserializer)?;
        if entries.is_empty() {
            return Err(D::Error::custom("battle animation table must not be empty"));
        }
        for (index, label) in entries.iter().enumerate() {
            if !is_exact_nonempty_battle_animation_token(label) {
                return Err(D::Error::custom(format!(
                    "battle animation table entry {index} must be an exact animation token, found {label:?}"
                )));
            }
        }
        Ok(Self(entries))
    }
}

fn is_exact_nonempty_battle_animation_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_nonempty_battle_animation_command(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !has_reserved_pack_prefix(value)
}

fn is_canonical_battle_animation_command(value: &str) -> bool {
    if !is_exact_nonempty_battle_animation_command(value) {
        return false;
    }
    if global_battle_animation_label(value).is_some()
        || local_battle_animation_label(value).is_some()
    {
        return true;
    }
    let opcode = value
        .split_once(char::is_whitespace)
        .map_or(value, |(opcode, _)| opcode);
    if CANONICAL_BATTLE_ANIMATION_OPCODES
        .binary_search(&opcode)
        .is_err()
    {
        return false;
    }
    let argument_count = value
        .split_once(char::is_whitespace)
        .map(|(_, arguments)| {
            arguments
                .split(',')
                .map(str::trim)
                .filter(|argument| !argument.is_empty())
                .count()
        })
        .unwrap_or(0);
    canonical_battle_animation_opcode_arity(opcode) == Some(argument_count)
}

fn canonical_battle_animation_opcode_arity(opcode: &str) -> Option<usize> {
    match opcode {
        "anim_battlergfx_1row"
        | "anim_battlergfx_2row"
        | "anim_beatup"
        | "anim_checkpokeball"
        | "anim_clearobjs"
        | "anim_dropsub"
        | "anim_incvar"
        | "anim_keepsprites"
        | "anim_minimize"
        | "anim_raisesub"
        | "anim_resetobp0"
        | "anim_ret"
        | "anim_transform"
        | "anim_updateactorpic" => Some(0),
        "anim_1gfx" | "anim_bgp" | "anim_call" | "anim_cry" | "anim_incbgeffect"
        | "anim_incobj" | "anim_jump" | "anim_jumpuntil" | "anim_obp0" | "anim_obp1"
        | "anim_setvar" | "anim_wait" => Some(1),
        "anim_2gfx"
        | "anim_if_param_and"
        | "anim_if_param_equal"
        | "anim_if_var_equal"
        | "anim_loop"
        | "anim_setobj" => Some(2),
        "anim_3gfx" | "anim_sound" => Some(3),
        "anim_bgeffect" | "anim_obj" => Some(4),
        _ => None,
    }
}

fn local_battle_animation_label(value: &str) -> Option<&str> {
    let label = value.strip_suffix(':').unwrap_or(value);
    (label.starts_with('.')
        && !label.contains(char::is_whitespace)
        && label
            .bytes()
            .skip(1)
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'))
    .then_some(label)
}

fn global_battle_animation_label(value: &str) -> Option<&str> {
    let label = value.strip_suffix(':')?;
    is_exact_nonempty_battle_animation_token(label).then_some(label)
}

fn battle_animation_command_target(command: &str) -> Option<&str> {
    if local_battle_animation_label(command).is_some() {
        return None;
    }
    let (opcode, raw_arguments) = command.split_once(char::is_whitespace)?;
    let arguments = raw_arguments
        .split(',')
        .map(str::trim)
        .filter(|argument| !argument.is_empty())
        .collect::<Vec<_>>();
    match opcode {
        "anim_call" | "anim_jump" | "anim_jumpuntil" => arguments.first().copied(),
        "anim_loop" | "anim_if_var_equal" | "anim_if_param_equal" | "anim_if_param_and" => {
            arguments.get(1).copied()
        }
        _ => None,
    }
}

const CANONICAL_BATTLE_ANIMATION_OPCODES: &[&str] = &[
    "anim_1gfx",
    "anim_2gfx",
    "anim_3gfx",
    "anim_battlergfx_1row",
    "anim_battlergfx_2row",
    "anim_beatup",
    "anim_bgeffect",
    "anim_bgp",
    "anim_call",
    "anim_checkpokeball",
    "anim_clearobjs",
    "anim_cry",
    "anim_dropsub",
    "anim_if_param_and",
    "anim_if_param_equal",
    "anim_if_var_equal",
    "anim_incbgeffect",
    "anim_incobj",
    "anim_incvar",
    "anim_jump",
    "anim_jumpuntil",
    "anim_keepsprites",
    "anim_loop",
    "anim_minimize",
    "anim_obj",
    "anim_obp0",
    "anim_obp1",
    "anim_raisesub",
    "anim_resetobp0",
    "anim_ret",
    "anim_setobj",
    "anim_setvar",
    "anim_sound",
    "anim_transform",
    "anim_updateactorpic",
    "anim_wait",
];

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battle_animation_catalog_issues_require_exact_nonempty_tables() {
        let animations = [
            ("".to_string(), vec!["anim_wait 1".to_string()]),
            (
                " BattleAnim_Padded".to_string(),
                vec!["anim_wait 1".to_string()],
            ),
            (
                "BattleAnim Tackle".to_string(),
                vec!["anim_wait 1".to_string()],
            ),
            ("BattleAnim_Pound".to_string(), Vec::new()),
            (
                "BattleAnim_Tackle".to_string(),
                vec!["anim_wait 1".to_string()],
            ),
        ]
        .into_iter()
        .collect();
        let animation_table = vec![
            "BattleAnim_Dummy".to_string(),
            String::new(),
            "BattleAnim_Tackle ".to_string(),
            "BattleAnim Tackle".to_string(),
        ];

        assert_eq!(
            battle_animation_catalog_issues(&animations, &animation_table, 1),
            vec![
                BattleAnimationCatalogIssue::InvalidAnimation {
                    label: String::new(),
                },
                BattleAnimationCatalogIssue::InvalidAnimation {
                    label: " BattleAnim_Padded".to_string(),
                },
                BattleAnimationCatalogIssue::InvalidAnimation {
                    label: "BattleAnim Tackle".to_string(),
                },
                BattleAnimationCatalogIssue::InvalidAnimation {
                    label: "BattleAnim_Pound".to_string(),
                },
                BattleAnimationCatalogIssue::UnknownTableAnimation {
                    index: 0,
                    label: "BattleAnim_Dummy".to_string(),
                },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 1 },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 2 },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 3 },
                BattleAnimationCatalogIssue::TableCountMismatch {
                    actual_count: 4,
                    expected_count: 2,
                },
            ],
        );
    }

    #[test]
    fn battle_animation_catalog_issues_allow_empty_table_for_partial_packs() {
        assert!(battle_animation_catalog_issues(&BTreeMap::new(), &[], 12).is_empty());
    }

    #[test]
    fn battle_animation_catalog_issues_reject_reserved_pack_prefix_tokens() {
        let animations = [(
            "fallbackBattleAnim_Tackle".to_string(),
            vec!["anim_wait 1".to_string()],
        )]
        .into_iter()
        .collect();
        let animation_table = vec!["legacyBattleAnim_Tackle".to_string()];

        assert_eq!(
            battle_animation_catalog_issues(&animations, &animation_table, 0),
            vec![
                BattleAnimationCatalogIssue::InvalidAnimation {
                    label: "fallbackBattleAnim_Tackle".to_string(),
                },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 0 },
            ],
        );
    }

    #[test]
    fn battle_animation_catalog_rejects_unknown_opcodes_instead_of_treating_them_as_noops() {
        let animations = [(
            "BattleAnim_Tackle".to_string(),
            vec![
                "anim_1gfx BATTLE_ANIM_GFX_HIT".to_string(),
                ".loop".to_string(),
                "anim_wait 1".to_string(),
                "anim_invented 4".to_string(),
                "anim_ret".to_string(),
            ],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            battle_animation_catalog_issues(&animations, &[], 0),
            vec![BattleAnimationCatalogIssue::InvalidCommand {
                label: "BattleAnim_Tackle".to_string(),
                index: 3,
                command: "anim_invented 4".to_string(),
            }]
        );
    }

    #[test]
    fn battle_animation_catalog_requires_exact_asm_command_arities() {
        let animations = [(
            "BattleAnim_Tackle".to_string(),
            vec![
                "anim_wait".to_string(),
                "anim_sound 0, 1".to_string(),
                "anim_obj BATTLE_ANIM_OBJ_HIT, 1, 2, 3, 4, 5".to_string(),
                "anim_ret 1".to_string(),
            ],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            battle_animation_catalog_issues(&animations, &[], 0),
            (0..4)
                .map(|index| BattleAnimationCatalogIssue::InvalidCommand {
                    label: "BattleAnim_Tackle".to_string(),
                    index,
                    command: animations["BattleAnim_Tackle"][index].clone(),
                })
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn battle_animation_catalog_requires_every_local_and_global_control_flow_target() {
        let animations = [
            (
                "BattleAnim_Tackle".to_string(),
                vec![
                    "anim_call BattleAnimSub_Hit".to_string(),
                    "anim_loop 2, .missing".to_string(),
                    "anim_ret".to_string(),
                ],
            ),
            (
                "BattleAnimSub_Hit".to_string(),
                vec![".loop".to_string(), "anim_jump .loop".to_string()],
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            battle_animation_catalog_issues(&animations, &[], 0),
            vec![BattleAnimationCatalogIssue::UnknownCommandTarget {
                label: "BattleAnim_Tackle".to_string(),
                index: 1,
                target: ".missing".to_string(),
            }]
        );

        let missing_global = [(
            "BattleAnim_Tackle".to_string(),
            vec![
                "anim_call BattleAnimSub_Missing".to_string(),
                "anim_ret".to_string(),
            ],
        )]
        .into_iter()
        .collect();
        assert_eq!(
            battle_animation_catalog_issues(&missing_global, &[], 0),
            vec![BattleAnimationCatalogIssue::UnknownCommandTarget {
                label: "BattleAnim_Tackle".to_string(),
                index: 0,
                target: "BattleAnimSub_Missing".to_string(),
            }]
        );
    }

    #[test]
    fn battle_animation_catalog_accepts_exported_colon_label_declarations() {
        let animations = [
            (
                "BattleAnim_FirePunch".to_string(),
                vec![
                    "anim_call BattleAnimSub_Fire".to_string(),
                    "anim_ret".to_string(),
                ],
            ),
            (
                "BattleAnim_BeatUp".to_string(),
                vec![
                    "anim_ret".to_string(),
                    "BattleAnimSub_Fire:".to_string(),
                    ".alternate:".to_string(),
                    "anim_jump .alternate".to_string(),
                    "anim_ret".to_string(),
                ],
            ),
        ]
        .into_iter()
        .collect();

        assert!(battle_animation_catalog_issues(&animations, &[], 0).is_empty());
    }

    #[test]
    fn battle_animation_catalog_rejects_legacy_inline_global_subroutine_sections() {
        let animations = [(
            "BattleAnim_BeatUp".to_string(),
            vec!["BattleAnimSub_Drain:".to_string(), "anim_ret".to_string()],
        )]
        .into_iter()
        .collect();

        assert_eq!(
            battle_animation_catalog_issues(&animations, &[], 0),
            vec![BattleAnimationCatalogIssue::InvalidCommand {
                label: "BattleAnim_BeatUp".to_string(),
                index: 0,
                command: "BattleAnimSub_Drain:".to_string(),
            }]
        );
    }

    #[test]
    fn battle_animation_command_table_deserialization_rejects_unresolved_targets() {
        let error = serde_json::from_value::<BattleAnimationCommandTable>(serde_json::json!({
            "BattleAnim_Tackle": ["anim_jump .missing"]
        }))
        .expect_err("unresolved local branch must fail during pack deserialization")
        .to_string();

        assert!(error.contains("unresolved ASM target"), "{error}");
        assert!(error.contains(".missing"), "{error}");
    }
}
