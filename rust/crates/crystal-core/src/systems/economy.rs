use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::state::GameState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScriptEconomyCommand {
    #[serde(deserialize_with = "required_economy_command_token")]
    pub command: String,
    #[serde(deserialize_with = "required_nullable_economy_token")]
    pub account: Option<String>,
    #[serde(deserialize_with = "required_economy_amount_token_vec")]
    pub amount_tokens: Vec<String>,
    #[serde(deserialize_with = "required_economy_source_token")]
    pub source_script: String,
    pub command_index: usize,
}

impl<'de> Deserialize<'de> for ScriptEconomyCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawScriptEconomyCommand {
            #[serde(default, deserialize_with = "required_economy_command_token")]
            command: String,
            #[serde(deserialize_with = "required_nullable_economy_token")]
            account: Option<String>,
            #[serde(deserialize_with = "required_economy_amount_token_vec")]
            amount_tokens: Vec<String>,
            #[serde(deserialize_with = "required_economy_source_token")]
            source_script: String,
            command_index: usize,
        }

        let raw = RawScriptEconomyCommand::deserialize(deserializer)?;
        let command = Self {
            command: raw.command,
            account: raw.account,
            amount_tokens: raw.amount_tokens,
            source_script: raw.source_script,
            command_index: raw.command_index,
        };
        if !command.command.is_empty() {
            validate_script_economy_command_shape(&command).map_err(D::Error::custom)?;
        }
        Ok(command)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct CurrencyCatalog(pub BTreeMap<String, u32>);

impl<'de> Deserialize<'de> for CurrencyCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let values = BTreeMap::<String, u32>::deserialize(deserializer)?;
        for constant in values.keys() {
            if !is_exact_economy_token(constant) {
                return Err(serde::de::Error::custom(format!(
                    "currency constant must be exact ASCII alphanumeric/underscore, found {constant:?}"
                )));
            }
        }
        Ok(Self(values))
    }
}

impl CurrencyCatalog {
    pub fn get(&self, id: &str) -> Option<u32> {
        self.0.get(id).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
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

    pub fn script_code(self) -> &'static str {
        match self {
            Self::HaveMore => "0",
            Self::HaveAmount => "1",
            Self::HaveLess => "2",
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
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
    SavedMoneyExceedsLimit { amount: u32, limit: u32 },
    SavedMomsMoneyExceedsLimit { amount: u32, limit: u32 },
    SavedCoinsExceedsLimit { amount: u16, limit: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
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

fn validate_script_economy_command_shape(command: &ScriptEconomyCommand) -> Result<(), String> {
    if !is_known_script_economy_command(&command.command) {
        return Err(format!(
            "unknown script economy command {}",
            command.command
        ));
    }
    if command.amount_tokens.is_empty() {
        return Err(format!(
            "script economy command {} requires amount tokens",
            command.command
        ));
    }
    if SCRIPT_MONEY_CHECK_COMMANDS.contains(&command.command.as_str())
        || SCRIPT_MONEY_MUTATION_COMMANDS.contains(&command.command.as_str())
    {
        let Some(account) = command.account.as_deref() else {
            return Err(format!(
                "script economy command {} requires money account",
                command.command
            ));
        };
        MoneyAccount::from_script_id(account).map_err(|error| format!("{error:?}"))?;
    } else if command.account.is_some() {
        return Err(format!(
            "script economy command {} must not declare money account",
            command.command
        ));
    }
    validate_amount_expression_shape(&command.amount_tokens)
}

fn validate_amount_expression_shape(amount_tokens: &[String]) -> Result<(), String> {
    if amount_tokens.is_empty() || amount_tokens.len() % 2 == 0 {
        return Err(format!(
            "script economy amount expression has invalid token count {}",
            amount_tokens.len()
        ));
    }
    for (index, token) in amount_tokens.iter().enumerate() {
        if index % 2 == 0 {
            if token == "+" || token == "-" {
                return Err(format!(
                    "script economy amount atom at index {index} must not be operator {token}"
                ));
            }
        } else if token != "+" && token != "-" {
            return Err(format!(
                "script economy amount operator at index {index} must be + or -"
            ));
        }
    }
    Ok(())
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
        && !has_reserved_pack_prefix(value)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn is_exact_economy_command_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_lowercase())
}

fn has_reserved_pack_prefix(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.starts_with("fallback") || value.starts_with("legacy")
}

fn is_exact_economy_source_token(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && !has_reserved_pack_prefix(value)
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn is_exact_economy_amount_token(value: &str) -> bool {
    value == "+"
        || value == "-"
        || (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
        || is_exact_economy_token(value)
        || is_exact_economy_amount_expression(value)
}

fn is_exact_economy_amount_expression(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-' | b' '))
        && !has_reserved_pack_prefix(value)
}

fn required_economy_command_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_economy_command_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script economy command must be exact lowercase ASCII, found {value:?}"
        )))
    }
}

fn required_nullable_economy_token<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(token) if is_exact_economy_token(&token) => Ok(Some(token)),
        Some(token) => Err(serde::de::Error::custom(format!(
            "script economy token must be exact ASCII alphanumeric/underscore, found {token:?}"
        ))),
        None => Ok(None),
    }
}

fn required_economy_amount_token_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values = Vec::<String>::deserialize(deserializer)?;
    for value in &values {
        if !is_exact_economy_amount_token(value) {
            return Err(serde::de::Error::custom(format!(
                "script economy amount token must be exact digits, '+', '-', or ASCII alphanumeric/underscore constant, found {value:?}"
            )));
        }
    }
    Ok(values)
}

fn required_economy_source_token<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if is_exact_economy_source_token(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(format!(
            "script economy source script must be exact visible ASCII, found {value:?}"
        )))
    }
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
            let script_value = check.comparison.script_code().to_string();
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
            let script_value = check.comparison.script_code().to_string();
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
    let Some(first) = amount_tokens.first() else {
        return Err(EconomyError::EmptyAmountExpression);
    };
    let mut value = resolve_amount_atom(first, constants)?;
    let mut chunks = amount_tokens[1..].chunks_exact(2);
    for pair in &mut chunks {
        let operator = pair[0].as_str();
        let rhs = pair[1].as_str();
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
    if !chunks.remainder().is_empty() {
        return Err(EconomyError::InvalidAmountExpression { expression });
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

pub fn validate_save_currency_for_runtime_pack(
    state: &GameState,
    constants: &CurrencyCatalog,
) -> Result<(), EconomyError> {
    let max_money = money_cap(constants)?;
    if state.money > max_money {
        return Err(EconomyError::SavedMoneyExceedsLimit {
            amount: state.money,
            limit: max_money,
        });
    }
    if state.moms_money > max_money {
        return Err(EconomyError::SavedMomsMoneyExceedsLimit {
            amount: state.moms_money,
            limit: max_money,
        });
    }

    let max_coins = coin_cap(constants)?;
    if state.coins > max_coins {
        return Err(EconomyError::SavedCoinsExceedsLimit {
            amount: state.coins,
            limit: max_coins,
        });
    }
    Ok(())
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
    fn economy_serialized_variants_reject_unknown_fallback_fields() {
        let outcome_error = serde_json::from_value::<ScriptEconomyOutcome>(serde_json::json!({
            "money_changed": {
                "command": "givemoney",
                "account": "YOUR_MONEY",
                "balance": 500,
                "source_script": "EconomyScript",
                "command_index": 9,
                "fallback_balance": 0
            }
        }))
        .expect_err("economy outcomes must not accept fallback balances")
        .to_string();
        assert!(
            outcome_error.contains("unknown field `fallback_balance`"),
            "{outcome_error}"
        );

        let error_error = serde_json::from_value::<EconomyError>(serde_json::json!({
            "MissingCurrencyLimit": {
                "constant": "MAX_MONEY",
                "fallback_limit": 999999
            }
        }))
        .expect_err("economy errors must not accept fallback currency limits")
        .to_string();
        assert!(
            error_error.contains("unknown field `fallback_limit`"),
            "{error_error}"
        );

        let issue_error = serde_json::from_value::<ScriptEconomyCommandIssue>(serde_json::json!({
            "unresolved_amount": {
                "error": {
                    "InvalidEconomyCommand": {
                        "command": "giveMoney",
                        "normalized_command": "givemoney"
                    }
                }
            }
        }))
        .expect_err("economy command issues must not accept normalized command aliases")
        .to_string();
        assert!(
            issue_error.contains("unknown field `normalized_command`"),
            "{issue_error}"
        );
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
            Err(EconomyError::InvalidAmountToken {
                token: "MAX_COINS - 1".to_string(),
            })
        );
        assert_eq!(
            resolve_amount(&tokens(&["MAX_COINS", "-", "1"]), &constants),
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

        let error = serde_json::from_str::<CurrencyCatalog>(
            r#"{"constants":{"MAX_COINS":9999},"fallback_limit":999999}"#,
        )
        .expect_err("currency catalogs must be the compiler-emitted constant map")
        .to_string();
        assert!(
            error.contains("invalid type") || error.contains("invalid value"),
            "{error}"
        );

        let error = serde_json::from_str::<CurrencyCatalog>(r#"{"MAX MONEY":999999}"#)
            .expect_err("malformed currency constant keys must fail during JSON load")
            .to_string();
        assert!(
            error.contains("currency constant")
                && error.contains("exact ASCII alphanumeric/underscore"),
            "{error}"
        );
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
    fn economy_tokens_reject_reserved_pack_prefixes() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());

        assert_eq!(
            script_economy_command_issues(
                &economy_command("fallbackmoney", Some("YOUR_MONEY"), &["PRICE"]),
                &constants,
            ),
            vec![ScriptEconomyCommandIssue::InvalidCommand]
        );
        assert_eq!(
            script_economy_command_issues(
                &economy_command("takemoney", Some("legacy_money"), &["PRICE"]),
                &constants,
            ),
            vec![
                ScriptEconomyCommandIssue::InvalidMoneyAccount,
                ScriptEconomyCommandIssue::MissingMoneyCap,
            ]
        );
        assert_eq!(
            resolve_amount(&["fallback_price".to_string()], &constants),
            Err(EconomyError::InvalidAmountToken {
                token: "fallback_price".to_string(),
            })
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
    fn applies_script_economy_checks_to_exact_numeric_accumulator_codes() {
        let constants = CurrencyCatalog([("PRICE".to_string(), 500)].into_iter().collect());
        let mut state = GameState {
            money: 500,
            coins: 400,
            ..GameState::default()
        };

        assert_eq!(AmountComparison::HaveMore.script_code(), "0");
        assert_eq!(AmountComparison::HaveAmount.script_code(), "1");
        assert_eq!(AmountComparison::HaveLess.script_code(), "2");

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
                script_value: "1".to_string(),
                source_script: "EconomyScript".to_string(),
                command_index: 9,
            }
        );
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("1"));

        apply_script_economy_command(
            &mut state,
            economy_command("checkcoins", None, &["PRICE"]),
            &constants,
        )
        .expect("check coins");
        assert_eq!(state.script_runtime.script_value.as_deref(), Some("2"));
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

    #[test]
    fn economy_json_rejects_legacy_alias_payloads() {
        let account_error = serde_json::from_value::<MoneyAccount>(serde_json::json!({
            "YOUR_MONEY": {
                "legacy_account": "money"
            }
        }))
        .expect_err("money accounts must not accept object-shaped aliases")
        .to_string();
        assert!(
            account_error.contains("invalid type")
                || account_error.contains("unknown field `legacy_account`"),
            "{account_error}"
        );

        let comparison_error = serde_json::from_value::<AmountComparison>(serde_json::json!({
            "HAVE_AMOUNT": {
                "fallback_comparison": "equal"
            }
        }))
        .expect_err("amount comparisons must not accept fallback aliases")
        .to_string();
        assert!(
            comparison_error.contains("invalid type")
                || comparison_error.contains("unknown field `fallback_comparison`"),
            "{comparison_error}"
        );

        for (field, value) in [
            ("command", serde_json::json!("take money")),
            ("command", serde_json::json!("fallbackmoney")),
            ("account", serde_json::json!("YOUR MONEY")),
            ("account", serde_json::json!("legacy_money")),
            ("source_script", serde_json::json!("fallback_script")),
        ] {
            let mut payload = serde_json::json!({
                "command": "takemoney",
                "account": "YOUR_MONEY",
                "amount_tokens": ["PRICE"],
                "source_script": "EconomyScript",
                "command_index": 9
            });
            payload[field] = value;
            let error = serde_json::from_value::<ScriptEconomyCommand>(payload)
                .expect_err("malformed economy command fields must fail during JSON load")
                .to_string();
            assert!(
                error.contains("script economy"),
                "{field} produced unexpected error: {error}"
            );
        }

        for amount_tokens in [
            serde_json::json!([""]),
            serde_json::json!(["PRICE TOKEN"]),
            serde_json::json!(["fallback_price"]),
        ] {
            let error = serde_json::from_value::<ScriptEconomyCommand>(serde_json::json!({
                "command": "takemoney",
                "account": "YOUR_MONEY",
                "amount_tokens": amount_tokens,
                "source_script": "EconomyScript",
                "command_index": 9
            }))
            .expect_err("malformed economy amount tokens must fail during JSON load")
            .to_string();
            assert!(error.contains("script economy amount token"), "{error}");
        }
    }
}
