use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MoneyAccount {
    YourMoney,
    MomsMoney,
}

impl MoneyAccount {
    pub fn from_script_id(value: &str) -> Result<Self, EconomyError> {
        if !is_exact_economy_token(value) {
            return Err(EconomyError::InvalidMoneyAccount {
                account: value.to_string(),
            });
        }
        match value {
            "YOUR_MONEY" => Ok(Self::YourMoney),
            "MOMS_MONEY" => Ok(Self::MomsMoney),
            _ => Err(EconomyError::UnknownMoneyAccount {
                account: value.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptEconomyCommand {
    pub command: String,
    pub account: Option<String>,
    pub amount_tokens: Vec<String>,
    pub source_script: String,
    pub command_index: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyCatalog(pub BTreeMap<String, u32>);

impl CurrencyCatalog {
    pub fn get(&self, id: &str) -> Option<u32> {
        self.0.get(id).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AmountComparison {
    HaveLess,
    HaveAmount,
    HaveMore,
}

impl AmountComparison {
    pub fn script_label(self) -> &'static str {
        match self {
            Self::HaveLess => "HAVE_LESS",
            Self::HaveAmount => "HAVE_AMOUNT",
            Self::HaveMore => "HAVE_MORE",
        }
    }

    pub fn is_enough(self) -> bool {
        matches!(self, Self::HaveAmount | Self::HaveMore)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrencyCheck {
    pub current: u32,
    pub required: u32,
    pub comparison: AmountComparison,
    pub enough: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptEconomyOutcome {
    Check {
        command: String,
        current: u32,
        required: u32,
        comparison: AmountComparison,
        script_value: String,
        source_script: String,
        command_index: usize,
    },
    MoneyChanged {
        command: String,
        account: MoneyAccount,
        balance: u32,
        source_script: String,
        command_index: usize,
    },
    CoinsChanged {
        command: String,
        balance: u16,
        source_script: String,
        command_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EconomyError {
    EmptyAmountExpression,
    InvalidAmountExpression { expression: String },
    InvalidAmountToken { token: String },
    UnknownCurrencyConstant { token: String },
    MissingCurrencyLimit { constant: String },
    AmountOverflow { expression: String },
    CoinsAmountOutOfRange { amount: u32 },
    InvalidMoneyAccount { account: String },
    UnknownMoneyAccount { account: String },
    InvalidEconomyCommand { command: String },
    UnknownEconomyCommand { command: String },
    MissingMoneyAccount { command: String },
    UnexpectedMoneyAccount { command: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptEconomyCommandIssue {
    InvalidCommand,
    UnknownCommand,
    MissingMoneyAccount,
    InvalidMoneyAccount,
    UnknownMoneyAccount,
    UnexpectedCoinAccount,
    MissingMoneyCap,
    MissingCoinCap,
    UnresolvedAmount { error: EconomyError },
}

pub const SCRIPT_MONEY_CHECK_COMMANDS: &[&str] = &["checkmoney"];
pub const SCRIPT_MONEY_MUTATION_COMMANDS: &[&str] = &["takemoney", "givemoney"];
pub const SCRIPT_COIN_CHECK_COMMANDS: &[&str] = &["checkcoins"];
pub const SCRIPT_COIN_MUTATION_COMMANDS: &[&str] = &["givecoins", "takecoins"];

pub fn is_known_script_economy_command(command: &str) -> bool {
    SCRIPT_MONEY_CHECK_COMMANDS.contains(&command)
        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command)
        || SCRIPT_COIN_CHECK_COMMANDS.contains(&command)
        || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command)
}

pub fn script_economy_command_issues(
    command: &ScriptEconomyCommand,
    constants: &CurrencyCatalog,
) -> Vec<ScriptEconomyCommandIssue> {
    let mut issues = Vec::new();
    if !is_exact_economy_command_token(&command.command) {
        issues.push(ScriptEconomyCommandIssue::InvalidCommand);
        return issues;
    }
    if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
    {
        let Some(account) = command.account.as_deref() else {
            issues.push(ScriptEconomyCommandIssue::MissingMoneyAccount);
            return issues;
        };
        if !is_exact_economy_token(account) {
            issues.push(ScriptEconomyCommandIssue::InvalidMoneyAccount);
        } else if MoneyAccount::from_script_id(account).is_err() {
            issues.push(ScriptEconomyCommandIssue::UnknownMoneyAccount);
        }
        if SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
            && constants.get("MAX_MONEY").is_none()
        {
            issues.push(ScriptEconomyCommandIssue::MissingMoneyCap);
        }
    } else if SCRIPT_COIN_CHECK_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_COIN_MUTATION_COMMANDS.contains(&command.command.as_str())
    {
        if command.account.is_some() {
            issues.push(ScriptEconomyCommandIssue::UnexpectedCoinAccount);
        }
        if SCRIPT_COIN_MUTATION_COMMANDS.contains(&command.command.as_str())
            && constants.get("MAX_COINS").is_none()
        {
            issues.push(ScriptEconomyCommandIssue::MissingCoinCap);
        }
    } else {
        issues.push(ScriptEconomyCommandIssue::UnknownCommand);
        return issues;
    }
    if let Err(error) = resolve_amount(&command.amount_tokens, constants) {
        issues.push(ScriptEconomyCommandIssue::UnresolvedAmount { error });
    }
    issues
}

fn is_exact_economy_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_economy_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

pub fn apply_script_economy_command(
    state: &mut GameState,
    command: ScriptEconomyCommand,
    constants: &CurrencyCatalog,
) -> Result<ScriptEconomyOutcome, EconomyError> {
    validate_script_economy_command_token(&command.command)?;
    match command.command.as_str() {
        "checkmoney" => {
            let account = require_money_account(&command)?;
            let check = check_money(state, account, &command.amount_tokens, constants)?;
            let script_value = check.comparison.script_label().to_string();
            state.script_runtime.script_value = Some(script_value.clone());
            Ok(ScriptEconomyOutcome::Check {
                command: command.command,
                current: check.current,
                required: check.required,
                comparison: check.comparison,
                script_value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "takemoney" => {
            let account = require_money_account(&command)?;
            let balance = take_money(state, account, &command.amount_tokens, constants)?;
            Ok(ScriptEconomyOutcome::MoneyChanged {
                command: command.command,
                account,
                balance,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "givemoney" => {
            let account = require_money_account(&command)?;
            let balance = give_money(state, account, &command.amount_tokens, constants)?;
            Ok(ScriptEconomyOutcome::MoneyChanged {
                command: command.command,
                account,
                balance,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "checkcoins" => {
            reject_money_account(&command)?;
            let check = check_coins(state, &command.amount_tokens, constants)?;
            let script_value = check.comparison.script_label().to_string();
            state.script_runtime.script_value = Some(script_value.clone());
            Ok(ScriptEconomyOutcome::Check {
                command: command.command,
                current: check.current,
                required: check.required,
                comparison: check.comparison,
                script_value,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "givecoins" => {
            reject_money_account(&command)?;
            let balance = give_coins(state, &command.amount_tokens, constants)?;
            Ok(ScriptEconomyOutcome::CoinsChanged {
                command: command.command,
                balance,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        "takecoins" => {
            reject_money_account(&command)?;
            let balance = take_coins(state, &command.amount_tokens, constants)?;
            Ok(ScriptEconomyOutcome::CoinsChanged {
                command: command.command,
                balance,
                source_script: command.source_script,
                command_index: command.command_index,
            })
        }
        other => Err(EconomyError::UnknownEconomyCommand {
            command: other.to_string(),
        }),
    }
}

fn validate_script_economy_command_token(command: &str) -> Result<(), EconomyError> {
    if is_exact_economy_command_token(command) {
        Ok(())
    } else {
        Err(EconomyError::InvalidEconomyCommand {
            command: command.to_string(),
        })
    }
}

pub fn resolve_amount(
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u32, EconomyError> {
    let expression = amount_tokens.join(" ");
    let mut tokens = expression.split_whitespace();
    let Some(first) = tokens.next() else {
        return Err(EconomyError::EmptyAmountExpression);
    };
    let mut value = resolve_amount_atom(first, constants)?;
    while let Some(operator) = tokens.next() {
        let Some(rhs) = tokens.next() else {
            return Err(EconomyError::InvalidAmountExpression { expression });
        };
        let amount = resolve_amount_atom(rhs, constants)?;
        value =
            match operator {
                "+" => value
                    .checked_add(amount)
                    .ok_or_else(|| EconomyError::AmountOverflow {
                        expression: expression.clone(),
                    })?,
                "-" => value.checked_sub(amount).ok_or_else(|| {
                    EconomyError::InvalidAmountExpression {
                        expression: expression.clone(),
                    }
                })?,
                _ => {
                    return Err(EconomyError::InvalidAmountToken {
                        token: operator.to_string(),
                    });
                }
            };
    }
    Ok(value)
}

fn resolve_amount_atom(token: &str, constants: &CurrencyCatalog) -> Result<u32, EconomyError> {
    if token.is_empty() {
        return Err(EconomyError::InvalidAmountToken {
            token: token.to_string(),
        });
    }
    if token.bytes().all(|byte| byte.is_ascii_digit()) {
        return token
            .parse::<u32>()
            .map_err(|_| EconomyError::InvalidAmountToken {
                token: token.to_string(),
            });
    }
    if !is_exact_economy_token(token) {
        return Err(EconomyError::InvalidAmountToken {
            token: token.to_string(),
        });
    }
    constants
        .get(token)
        .ok_or_else(|| EconomyError::UnknownCurrencyConstant {
            token: token.to_string(),
        })
}

pub fn check_money(
    state: &GameState,
    account: MoneyAccount,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<CurrencyCheck, EconomyError> {
    let required = resolve_amount(amount_tokens, constants)?;
    let current = match account {
        MoneyAccount::YourMoney => state.money,
        MoneyAccount::MomsMoney => state.moms_money,
    };
    Ok(compare_currency(current, required))
}

pub fn take_money(
    state: &mut GameState,
    account: MoneyAccount,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u32, EconomyError> {
    let amount = resolve_amount(amount_tokens, constants)?;
    let cap = money_cap(constants)?;
    let balance = match account {
        MoneyAccount::YourMoney => &mut state.money,
        MoneyAccount::MomsMoney => &mut state.moms_money,
    };
    *balance = balance.saturating_sub(amount).min(cap);
    Ok(*balance)
}

pub fn give_money(
    state: &mut GameState,
    account: MoneyAccount,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u32, EconomyError> {
    let amount = resolve_amount(amount_tokens, constants)?;
    let cap = money_cap(constants)?;
    let balance = match account {
        MoneyAccount::YourMoney => &mut state.money,
        MoneyAccount::MomsMoney => &mut state.moms_money,
    };
    *balance = balance.saturating_add(amount).min(cap);
    Ok(*balance)
}

fn money_cap(constants: &CurrencyCatalog) -> Result<u32, EconomyError> {
    constants
        .get("MAX_MONEY")
        .ok_or_else(|| EconomyError::MissingCurrencyLimit {
            constant: "MAX_MONEY".to_string(),
        })
}

pub fn check_coins(
    state: &GameState,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<CurrencyCheck, EconomyError> {
    let required = resolve_amount(amount_tokens, constants)?;
    Ok(compare_currency(u32::from(state.coins), required))
}

pub fn give_coins(
    state: &mut GameState,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u16, EconomyError> {
    let amount = resolve_coin_amount(amount_tokens, constants)?;
    let cap = coin_cap(constants)?;
    state.coins = state.coins.saturating_add(amount).min(cap);
    Ok(state.coins)
}

pub fn take_coins(
    state: &mut GameState,
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u16, EconomyError> {
    let amount = resolve_coin_amount(amount_tokens, constants)?;
    state.coins = state.coins.saturating_sub(amount);
    Ok(state.coins)
}

fn resolve_coin_amount(
    amount_tokens: &[String],
    constants: &CurrencyCatalog,
) -> Result<u16, EconomyError> {
    let amount = resolve_amount(amount_tokens, constants)?;
    let cap = u32::from(coin_cap(constants)?);
    u16::try_from(amount)
        .ok()
        .filter(|amount| u32::from(*amount) <= cap)
        .ok_or(EconomyError::CoinsAmountOutOfRange { amount })
}

fn coin_cap(constants: &CurrencyCatalog) -> Result<u16, EconomyError> {
    let cap = constants
        .get("MAX_COINS")
        .ok_or_else(|| EconomyError::MissingCurrencyLimit {
            constant: "MAX_COINS".to_string(),
        })?;
    u16::try_from(cap).map_err(|_| EconomyError::CoinsAmountOutOfRange { amount: cap })
}

fn compare_currency(current: u32, required: u32) -> CurrencyCheck {
    let comparison = if current < required {
        AmountComparison::HaveLess
    } else if current == required {
        AmountComparison::HaveAmount
    } else {
        AmountComparison::HaveMore
    };
    CurrencyCheck {
        current,
        required,
        comparison,
        enough: comparison.is_enough(),
    }
}

fn require_money_account(command: &ScriptEconomyCommand) -> Result<MoneyAccount, EconomyError> {
    command
        .account
        .as_deref()
        .ok_or_else(|| EconomyError::MissingMoneyAccount {
            command: command.command.clone(),
        })
        .and_then(MoneyAccount::from_script_id)
}

fn reject_money_account(command: &ScriptEconomyCommand) -> Result<(), EconomyError> {
    if command.account.is_some() {
        Err(EconomyError::UnexpectedMoneyAccount {
            command: command.command.clone(),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_MAX_MONEY: u32 = 999_999;
    const TEST_MAX_COINS: u16 = 9_999;

    fn tokens(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn economy_command(
        name: &str,
        account: Option<&str>,
        amount_tokens: &[&str],
    ) -> ScriptEconomyCommand {
        ScriptEconomyCommand {
            command: name.to_string(),
            account: account.map(str::to_string),
            amount_tokens: tokens(amount_tokens),
            source_script: "EconomyScript".to_string(),
            command_index: 9,
        }
    }

    #[test]
    fn resolves_amounts_from_exact_constants_and_expressions() {
        let constants = CurrencyCatalog(
            [
                ("ROUTE43GATE_TOLL".to_string(), 1_000),
                ("MAX_COINS".to_string(), u32::from(TEST_MAX_COINS)),
            ]
            .into_iter()
            .collect(),
        );

        assert_eq!(
            resolve_amount(&tokens(&["ROUTE43GATE_TOLL", "-", "1"]), &constants),
            Ok(999)
        );
        assert_eq!(
            resolve_amount(&tokens(&["MAX_COINS - 1"]), &constants),
            Ok(9_998)
        );
        assert_eq!(
            resolve_amount(&tokens(&["route43gate_toll"]), &constants),
            Err(EconomyError::UnknownCurrencyConstant {
                token: "route43gate_toll".to_string(),
            })
        );
        assert_eq!(
            resolve_amount(&tokens(&["ROUTE43GATE-TOLL"]), &constants),
            Err(EconomyError::InvalidAmountToken {
                token: "ROUTE43GATE-TOLL".to_string(),
            })
        );
    }

    #[test]
    fn money_checks_and_mutation_use_exact_accounts() {
        let constants = CurrencyCatalog(
            [
                ("TOLL".to_string(), 1_000),
                ("MAX_MONEY".to_string(), TEST_MAX_MONEY),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState {
            money: 1_200,
            moms_money: 500,
            ..GameState::default()
        };

        let check = check_money(
            &state,
            MoneyAccount::YourMoney,
            &tokens(&["TOLL"]),
            &constants,
        )
        .expect("check money");
        assert_eq!(check.comparison, AmountComparison::HaveMore);
        assert!(check.enough);

        assert_eq!(
            take_money(
                &mut state,
                MoneyAccount::YourMoney,
                &tokens(&["TOLL"]),
                &constants
            ),
            Ok(200)
        );
        assert_eq!(
            take_money(
                &mut state,
                MoneyAccount::MomsMoney,
                &tokens(&["TOLL"]),
                &constants
            ),
            Ok(0)
        );
    }

    #[test]
    fn coin_operations_clamp_to_coin_case_and_reject_oversized_amounts() {
        let constants = CurrencyCatalog(
            [("MAX_COINS".to_string(), u32::from(TEST_MAX_COINS))]
                .into_iter()
                .collect(),
        );
        let mut state = GameState {
            coins: TEST_MAX_COINS - 1,
            ..GameState::default()
        };

        assert_eq!(
            give_coins(&mut state, &tokens(&["18"]), &constants),
            Ok(TEST_MAX_COINS)
        );
        assert_eq!(
            check_coins(&state, &tokens(&["MAX_COINS", "-", "1"]), &constants)
                .expect("check coins")
                .comparison,
            AmountComparison::HaveMore
        );
        assert_eq!(
            take_coins(&mut state, &tokens(&["99999"]), &constants),
            Err(EconomyError::CoinsAmountOutOfRange { amount: 99_999 })
        );
    }

    #[test]
    fn coin_mutations_require_explicit_max_coins_constant_without_global_cap_fallback() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        let mut state = GameState {
            coins: 800,
            ..GameState::default()
        };

        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("givecoins", None, &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::MissingCurrencyLimit {
                constant: "MAX_COINS".to_string(),
            })
        );
        assert_eq!(state.coins, 800);
    }

    #[test]
    fn currency_catalog_default_is_empty_without_builtin_constants() {
        let constants = CurrencyCatalog::default();

        assert_eq!(constants.get("MAX_COINS"), None);
        assert_eq!(constants.get("MAX_MONEY"), None);
    }

    #[test]
    fn exported_economy_command_sets_are_exact() {
        assert!(SCRIPT_MONEY_CHECK_COMMANDS.contains(&"checkmoney"));
        assert!(SCRIPT_MONEY_MUTATION_COMMANDS.contains(&"takemoney"));
        assert!(SCRIPT_MONEY_MUTATION_COMMANDS.contains(&"givemoney"));
        assert!(SCRIPT_COIN_CHECK_COMMANDS.contains(&"checkcoins"));
        assert!(SCRIPT_COIN_MUTATION_COMMANDS.contains(&"givecoins"));
        assert!(SCRIPT_COIN_MUTATION_COMMANDS.contains(&"takecoins"));
        assert!(is_known_script_economy_command("checkmoney"));
        assert!(!is_known_script_economy_command("CheckMoney"));
        assert!(!is_known_script_economy_command("paymoney"));
    }

    #[test]
    fn script_economy_issue_collector_reports_exact_pack_shape_errors() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        assert_eq!(
            script_economy_command_issues(
                &economy_command("takemoney", None, &["PRICE"]),
                &constants,
            ),
            vec![ScriptEconomyCommandIssue::MissingMoneyAccount]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("takemoney", Some("your_money"), &["PRICE"]),
                &constants,
            ),
            vec![
                ScriptEconomyCommandIssue::UnknownMoneyAccount,
                ScriptEconomyCommandIssue::MissingMoneyCap,
            ]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("takemoney", Some(" YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            vec![
                ScriptEconomyCommandIssue::InvalidMoneyAccount,
                ScriptEconomyCommandIssue::MissingMoneyCap,
            ]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("takemoney", Some("YOUR MONEY"), &["PRICE"]),
                &constants,
            ),
            vec![
                ScriptEconomyCommandIssue::InvalidMoneyAccount,
                ScriptEconomyCommandIssue::MissingMoneyCap,
            ]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("givecoins", Some("YOUR_MONEY"), &["price"]),
                &constants,
            ),
            vec![
                ScriptEconomyCommandIssue::UnexpectedCoinAccount,
                ScriptEconomyCommandIssue::MissingCoinCap,
                ScriptEconomyCommandIssue::UnresolvedAmount {
                    error: EconomyError::UnknownCurrencyConstant {
                        token: "price".to_string(),
                    },
                },
            ]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("CheckMoney", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            vec![ScriptEconomyCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("paymoney", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            vec![ScriptEconomyCommandIssue::UnknownCommand]
        );
    }

    #[test]
    fn money_account_ids_are_exact() {
        assert_eq!(
            MoneyAccount::from_script_id("your_money"),
            Err(EconomyError::UnknownMoneyAccount {
                account: "your_money".to_string(),
            })
        );
        assert_eq!(
            MoneyAccount::from_script_id("YOUR MONEY"),
            Err(EconomyError::InvalidMoneyAccount {
                account: "YOUR MONEY".to_string(),
            })
        );
        assert_eq!(
            MoneyAccount::from_script_id("YOUR_MONEY"),
            Ok(MoneyAccount::YourMoney)
        );
    }

    #[test]
    fn applies_script_economy_checks_to_exact_accumulator_labels() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        let mut state = GameState {
            money: 500,
            coins: 400,
            ..GameState::default()
        };

        let money = apply_script_economy_command(
            &mut state,
            economy_command("checkmoney", Some("YOUR_MONEY"), &["PRICE"]),
            &constants,
        )
        .expect("check money");
        assert_eq!(
            money,
            ScriptEconomyOutcome::Check {
                command: "checkmoney".to_string(),
                current: 500,
                required: 500,
                comparison: AmountComparison::HaveAmount,
                script_value: "HAVE_AMOUNT".to_string(),
                source_script: "EconomyScript".to_string(),
                command_index: 9,
            }
        );
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some("HAVE_AMOUNT")
        );

        apply_script_economy_command(
            &mut state,
            economy_command("checkcoins", None, &["PRICE"]),
            &constants,
        )
        .expect("check coins");
        assert_eq!(
            state.script_runtime.script_value.as_deref(),
            Some("HAVE_LESS")
        );
    }

    #[test]
    fn applies_script_economy_mutations_with_exact_accounts() {
        let constants = CurrencyCatalog(
            [
                ("PRICE".to_string(), 500),
                ("MAX_MONEY".to_string(), TEST_MAX_MONEY),
                ("MAX_COINS".to_string(), u32::from(TEST_MAX_COINS)),
            ]
            .into_iter()
            .collect(),
        );
        let mut state = GameState {
            money: 800,
            moms_money: 300,
            coins: 10,
            ..GameState::default()
        };

        let money = apply_script_economy_command(
            &mut state,
            economy_command("takemoney", Some("YOUR_MONEY"), &["PRICE"]),
            &constants,
        )
        .expect("take money");
        assert_eq!(
            money,
            ScriptEconomyOutcome::MoneyChanged {
                command: "takemoney".to_string(),
                account: MoneyAccount::YourMoney,
                balance: 300,
                source_script: "EconomyScript".to_string(),
                command_index: 9,
            }
        );
        assert_eq!(state.money, 300);

        apply_script_economy_command(
            &mut state,
            economy_command("givecoins", None, &["PRICE"]),
            &constants,
        )
        .expect("give coins");
        assert_eq!(state.coins, 510);
    }

    #[test]
    fn money_mutations_require_explicit_max_money_constant_without_global_cap_fallback() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        let mut state = GameState {
            money: 800,
            ..GameState::default()
        };

        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("givemoney", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::MissingCurrencyLimit {
                constant: "MAX_MONEY".to_string(),
            })
        );
        assert_eq!(state.money, 800);
    }

    #[test]
    fn rejects_malformed_script_economy_commands_without_mutation() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        let mut state = GameState {
            money: 800,
            coins: 10,
            ..GameState::default()
        };

        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("take money", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::InvalidEconomyCommand {
                command: "take money".to_string(),
            })
        );
        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("takemoney", Some("YOUR MONEY"), &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::InvalidMoneyAccount {
                account: "YOUR MONEY".to_string(),
            })
        );
        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("takemoney", Some("your_money"), &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::UnknownMoneyAccount {
                account: "your_money".to_string(),
            })
        );
        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("checkcoins", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            Err(EconomyError::UnexpectedMoneyAccount {
                command: "checkcoins".to_string(),
            })
        );
        assert_eq!(
            apply_script_economy_command(
                &mut state,
                economy_command("givecoins", None, &["price"]),
                &constants,
            ),
            Err(EconomyError::UnknownCurrencyConstant {
                token: "price".to_string(),
            })
        );
        assert_eq!(state.money, 800);
        assert_eq!(state.coins, 10);
        assert_eq!(state.script_runtime.script_value, None);
    }
}
