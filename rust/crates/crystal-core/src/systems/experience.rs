use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrowthRateCurve {
    #[serde(deserialize_with = "required_growth_rate_token")]
    pub id: String,
    pub numerator: i32,
    pub denominator: i32,
    pub quadratic: i32,
    pub linear: i32,
    pub constant: i32,
}

pub type GrowthRateCatalog = BTreeMap<String, GrowthRateCurve>;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ExperienceError {
    #[error("invalid growth-rate id '{growth_rate}'")]
    InvalidGrowthRate { growth_rate: String },
    #[error("missing growth-rate curve '{growth_rate}'")]
    MissingGrowthRate { growth_rate: String },
    #[error("growth-rate curve '{growth_rate}' has zero denominator")]
    ZeroDenominator { growth_rate: String },
    #[error("growth-rate curve '{growth_rate}' does not declare matching id '{declared_id}'")]
    MismatchedGrowthRateId {
        growth_rate: String,
        declared_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum GrowthRateCatalogIssue {
    InvalidCatalogId {
        growth_rate: String,
    },
    MismatchedCurveId {
        growth_rate: String,
        declared_id: String,
    },
    ZeroDenominator {
        growth_rate: String,
    },
}

pub fn growth_rate_catalog_issues(catalog: &GrowthRateCatalog) -> Vec<GrowthRateCatalogIssue> {
    let mut issues = Vec::new();
    for (growth_rate, curve) in catalog {
        if !is_exact_growth_rate_token(growth_rate) {
            issues.push(GrowthRateCatalogIssue::InvalidCatalogId {
                growth_rate: growth_rate.clone(),
            });
        }
        if curve.id != *growth_rate {
            issues.push(GrowthRateCatalogIssue::MismatchedCurveId {
                growth_rate: growth_rate.clone(),
                declared_id: curve.id.clone(),
            });
        }
        if curve.denominator == 0 {
            issues.push(GrowthRateCatalogIssue::ZeroDenominator {
                growth_rate: growth_rate.clone(),
            });
        }
    }
    issues
}

pub fn calculate_experience(
    catalog: &GrowthRateCatalog,
    growth_rate: &str,
    level: u8,
) -> Result<i32, ExperienceError> {
    if !is_exact_growth_rate_token(growth_rate) {
        return Err(ExperienceError::InvalidGrowthRate {
            growth_rate: growth_rate.to_string(),
        });
    }
    let curve = catalog
        .get(growth_rate)
        .ok_or_else(|| ExperienceError::MissingGrowthRate {
            growth_rate: growth_rate.to_string(),
        })?;
    if curve.id != growth_rate {
        return Err(ExperienceError::MismatchedGrowthRateId {
            growth_rate: growth_rate.to_string(),
            declared_id: curve.id.clone(),
        });
    }
    if curve.denominator == 0 {
        return Err(ExperienceError::ZeroDenominator {
            growth_rate: growth_rate.to_string(),
        });
    }

    let n = i32::from(level);
    let n2 = n * n;
    let n3 = n2 * n;
    Ok(
        ((curve.numerator * n3) / curve.denominator) + (curve.quadratic * n2) + (curve.linear * n)
            - curve.constant,
    )
}

fn is_exact_growth_rate_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn required_growth_rate_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_growth_rate_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "growth-rate id must be exact ASCII alphanumeric/underscore, found {value:?}"
        )))
    }
}

#[cfg(test)]
pub fn crystal_growth_rate_catalog_for_tests() -> GrowthRateCatalog {
    [
        ("GROWTH_MEDIUM_FAST", 1, 1, 0, 0, 0),
        ("GROWTH_SLIGHTLY_FAST", 3, 4, 10, 0, 30),
        ("GROWTH_SLIGHTLY_SLOW", 3, 4, 20, 0, 70),
        ("GROWTH_MEDIUM_SLOW", 6, 5, -15, 100, 140),
        ("GROWTH_FAST", 4, 5, 0, 0, 0),
        ("GROWTH_SLOW", 5, 4, 0, 0, 0),
    ]
    .into_iter()
    .map(
        |(id, numerator, denominator, quadratic, linear, constant)| {
            (
                id.to_string(),
                GrowthRateCurve {
                    id: id.to_string(),
                    numerator,
                    denominator,
                    quadratic,
                    linear,
                    constant,
                },
            )
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_pokecrystal_growth_rate_table_cases() {
        let catalog = crystal_growth_rate_catalog_for_tests();

        assert_eq!(
            calculate_experience(&catalog, "GROWTH_MEDIUM_FAST", 1),
            Ok(1)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_MEDIUM_FAST", 50),
            Ok(125000)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_MEDIUM_FAST", 100),
            Ok(1000000)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_SLIGHTLY_FAST", 50),
            Ok(118720)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_SLIGHTLY_SLOW", 50),
            Ok(143680)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_MEDIUM_SLOW", 50),
            Ok(117360)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_FAST", 50),
            Ok(100000)
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_SLOW", 50),
            Ok(156250)
        );
    }

    #[test]
    fn rejects_missing_or_invalid_growth_rate_data_without_fallback() {
        let catalog = crystal_growth_rate_catalog_for_tests();
        assert_eq!(
            calculate_experience(&catalog, "GROWTH CUSTOM", 5),
            Err(ExperienceError::InvalidGrowthRate {
                growth_rate: "GROWTH CUSTOM".to_string()
            })
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_CUSTOM ", 5),
            Err(ExperienceError::InvalidGrowthRate {
                growth_rate: "GROWTH_CUSTOM ".to_string()
            })
        );
        assert_eq!(
            calculate_experience(&catalog, "GROWTH_CUSTOM", 5),
            Err(ExperienceError::MissingGrowthRate {
                growth_rate: "GROWTH_CUSTOM".to_string()
            })
        );

        let mut invalid = catalog;
        invalid.insert(
            "GROWTH_ZERO".to_string(),
            GrowthRateCurve {
                id: "GROWTH_ZERO".to_string(),
                numerator: 1,
                denominator: 0,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );
        assert_eq!(
            calculate_experience(&invalid, "GROWTH_ZERO", 5),
            Err(ExperienceError::ZeroDenominator {
                growth_rate: "GROWTH_ZERO".to_string()
            })
        );
    }

    #[test]
    fn growth_rate_catalog_issues_validate_exact_curve_ids() {
        let mut catalog = crystal_growth_rate_catalog_for_tests();
        catalog.insert(
            "GROWTH BAD".to_string(),
            GrowthRateCurve {
                id: "GROWTH BAD".to_string(),
                numerator: 1,
                denominator: 1,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );
        catalog.insert(
            "GROWTH_MISMATCH".to_string(),
            GrowthRateCurve {
                id: "GROWTH_OTHER".to_string(),
                numerator: 1,
                denominator: 1,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );
        catalog.insert(
            "GROWTH_ZERO".to_string(),
            GrowthRateCurve {
                id: "GROWTH_ZERO".to_string(),
                numerator: 1,
                denominator: 0,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );

        assert_eq!(
            growth_rate_catalog_issues(&catalog),
            vec![
                GrowthRateCatalogIssue::InvalidCatalogId {
                    growth_rate: "GROWTH BAD".to_string()
                },
                GrowthRateCatalogIssue::MismatchedCurveId {
                    growth_rate: "GROWTH_MISMATCH".to_string(),
                    declared_id: "GROWTH_OTHER".to_string(),
                },
                GrowthRateCatalogIssue::ZeroDenominator {
                    growth_rate: "GROWTH_ZERO".to_string()
                },
            ]
        );
    }

    #[test]
    fn growth_rate_tokens_reject_reserved_pack_prefixes() {
        let mut catalog = crystal_growth_rate_catalog_for_tests();
        catalog.insert(
            "fallback_growth_fast".to_string(),
            GrowthRateCurve {
                id: "fallback_growth_fast".to_string(),
                numerator: 1,
                denominator: 1,
                quadratic: 0,
                linear: 0,
                constant: 0,
            },
        );

        assert_eq!(
            growth_rate_catalog_issues(&catalog),
            vec![GrowthRateCatalogIssue::InvalidCatalogId {
                growth_rate: "fallback_growth_fast".to_string(),
            }]
        );
        assert_eq!(
            calculate_experience(&catalog, "legacy_growth_fast", 5),
            Err(ExperienceError::InvalidGrowthRate {
                growth_rate: "legacy_growth_fast".to_string(),
            })
        );
    }

    #[test]
    fn experience_json_rejects_legacy_alias_payloads() {
        let curve_error = serde_json::from_value::<GrowthRateCurve>(serde_json::json!({
            "id": "GROWTH FAST",
            "numerator": 1,
            "denominator": 1,
            "quadratic": 0,
            "linear": 0,
            "constant": 0
        }))
        .expect_err("growth-rate curve ids must be exact during JSON load")
        .to_string();
        assert!(
            curve_error.contains("growth-rate id must be exact ASCII alphanumeric/underscore"),
            "{curve_error}"
        );

        let issue_error = serde_json::from_value::<GrowthRateCatalogIssue>(serde_json::json!({
            "mismatched_curve_id": {
                "growth_rate": "GROWTH_FAST",
                "declared_id": "GROWTH_MEDIUM_FAST",
                "fallback_growth_rate": "FAST"
            }
        }))
        .expect_err("growth-rate issues must not accept fallback aliases")
        .to_string();
        assert!(
            issue_error.contains("unknown field `fallback_growth_rate`"),
            "{issue_error}"
        );

        let error_error = serde_json::from_value::<ExperienceError>(serde_json::json!({
            "MissingGrowthRate": {
                "growth_rate": "GROWTH_FAST",
                "legacy_growth_rate": "FAST"
            }
        }))
        .expect_err("experience errors must not accept legacy aliases")
        .to_string();
        assert!(
            error_error.contains("unknown field `legacy_growth_rate`"),
            "{error_error}"
        );
    }
}
