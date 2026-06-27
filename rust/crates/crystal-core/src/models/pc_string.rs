use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PcStringCatalogIssue {
    InvalidString { key: String },
}

pub fn pc_string_catalog_issues(
    pc_strings: &BTreeMap<String, String>,
) -> Vec<PcStringCatalogIssue> {
    pc_strings
        .iter()
        .filter(|(key, value)| !is_exact_nonempty_pc_string_key(key) || value.trim().is_empty())
        .map(|(key, _)| PcStringCatalogIssue::InvalidString { key: key.clone() })
        .collect()
}

fn is_exact_nonempty_pc_string_key(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pc_string_catalog_issues_require_nonempty_keys_and_values() {
        let pc_strings = [
            ("".to_string(), "Choose a <PK><MN>.".to_string()),
            (
                " PCString_Deposit".to_string(),
                "Deposit <PK><MN>.".to_string(),
            ),
            (
                "PCString Deposit".to_string(),
                "Deposit <PK><MN>.".to_string(),
            ),
            (
                "PCString_ChooseaPKMN".to_string(),
                "Choose a <PK><MN>.".to_string(),
            ),
            ("PCString_Padded".to_string(), " Withdraw.".to_string()),
            ("PCString_Trailing".to_string(), "Stored ".to_string()),
            ("PCString_Withdraw".to_string(), " ".to_string()),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            pc_string_catalog_issues(&pc_strings),
            vec![
                PcStringCatalogIssue::InvalidString { key: String::new() },
                PcStringCatalogIssue::InvalidString {
                    key: " PCString_Deposit".to_string(),
                },
                PcStringCatalogIssue::InvalidString {
                    key: "PCString Deposit".to_string(),
                },
                PcStringCatalogIssue::InvalidString {
                    key: "PCString_Withdraw".to_string(),
                },
            ],
        );
    }
}
