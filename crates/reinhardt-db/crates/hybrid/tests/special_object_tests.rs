//! Tests for hybrid properties with special object types (like Amount)
//! Based on SpecialObjectTest from SQLAlchemy

use reinhardt_hybrid::prelude::*;
use std::cmp::Ordering;

/// Represents a monetary amount with currency
#[derive(Debug, Clone, PartialEq)]
struct Amount {
	value: f64,
	currency: String,
}

impl Amount {
	fn new(value: f64, currency: &str) -> Self {
		Self {
			value,
			currency: currency.to_string(),
		}
	}

	/// Convert this amount to another currency
	fn as_currency(&self, target_currency: &str) -> Amount {
		// Simplified conversion rates (USD as base)
		let to_usd = match self.currency.as_str() {
			"USD" => 1.0,
			"EUR" => 1.1,
			"GBP" => 1.3,
			"CAD" => 0.75,
			_ => 1.0,
		};

		let from_usd = match target_currency {
			"USD" => 1.0,
			"EUR" => 0.91,
			"GBP" => 0.77,
			"CAD" => 1.33,
			_ => 1.0,
		};

		Amount::new(self.value * to_usd * from_usd, target_currency)
	}

	/// Generate SQL for currency conversion
	fn to_sql_conversion(&self, target_currency: &str) -> String {
		format!(
			"balance * {} /* {} to {} */",
			self.get_rate(target_currency),
			self.currency,
			target_currency
		)
	}

	fn get_rate(&self, target_currency: &str) -> f64 {
		let to_usd = match self.currency.as_str() {
			"USD" => 1.0,
			"EUR" => 1.1,
			"GBP" => 1.3,
			"CAD" => 0.75,
			_ => 1.0,
		};

		let from_usd = match target_currency {
			"USD" => 1.0,
			"EUR" => 0.91,
			"GBP" => 0.77,
			"CAD" => 1.33,
			_ => 1.0,
		};

		to_usd * from_usd
	}
}

impl PartialOrd for Amount {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		let other_in_my_currency = other.as_currency(&self.currency);
		self.value.partial_cmp(&other_in_my_currency.value)
	}
}

#[derive(Debug)]
struct BankAccount {
	id: i32,
	balance_value: f64,
}

impl BankAccount {
	fn new(id: i32, balance_value: f64) -> Self {
		Self { id, balance_value }
	}

	fn balance(&self) -> Amount {
		Amount::new(self.balance_value, "USD")
	}

	fn set_balance(&mut self, amount: Amount) {
		self.balance_value = amount.as_currency("USD").value;
	}
}

#[test]
fn test_instance_one() {
	// Test getting balance as Amount object
	let account = BankAccount::new(1, 100.0);
	let balance = account.balance();

	assert_eq!(balance.value, 100.0);
	assert_eq!(balance.currency, "USD");
}

#[test]
fn test_instance_two() {
	// Test balance conversion to GBP
	let account = BankAccount::new(1, 100.0);
	let balance = account.balance();
	let balance_gbp = balance.as_currency("GBP");

	assert_eq!(balance_gbp.currency, "GBP");
	// 100 USD * 1.0 (to USD) * 0.77 (to GBP) = 77
	assert!((balance_gbp.value - 77.0).abs() < 0.01);
}

#[test]
fn test_instance_three() {
	// Test currency-agnostic comparisons
	let account1 = BankAccount::new(1, 100.0);
	let account2 = BankAccount::new(2, 80.0);

	let balance1 = account1.balance();
	let balance2 = account2.balance();

	assert!(balance1 > balance2);
	assert!(balance2 < balance1);
}

#[test]
fn test_instance_four() {
	// Test arithmetic operations with Amount
	let account = BankAccount::new(1, 100.0);
	let balance = account.balance();
	let balance_eur = balance.as_currency("EUR");

	// 100 USD * 1.0 * 0.91 = 91 EUR
	assert!((balance_eur.value - 91.0).abs() < 0.01);
}

#[test]
fn test_query_one() {
	// Test hybrid property for database query
	let property = HybridProperty::new(|acc: &BankAccount| Amount::new(acc.balance_value, "USD"))
		.with_expression(|| "balance".to_string());

	let account = BankAccount::new(1, 100.0);
	let balance = property.get(&account);

	assert_eq!(balance.value, 100.0);
	assert_eq!(property.expression(), Some("balance".to_string()));
}

#[test]
fn test_query_two() {
	// Test SQL generation for currency conversion
	let account = BankAccount::new(1, 100.0);
	let balance = account.balance();

	let sql = balance.to_sql_conversion("EUR");
	assert!(sql.contains("balance"));
	assert!(sql.contains("EUR"));
}

#[test]
fn test_query_three() {
	// Test comparison in SQL expression
	let property = HybridProperty::new(|acc: &BankAccount| acc.balance_value > 50.0)
		.with_expression(|| "balance > 50".to_string());

	let account = BankAccount::new(1, 100.0);
	assert_eq!(property.get(&account), true);
}

#[test]
fn test_query_four() {
	// Test converting to CAD in SQL
	let account = BankAccount::new(1, 100.0);
	let balance = account.balance();
	let sql = balance.to_sql_conversion("CAD");

	assert!(sql.contains("CAD"));
	assert!(sql.contains("balance"));
}

#[test]
fn test_query_five() {
	// Test average balance calculation (SQL expression)
	let property: HybridProperty<BankAccount, f64> =
		HybridProperty::new(|acc: &BankAccount| acc.balance_value)
			.with_expression(|| "AVG(balance)".to_string());

	assert_eq!(property.expression(), Some("AVG(balance)".to_string()));
}

#[test]
fn test_setting_balance() {
	// Test setting balance with Amount object
	let mut account = BankAccount::new(1, 100.0);
	let new_balance = Amount::new(150.0, "USD");

	account.set_balance(new_balance);

	assert_eq!(account.balance_value, 150.0);
}

#[test]
fn test_setting_balance_with_conversion() {
	// Test setting balance with currency conversion
	let mut account = BankAccount::new(1, 100.0);
	let new_balance = Amount::new(100.0, "EUR");

	account.set_balance(new_balance);

	// 100 EUR * 1.1 (to USD) = 110 USD
	assert!((account.balance_value - 110.0).abs() < 0.01);
}

#[test]
fn test_amount_equality() {
	// Test Amount equality across currencies
	let amount1 = Amount::new(100.0, "USD");
	let amount2 = Amount::new(100.0, "USD");

	assert_eq!(amount1, amount2);
}

#[test]
fn test_amount_comparison_different_currencies() {
	// Test comparing amounts in different currencies
	let amount_usd = Amount::new(100.0, "USD");
	let amount_eur = Amount::new(100.0, "EUR");

	// 100 EUR is worth more than 100 USD
	assert!(amount_eur > amount_usd);
}

#[test]
fn test_hybrid_property_with_amount() {
	// Test hybrid property returning Amount
	let property = HybridProperty::new(|acc: &BankAccount| Amount::new(acc.balance_value, "USD"))
		.with_expression(|| "balance".to_string());

	let account = BankAccount::new(1, 250.0);
	let balance = property.get(&account);

	assert_eq!(balance.value, 250.0);
	assert_eq!(balance.currency, "USD");
}

#[test]
fn test_conversion_rate_accuracy() {
	// Test that conversion rates are applied correctly
	let amount = Amount::new(100.0, "USD");
	let eur = amount.as_currency("EUR");
	let back_to_usd = eur.as_currency("USD");

	// Due to floating point and conversion rates not being perfectly inverse,
	// we allow for a larger margin of error (0.11 to account for 1.1 * 0.91 = 1.001)
	assert!((back_to_usd.value - 100.0).abs() < 0.11);
}
