use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleAnimationCatalogIssue {
    InvalidAnimation {
        label: String,
    },
    InvalidTableEntry {
        index: usize,
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

    for (label, commands) in animations {
        if !is_exact_nonempty_battle_animation_token(label) || commands.is_empty() {
            issues.push(BattleAnimationCatalogIssue::InvalidAnimation {
                label: label.clone(),
            });
        }
    }
    for (index, label) in animation_table.iter().enumerate() {
        if !is_exact_nonempty_battle_animation_token(label) {
            issues.push(BattleAnimationCatalogIssue::InvalidTableEntry { index });
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

fn is_exact_nonempty_battle_animation_token(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
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
                    label: "BattleAnim_Pound".to_string(),
                },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 1 },
                BattleAnimationCatalogIssue::InvalidTableEntry { index: 2 },
                BattleAnimationCatalogIssue::TableCountMismatch {
                    actual_count: 3,
                    expected_count: 2,
                },
            ],
        );
    }

    #[test]
    fn battle_animation_catalog_issues_allow_empty_table_for_partial_packs() {
        assert!(battle_animation_catalog_issues(&BTreeMap::new(), &[], 12).is_empty());
    }
}
