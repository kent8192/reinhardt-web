//! Tests for hybrid methods
//! Based on MethodExpressionTest from SQLAlchemy

use reinhardt_hybrid::prelude::*;

#[derive(Debug)]
struct Account {
	id: i32,
	balance: f64,
	currency: String,
}

impl Account {
	fn new(id: i32, balance: f64, currency: String) -> Self {
		Self {
			id,
			balance,
			currency,
		}
	}
}

#[test]
fn test_method_call() {
	// Test basic hybrid method invocation
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, multiplier: f64| acc.balance * multiplier);

	assert_eq!(method.call(&account, 2.0), 200.0);
}

#[test]
fn test_method_with_expression() {
	// Test hybrid method with SQL expression
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, multiplier: f64| acc.balance * multiplier)
		.with_expression(|multiplier: f64| format!("balance * {}", multiplier));

	assert_eq!(method.call(&account, 2.0), 200.0);
	assert_eq!(method.expression(2.0), Some("balance * 2".to_string()));
}

#[test]
fn test_method_with_string_argument() {
	// Test hybrid method with string argument
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, currency: String| {
		if acc.currency == currency {
			acc.balance
		} else {
			0.0
		}
	});

	assert_eq!(method.call(&account, "USD".to_string()), 100.0);
	assert_eq!(method.call(&account, "EUR".to_string()), 0.0);
}

#[test]
fn test_method_expression_with_column() {
	// Test method expression that references column
	let method: HybridMethod<Account, f64, f64> =
		HybridMethod::new(|acc: &Account, rate: f64| acc.balance * rate)
			.with_expression(|rate: f64| format!("balance * {}", rate));

	assert_eq!(method.expression(1.5), Some("balance * 1.5".to_string()));
}

#[test]
fn test_method_with_multiple_operations() {
	// Test method with multiple operations
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, amount: f64| (acc.balance + amount) * 1.1)
		.with_expression(|amount: f64| format!("(balance + {}) * 1.1", amount));

	assert_eq!(method.call(&account, 50.0), 165.0);
	assert_eq!(
		method.expression(50.0),
		Some("(balance + 50) * 1.1".to_string())
	);
}

#[test]
fn test_method_without_expression() {
	// Test method without SQL expression
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, tax_rate: f64| acc.balance * (1.0 - tax_rate));

	assert_eq!(method.call(&account, 0.1), 90.0);
	assert_eq!(method.expression(0.1), None);
}

#[test]
fn test_method_with_conditional() {
	// Test method with conditional logic
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, threshold: f64| {
		if acc.balance > threshold {
			"high".to_string()
		} else {
			"low".to_string()
		}
	});

	assert_eq!(method.call(&account, 50.0), "high");
	assert_eq!(method.call(&account, 150.0), "low");
}

#[test]
fn test_method_expression_with_function() {
	// Test method expression with SQL function
	let method: HybridMethod<Account, String, String> =
		HybridMethod::new(|_acc: &Account, prefix: String| prefix)
			.with_expression(|prefix: String| format!("CONCAT('{}', balance)", prefix));

	assert_eq!(
		method.expression("$".to_string()),
		Some("CONCAT('$', balance)".to_string())
	);
}

#[test]
fn test_method_with_zero_argument() {
	// Test that method can work with simple types
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, _unit: ()| acc.balance);

	assert_eq!(method.call(&account, ()), 100.0);
}

#[test]
fn test_method_chaining_result() {
	// Test that method results can be chained
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, divisor: f64| acc.balance / divisor);

	let result = method.call(&account, 2.0);
	assert_eq!(result, 50.0);
	assert_eq!(result * 2.0, 100.0);
}

#[test]
fn test_method_with_complex_type() {
	// Test method with tuple argument
	let account = Account::new(1, 100.0, "USD".to_string());

	let method =
		HybridMethod::new(|acc: &Account, params: (f64, f64)| acc.balance * params.0 + params.1);

	assert_eq!(method.call(&account, (2.0, 10.0)), 210.0);
}

#[test]
fn test_method_expression_with_case() {
	// Test method expression with CASE statement
	let method: HybridMethod<Account, f64, String> = HybridMethod::new(
		|_acc: &Account, _threshold: f64| "result".to_string(),
	)
	.with_expression(|threshold: f64| {
		format!(
			"CASE WHEN balance > {} THEN 'high' ELSE 'low' END",
			threshold
		)
	});

	let expr = method.expression(100.0).unwrap();
	assert!(expr.contains("CASE"));
	assert!(expr.contains("100"));
}

#[test]
fn test_multiple_methods() {
	// Test multiple methods on the same type
	let account = Account::new(1, 100.0, "USD".to_string());

	let method1 = HybridMethod::new(|acc: &Account, x: f64| acc.balance + x);
	let method2 = HybridMethod::new(|acc: &Account, x: f64| acc.balance * x);

	assert_eq!(method1.call(&account, 50.0), 150.0);
	assert_eq!(method2.call(&account, 2.0), 200.0);
}

#[test]
fn test_method_with_negative_number() {
	// Test method with negative numbers
	let account = Account::new(1, 100.0, "USD".to_string());

	let method = HybridMethod::new(|acc: &Account, amount: f64| acc.balance + amount);

	assert_eq!(method.call(&account, -30.0), 70.0);
}
