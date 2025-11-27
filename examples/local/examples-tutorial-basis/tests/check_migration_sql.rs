#[cfg(test)]
mod tests {
	use examples_tutorial_basis::apps::polls::migrations::Migration0001Initial;

	#[test]
	fn print_migration_sql() {
		let sql = Migration0001Initial::up();
		eprintln!("=== Generated SQL ===");
		eprintln!("{}", sql);
		eprintln!("\n=== SQL Analysis ===");
		eprintln!("SQL Length: {}", sql.len());
		eprintln!("Contains semicolons: {}", sql.matches(';').count());
		eprintln!("Contains newlines: {}", sql.matches('\n').count());

		// Split by semicolon to see individual statements
		let statements: Vec<&str> = sql.split(';').collect();
		eprintln!("\n=== Statements Count: {} ===", statements.len());
		for (i, stmt) in statements.iter().enumerate() {
			if !stmt.trim().is_empty() {
				eprintln!("\n--- Statement {} ---", i + 1);
				eprintln!("{}", stmt.trim());
			}
		}
	}
}
