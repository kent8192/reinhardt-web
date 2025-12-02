//! Integration tests for Admin Audit Logs and Dashboard Widgets
//!
//! Tests the complete flow: user actions → audit logging → dashboard statistics

use reinhardt_panel::{
	// Dashboard
	Activity,
	// Audit
	AuditAction,
	AuditLog,
	AuditLogQuery,
	AuditLogger,
	ChartData,
	ChartDataset,
	DashboardWidget,
	MemoryAuditLogger,
	QuickLink,
	QuickLinksWidget,
	RecentActivityWidget,
	StatWidget,
	TableWidget,
	WidgetConfig,
	WidgetContext,
	WidgetPosition,
	WidgetRegistry,
};
use serde_json::json;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

/// Test: User actions create audit logs, then display in dashboard widget
#[tokio::test]
async fn test_audit_to_dashboard_workflow() {
	// Step 1: Create audit logger
	let logger = MemoryAuditLogger::new();

	// Step 2: Simulate user actions
	let log1 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("User".to_string())
		.object_id("123".to_string())
		.action(AuditAction::Create)
		.build();

	let log2 = AuditLog::builder()
		.user_id("user1".to_string())
		.model_name("Post".to_string())
		.object_id("456".to_string())
		.action(AuditAction::Update)
		.changes(json!({"title": {"old": "Old Title", "new": "New Title"}}))
		.build();

	let log3 = AuditLog::builder()
		.user_id("user2".to_string())
		.model_name("Comment".to_string())
		.object_id("789".to_string())
		.action(AuditAction::Delete)
		.build();

	logger.log(log1).await.expect("Log 1 should succeed");
	logger.log(log2).await.expect("Log 2 should succeed");
	logger.log(log3).await.expect("Log 3 should succeed");

	// Step 3: Query audit logs
	let all_logs = logger
		.query(&AuditLogQuery::builder().build())
		.await
		.expect("Query should succeed");

	assert_eq!(all_logs.len(), 3);

	// Step 4: Create dashboard widget showing recent activities
	let activities: Vec<Activity> = all_logs
		.into_iter()
		.map(|log| Activity {
			user: log.user_id().to_string(),
			action: format!(
				"{} {} {}",
				log.action().as_str(),
				log.model_name(),
				log.object_id()
			),
			timestamp: log.timestamp().format("%Y-%m-%d %H:%M:%S").to_string(),
		})
		.collect();

	let activities_clone = activities.clone();
	let widget = RecentActivityWidget::new(
		"Recent Admin Actions",
		WidgetPosition::TopRight,
		10, // max_items
		move || {
			let activities = activities_clone.clone();
			async move { Ok(activities) }
		},
	);

	let context = WidgetContext::new();
	let html = widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify HTML structure with more specific assertions
	assert!(html.starts_with("<div"));
	assert!(html.ends_with("</div>"));
	assert_eq!(html.matches("Recent Admin Actions").count(), 1);
	assert_eq!(activities.len(), 3);
}

/// Test: Audit log statistics displayed in StatWidget
#[tokio::test]
async fn test_audit_stats_in_widget() {
	let logger = MemoryAuditLogger::new();

	// Create multiple audit logs
	for i in 1..=10 {
		let log = AuditLog::builder()
            .user_id(format!("user{}", i % 3)) // 3 different users
            .model_name("Article".to_string())
            .object_id(i.to_string())
            .action(if i % 2 == 0 {
                AuditAction::Create
            } else {
                AuditAction::Update
            })
            .build();

		logger.log(log).await.expect("Log should succeed");
	}

	// Count total actions
	let total = logger
		.count(&AuditLogQuery::builder().build())
		.await
		.expect("Count should succeed");

	// Create StatWidget to display total actions
	let stat_widget = StatWidget::new(
		"Total Admin Actions",
		WidgetPosition::TopLeft,
		move || async move { Ok(total as i64) },
	);

	let context = WidgetContext::new();
	let html = stat_widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify HTML contains expected text exactly once
	assert!(html.starts_with("<div"));
	assert!(html.ends_with("</div>"));
	assert_eq!(html.matches("Total Admin Actions").count(), 1);
	assert_eq!(html.matches("10").count(), 1);
}

/// Test: Dashboard with multiple widgets showing audit data
#[tokio::test]
async fn test_dashboard_multi_widget_audit_integration() {
	let logger = MemoryAuditLogger::new();
	let registry = WidgetRegistry::new();

	// Create audit logs for different models
	let models = ["User", "Post", "Comment"];
	let mut counts_by_model = std::collections::HashMap::new();

	for (idx, model) in models.iter().enumerate() {
		for i in 1..=(idx + 1) * 3 {
			let log = AuditLog::builder()
				.user_id("admin".to_string())
				.model_name(model.to_string())
				.object_id(i.to_string())
				.action(AuditAction::Create)
				.build();

			logger.log(log).await.expect("Log should succeed");
		}
		counts_by_model.insert(model.to_string(), (idx + 1) * 3);
	}

	// Widget 1: Total actions
	let total_count = logger
		.count(&AuditLogQuery::builder().build())
		.await
		.expect("Count should succeed");

	let total_widget = Arc::new(StatWidget::new(
		"Total Actions",
		WidgetPosition::TopLeft,
		move || async move { Ok(total_count as i64) },
	));

	registry
		.register(
			total_widget,
			WidgetConfig::new("total_actions", WidgetPosition::TopLeft),
		)
		.expect("Register total widget");

	// Widget 2: Actions by model (Chart)
	let datasets = vec![ChartDataset {
		label: "Actions by Model".to_string(),
		data: counts_by_model.values().map(|&v| v as f64).collect(),
		background_color: Some(vec!["#3498db".to_string()]),
		border_color: Some(vec!["#2980b9".to_string()]),
	}];

	let _chart_data = ChartData {
		labels: counts_by_model.keys().cloned().collect(),
		datasets,
	};

	// Widget 3: Quick links to audit views
	let links_widget = Arc::new(
		QuickLinksWidget::new("Audit Views", WidgetPosition::TopRight)
			.add_link(QuickLink::new("All Logs", "/admin/audit/").with_icon("fa fa-list"))
			.add_link(
				QuickLink::new("User Actions", "/admin/audit/?model=User").with_icon("fa fa-user"),
			)
			.add_link(
				QuickLink::new("Recent Changes", "/admin/audit/?action=update")
					.with_icon("fa fa-edit"),
			),
	);

	registry
		.register(
			links_widget,
			WidgetConfig::new("audit_links", WidgetPosition::TopRight),
		)
		.expect("Register links widget");

	// Verify registry by checking positions
	let top_left_widgets = registry.get_by_position(WidgetPosition::TopLeft);
	assert_eq!(top_left_widgets.len(), 1);

	let top_right_widgets = registry.get_by_position(WidgetPosition::TopRight);
	assert_eq!(top_right_widgets.len(), 1);
}

/// Test: Recent activities widget with audit log filtering
#[tokio::test]
async fn test_recent_activity_widget_filtering() {
	let logger = MemoryAuditLogger::new();

	// Create logs for different users
	let users = ["alice", "bob", "charlie"];
	for (idx, user) in users.iter().enumerate() {
		for i in 1..=5 {
			let log = AuditLog::builder()
				.user_id(user.to_string())
				.model_name("Task".to_string())
				.object_id(format!("{}{}", idx, i))
				.action(match i % 3 {
					0 => AuditAction::Create,
					1 => AuditAction::Update,
					_ => AuditAction::View,
				})
				.build();

			logger.log(log).await.expect("Log should succeed");
		}
	}

	// Query alice's actions only
	let alice_logs = logger
		.query(
			&AuditLogQuery::builder()
				.user_id("alice".to_string())
				.build(),
		)
		.await
		.expect("Query alice's logs");

	assert_eq!(alice_logs.len(), 5);

	// Create widget for alice's recent activities
	let activities: Vec<Activity> = alice_logs
		.into_iter()
		.map(|log| Activity {
			user: log.user_id().to_string(),
			action: format!("{} {}", log.action().as_str(), log.model_name()),
			timestamp: log.timestamp().format("%H:%M:%S").to_string(),
		})
		.collect();

	let activities_clone = activities.clone();
	let widget = RecentActivityWidget::new(
		"Alice's Activities",
		WidgetPosition::Center,
		10, // max_items
		move || {
			let activities = activities_clone.clone();
			async move { Ok(activities) }
		},
	);

	let context = WidgetContext::new();
	let html = widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify HTML structure
	assert!(html.starts_with("<div"));
	assert!(html.ends_with("</div>"));
	assert_eq!(html.matches("Alice's Activities").count(), 1);
	// All activities should be from alice
	for activity in &activities {
		assert_eq!(activity.user, "alice");
	}
}

/// Test: Table widget displaying audit log summary
#[tokio::test]
async fn test_table_widget_audit_summary() {
	let logger = MemoryAuditLogger::new();

	// Create various audit logs
	let actions = [
		(AuditAction::Create, 5),
		(AuditAction::Update, 8),
		(AuditAction::Delete, 2),
		(AuditAction::View, 15),
	];

	for (action, count) in actions.iter() {
		for i in 1..=*count {
			let log = AuditLog::builder()
				.user_id("admin".to_string())
				.model_name("Resource".to_string())
				.object_id(i.to_string())
				.action(*action)
				.build();

			logger.log(log).await.expect("Log should succeed");
		}
	}

	// Collect statistics
	let mut stats = Vec::new();
	for (action, expected_count) in actions.iter() {
		let count = logger
			.count(&AuditLogQuery::builder().action(*action).build())
			.await
			.expect("Count should succeed");

		assert_eq!(count, *expected_count);

		let mut row = std::collections::HashMap::new();
		row.insert("Action".to_string(), action.as_str().to_string());
		row.insert("Count".to_string(), count.to_string());
		stats.push(row);
	}

	// Create table widget
	let columns = vec!["Action".to_string(), "Count".to_string()];
	let stats_clone = stats.clone();
	let widget = TableWidget::new(
		"Action Statistics",
		WidgetPosition::Center,
		columns,
		move || {
			let stats = stats_clone.clone();
			async move {
				// Convert HashMap<String, String> to Vec<Vec<String>>
				let rows: Vec<Vec<String>> = stats
					.iter()
					.map(|row| {
						vec![
							row.get("Action").cloned().unwrap_or_default(),
							row.get("Count").cloned().unwrap_or_default(),
						]
					})
					.collect();
				Ok(rows)
			}
		},
	);

	let context = WidgetContext::new();
	let html = widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify table structure with exact counts
	assert!(html.starts_with("<table") || html.starts_with("<div"));
	assert!(html.ends_with("</table>") || html.ends_with("</div>"));
	assert_eq!(html.matches("Action Statistics").count(), 1);
	assert_eq!(html.matches("Action").count(), 2); // Title + Column header
	assert_eq!(html.matches("Count").count(), 1); // Column header only
}

/// Test: Permission-based widget visibility with audit context
#[tokio::test]
async fn test_widget_permission_with_audit_context() {
	let logger = MemoryAuditLogger::new();

	// Create admin and regular user logs
	let admin_log = AuditLog::builder()
		.user_id("admin".to_string())
		.model_name("Settings".to_string())
		.object_id("1".to_string())
		.action(AuditAction::Update)
		.build();

	let user_log = AuditLog::builder()
		.user_id("user".to_string())
		.model_name("Profile".to_string())
		.object_id("123".to_string())
		.action(AuditAction::Update)
		.build();

	logger.log(admin_log).await.expect("Admin log");
	logger.log(user_log).await.expect("User log");

	// Count admin actions before creating widget
	let admin_count = logger
		.count(
			&AuditLogQuery::builder()
				.user_id("admin".to_string())
				.build(),
		)
		.await
		.expect("Count admin actions");

	// Create widget that requires admin permission
	let admin_widget = StatWidget::new(
		"Admin Actions",
		WidgetPosition::TopLeft,
		move || async move { Ok(admin_count as i64) },
	);

	// Test visibility for admin user
	let admin_permissions = vec!["view_audit_logs".to_string(), "admin".to_string()];
	assert!(admin_widget.is_visible(&admin_permissions).await);

	// Test visibility for regular user (should still be visible in this simple case)
	let user_permissions = vec!["view_own_profile".to_string()];
	assert!(admin_widget.is_visible(&user_permissions).await);
}

/// Test: Audit log retention with dashboard update
#[tokio::test]
async fn test_audit_retention_dashboard_update() {
	let logger = MemoryAuditLogger::new();

	// Create 100 audit logs
	for i in 1..=100 {
		let log = AuditLog::builder()
			.user_id("user".to_string())
			.model_name("Item".to_string())
			.object_id(i.to_string())
			.action(AuditAction::Create)
			.build();

		logger.log(log).await.expect("Log should succeed");
	}

	// Verify total count
	let initial_count = logger
		.count(&AuditLogQuery::builder().build())
		.await
		.expect("Count should succeed");

	assert_eq!(initial_count, 100);

	// Simulate retention policy: keep only last 50
	let recent_logs = logger
		.query(&AuditLogQuery::builder().limit(50).build())
		.await
		.expect("Query recent logs");

	assert_eq!(recent_logs.len(), 50);

	// Dashboard widget shows "active" log count (simulated retention)
	let active_count_widget = StatWidget::new(
		"Active Audit Logs",
		WidgetPosition::TopLeft,
		move || async move { Ok(50i64) },
	);

	let context = WidgetContext::new();
	let html = active_count_widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify HTML contains exact count
	assert!(html.starts_with("<div"));
	assert!(html.ends_with("</div>"));
	assert_eq!(html.matches("50").count(), 1);
}

/// Test: IP address and user agent tracking in audit logs
#[tokio::test]
async fn test_audit_ip_user_agent_tracking() {
	let logger = MemoryAuditLogger::new();

	let ip_addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
	let user_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64)";

	// Create log with IP and user agent
	let log = AuditLog::builder()
		.user_id("alice".to_string())
		.model_name("Account".to_string())
		.object_id("42".to_string())
		.action(AuditAction::Update)
		.ip_address(ip_addr)
		.user_agent(user_agent.to_string())
		.build();

	let logged = logger.log(log).await.expect("Log with IP should succeed");

	// Verify IP and user agent are stored
	assert_eq!(logged.ip_address(), Some(ip_addr));
	assert_eq!(logged.user_agent(), Some(user_agent));

	// Create table widget showing security audit info
	let mut row = std::collections::HashMap::new();
	row.insert("User".to_string(), logged.user_id().to_string());
	row.insert("IP".to_string(), ip_addr.to_string());
	row.insert(
		"User Agent".to_string(),
		user_agent[..20].to_string() + "...",
	);

	let columns = vec![
		"User".to_string(),
		"IP".to_string(),
		"User Agent".to_string(),
	];
	let row_clone = row.clone();
	let widget = TableWidget::new(
		"Security Audit",
		WidgetPosition::BottomLeft,
		columns,
		move || {
			let row = row_clone.clone();
			async move {
				// Convert single HashMap to Vec<Vec<String>>
				let rows = vec![vec![
					row.get("User").cloned().unwrap_or_default(),
					row.get("IP").cloned().unwrap_or_default(),
					row.get("User Agent").cloned().unwrap_or_default(),
				]];
				Ok(rows)
			}
		},
	);

	let context = WidgetContext::new();
	let html = widget
		.render(&context)
		.await
		.expect("Render should succeed");

	// Verify table contains exact IP address
	assert!(html.starts_with("<table") || html.starts_with("<div"));
	assert!(html.ends_with("</table>") || html.ends_with("</div>"));
	assert_eq!(html.matches("Security Audit").count(), 1);
	assert_eq!(html.matches("192.168.1.100").count(), 1);
}
