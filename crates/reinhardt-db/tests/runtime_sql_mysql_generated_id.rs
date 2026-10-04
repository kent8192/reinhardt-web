//! Transaction insert IDs use typed expressions on the same native connection.
#![cfg(feature = "mysql")]

use reinhardt_db::backends::{DatabaseConnection, DatabaseErrorKind};
use reinhardt_db::orm::model::FieldSelector;
use reinhardt_db::orm::{Manager, Model};
use reinhardt_query::{Expr, Func, MySqlQueryBuilder, Query};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use testcontainers::{ImageExt, runners::AsyncRunner};

#[derive(Clone)]
struct Fields;
impl FieldSelector for Fields {
	fn with_alias(self, _alias: &str) -> Self {
		self
	}
}

macro_rules! id_model {
	($name:ident, $table:literal) => {
		#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
		struct $name {
			id: Option<i64>,
			name: String,
		}
		impl Model for $name {
			type PrimaryKey = i64;
			type Fields = Fields;
			type Objects = Manager<Self>;
			fn table_name() -> &'static str {
				$table
			}
			fn new_fields() -> Self::Fields {
				Fields
			}
			fn primary_key(&self) -> Option<i64> {
				self.id
			}
			fn set_primary_key(&mut self, value: i64) {
				self.id = Some(value);
			}
		}
	};
}
id_model!(Generated, "generated_ids");
id_model!(Missing, "missing_ids");

#[rstest]
#[tokio::test]
async fn mysql_transaction_ids_reset_read_and_reload_on_one_executor() {
	// Arrange: RAII owns the isolated native database fixture.
	let container = testcontainers_modules::mysql::Mysql::default()
		.with_startup_timeout(std::time::Duration::from_secs(180))
		.start()
		.await
		.unwrap();
	let port = container.get_host_port_ipv4(3306).await.unwrap();
	let connection =
		DatabaseConnection::connect_mysql(&format!("mysql://root@127.0.0.1:{port}/test"))
			.await
			.unwrap();
	connection
		.execute(
			"CREATE TABLE generated_ids (id BIGINT PRIMARY KEY AUTO_INCREMENT, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	connection
		.execute(
			"CREATE TABLE missing_ids (id BIGINT PRIMARY KEY DEFAULT 0, name TEXT)",
			vec![],
		)
		.await
		.unwrap();
	let mut transaction = connection.begin_write().await.unwrap();
	let seed = Query::select()
		.expr(Func::mysql_last_insert_id(Some(
			Expr::val(900_i64).into_simple_expr(),
		)))
		.to_owned();
	transaction
		.fetch_one_generated(MySqlQueryBuilder.build_select_checked(&seed).unwrap(), None)
		.await
		.unwrap();
	// Act: each generated insert resets state, then reloads using its own new ID.
	let generated = Manager::<Generated>::new()
		.insert_with_executor(
			&mut *transaction,
			&Generated {
				id: None,
				name: "bound' ? $55".into(),
			},
		)
		.await
		.unwrap();
	// Assert
	assert_eq!(
		generated,
		Generated {
			id: Some(1),
			name: "bound' ? $55".into()
		}
	);
	let explicit = Generated {
		id: Some(77),
		name: "explicit".into(),
	};
	assert_eq!(
		Manager::<Generated>::new()
			.insert_with_executor(&mut *transaction, &explicit)
			.await
			.unwrap(),
		explicit
	);
	// A non-generating insert cannot reuse connection state left by an earlier write.
	transaction
		.fetch_one_generated(MySqlQueryBuilder.build_select_checked(&seed).unwrap(), None)
		.await
		.unwrap();
	let error = Manager::<Missing>::new()
		.insert_with_executor(
			&mut *transaction,
			&Missing {
				id: None,
				name: "no generated ID".into(),
			},
		)
		.await
		.unwrap_err();
	assert_eq!(error.kind(), DatabaseErrorKind::Unsupported);
	assert!(
		error
			.to_string()
			.contains("auto-increment integer primary key")
	);
	transaction.rollback().await.unwrap();
	assert_eq!(
		connection
			.fetch_one("SELECT COUNT(*) AS remaining FROM generated_ids", vec![])
			.await
			.unwrap()
			.get::<i64>("remaining")
			.unwrap(),
		0
	);
}
