//! Integration tests for reinhardt-mobile.
//!
//! Tests the integration between various components:
//! - MobileConfig and platform detection
//! - IRVisitor with ComponentIR
//! - IPC code generation
//! - Platform initialization

use proc_macro2::Span;
use reinhardt_manouche::codegen::IRVisitor;
use reinhardt_manouche::ir::{
	AttrValueIR, AttributeIR, ComponentCallIR, ComponentIR, ConditionalIR, ElementIR, EventIR,
	ExprIR, FieldIR, FieldTypeIR, FormActionIR, FormIR, FormMethodIR, FormStylingIR, HeadElementIR,
	HeadIR, LoopIR, NodeIR, PropIR, TextIR, TitleIR, WatchIR, WidgetIR, WidgetTypeIR,
};
use reinhardt_mobile::{
	AndroidPlatform, BuildConfig, IosPlatform, IpcBridge, IpcCodeGenerator, IpcCommandDef,
	IpcRequest, IpcResponse, MobileConfig, MobileError, MobilePlatform, MobileVisitor,
	SecurityConfig, TargetPlatform,
};
use rstest::rstest;

// =============================================================================
// MobileConfig Tests
// =============================================================================

mod config_tests {
	use super::*;

	#[rstest]
	fn test_mobile_config_default_creates_valid_config() {
		// Arrange
		// No setup needed for default config

		// Act
		let config = MobileConfig::default();

		// Assert
		assert_eq!(config.app_name, "ReinhardtApp");
		assert_eq!(config.app_id, "com.example.reinhardt");
		assert_eq!(config.version, "1.0.0");
		assert_eq!(config.platform, TargetPlatform::Android);
		assert!(!config.build.release);
		assert_eq!(config.build.min_api_level, 26);
		assert!(!config.security.allow_remote_navigation);
	}

	#[rstest]
	fn test_mobile_config_with_custom_values() {
		// Arrange
		let custom_name = "MyMobileApp";
		let custom_id = "com.mycompany.myapp";
		let custom_version = "2.0.0";

		// Act
		let config = MobileConfig {
			app_name: custom_name.to_string(),
			app_id: custom_id.to_string(),
			version: custom_version.to_string(),
			platform: TargetPlatform::Ios,
			build: BuildConfig {
				release: true,
				min_api_level: 13,
				target_api_level: None,
				experimental: false,
			},
			security: SecurityConfig {
				allow_remote_navigation: true,
				allowed_ipc_origins: vec!["https://example.com".to_string()],
				protocol_scheme: "myapp".to_string(),
			},
		};

		// Assert
		assert_eq!(config.app_name, custom_name);
		assert_eq!(config.app_id, custom_id);
		assert_eq!(config.version, custom_version);
		assert_eq!(config.platform, TargetPlatform::Ios);
		assert!(config.build.release);
		assert_eq!(config.build.min_api_level, 13);
		assert!(config.build.target_api_level.is_none());
		assert!(config.security.allow_remote_navigation);
		assert_eq!(config.security.allowed_ipc_origins.len(), 1);
	}

	#[rstest]
	#[case(TargetPlatform::Android)]
	#[case(TargetPlatform::Ios)]
	fn test_target_platform_variants(#[case] platform: TargetPlatform) {
		// Arrange
		let config = MobileConfig {
			platform,
			..MobileConfig::default()
		};

		// Act
		let is_android = config.platform == TargetPlatform::Android;
		let is_ios = config.platform == TargetPlatform::Ios;

		// Assert
		match platform {
			TargetPlatform::Android => {
				assert!(is_android);
				assert!(!is_ios);
			}
			TargetPlatform::Ios => {
				assert!(!is_android);
				assert!(is_ios);
			}
		}
	}

	#[rstest]
	fn test_build_config_default_values() {
		// Arrange
		// No setup needed

		// Act
		let build_config = BuildConfig::default();

		// Assert
		assert!(!build_config.release);
		assert_eq!(build_config.min_api_level, 26);
		assert_eq!(build_config.target_api_level, Some(33));
		assert!(build_config.experimental);
	}

	#[rstest]
	fn test_security_config_default_values() {
		// Arrange
		// No setup needed

		// Act
		let security_config = SecurityConfig::default();

		// Assert
		assert!(!security_config.allow_remote_navigation);
		assert!(security_config.allowed_ipc_origins.is_empty());
		assert_eq!(security_config.protocol_scheme, "reinhardt");
	}
}

// =============================================================================
// Platform Tests
// =============================================================================

mod platform_tests {
	use super::*;

	#[rstest]
	fn test_android_platform_creation() {
		// Arrange
		// No setup needed

		// Act
		let platform = AndroidPlatform::new();

		// Assert
		assert_eq!(platform.name(), "android");
		assert_eq!(platform.protocol_scheme(), "http://wry");
	}

	#[rstest]
	fn test_ios_platform_creation() {
		// Arrange
		// No setup needed

		// Act
		let platform = IosPlatform::new();

		// Assert
		assert_eq!(platform.name(), "ios");
		assert_eq!(platform.protocol_scheme(), "wry");
	}

	#[rstest]
	fn test_android_platform_initialization() {
		// Arrange
		let mut platform = AndroidPlatform::new();
		let config = MobileConfig::default();

		// Act
		let result = platform.initialize(&config);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_ios_platform_initialization() {
		// Arrange
		let mut platform = IosPlatform::new();
		let config = MobileConfig {
			platform: TargetPlatform::Ios,
			..MobileConfig::default()
		};

		// Act
		let result = platform.initialize(&config);

		// Assert
		assert!(result.is_ok());
	}

	#[rstest]
	fn test_android_platform_is_available() {
		// Arrange
		// No setup needed

		// Act
		let is_available = AndroidPlatform::is_available();

		// Assert
		// On non-Android platforms, this should be false
		#[cfg(not(target_os = "android"))]
		assert!(!is_available);

		#[cfg(target_os = "android")]
		assert!(is_available);
	}

	#[rstest]
	fn test_ios_platform_is_available() {
		// Arrange
		// No setup needed

		// Act
		let is_available = IosPlatform::is_available();

		// Assert
		// On non-iOS platforms, this should be false
		#[cfg(not(target_os = "ios"))]
		assert!(!is_available);

		#[cfg(target_os = "ios")]
		assert!(is_available);
	}

	#[rstest]
	fn test_platform_default_implementation() {
		// Arrange
		// No setup needed

		// Act
		let android = AndroidPlatform::default();
		let ios = IosPlatform::default();

		// Assert
		assert_eq!(android.name(), "android");
		assert_eq!(ios.name(), "ios");
	}
}

// =============================================================================
// MobileVisitor Tests
// =============================================================================

mod visitor_tests {
	use super::*;

	fn create_simple_element_ir() -> ElementIR {
		ElementIR {
			tag: "div".to_string(),
			attributes: vec![],
			events: vec![],
			children: vec![],
			span: Span::call_site(),
		}
	}

	fn create_text_ir(content: &str) -> TextIR {
		TextIR {
			content: content.to_string(),
			span: Span::call_site(),
		}
	}

	fn create_element_with_children() -> ElementIR {
		ElementIR {
			tag: "div".to_string(),
			attributes: vec![AttributeIR {
				name: "class".to_string(),
				value: AttrValueIR::Static("container".to_string()),
				span: Span::call_site(),
			}],
			events: vec![],
			children: vec![
				NodeIR::Text(create_text_ir("Hello, ")),
				NodeIR::Element(ElementIR {
					tag: "span".to_string(),
					attributes: vec![],
					events: vec![],
					children: vec![NodeIR::Text(create_text_ir("World!"))],
					span: Span::call_site(),
				}),
			],
			span: Span::call_site(),
		}
	}

	#[rstest]
	fn test_mobile_visitor_creation() {
		// Arrange
		let config = MobileConfig::default();

		// Act
		let visitor = MobileVisitor::new(config);

		// Assert
		assert!(visitor.html().is_empty());
		assert!(visitor.js().is_empty());
	}

	#[rstest]
	fn test_mobile_visitor_reset() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);

		// Act
		visitor.reset();

		// Assert
		assert!(visitor.html().is_empty());
		assert!(visitor.js().is_empty());
	}

	#[rstest]
	fn test_visit_simple_element() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let element = create_simple_element_ir();

		// Act
		let output = visitor.visit_element(&element);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileElement"));
		assert!(output_str.contains("\"div\""));
	}

	#[rstest]
	fn test_visit_text_node() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let text = create_text_ir("Hello, World!");

		// Act
		let output = visitor.visit_text(&text);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileText"));
		assert!(output_str.contains("Hello, World!"));
	}

	#[rstest]
	fn test_visit_element_with_children() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let element = create_element_with_children();

		// Act
		let output = visitor.visit_element(&element);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileElement"));
		assert!(output_str.contains("\"div\""));
		assert!(output_str.contains("child"));
	}

	#[rstest]
	fn test_visit_component_ir() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let component = ComponentIR {
			props: vec![PropIR {
				name: "title".to_string(),
				ty: "String".to_string(),
				span: Span::call_site(),
			}],
			body: vec![NodeIR::Element(create_simple_element_ir())],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_component(&component);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileElement"));
	}

	#[rstest]
	fn test_visit_prop_ir() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let prop = PropIR {
			name: "count".to_string(),
			ty: "i32".to_string(),
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_prop(&prop);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("count"));
		assert!(output_str.contains("i32"));
	}

	#[rstest]
	fn test_visit_fragment() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let fragment = vec![
			NodeIR::Text(create_text_ir("First")),
			NodeIR::Text(create_text_ir("Second")),
		];

		// Act
		let output = visitor.visit_fragment(&fragment);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileText"));
	}

	#[rstest]
	fn test_visit_component_call() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let component_call = ComponentCallIR {
			name: "MyComponent".to_string(),
			args: vec![],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_component_call(&component_call);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MyComponent"));
	}

	#[rstest]
	fn test_visit_conditional() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let condition: syn::Expr = syn::parse_quote!(true);
		let conditional = ConditionalIR {
			condition,
			then_body: vec![NodeIR::Text(create_text_ir("Visible"))],
			else_branch: None,
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_conditional(&conditional);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("if"));
		assert!(output_str.contains("true"));
	}

	#[rstest]
	fn test_visit_loop() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let iterator: syn::Expr = syn::parse_quote!(items);
		let loop_ir = LoopIR {
			pattern: "item".to_string(),
			iterator,
			body: vec![NodeIR::Text(create_text_ir("Item"))],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_loop(&loop_ir);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("for"));
		assert!(output_str.contains("items"));
	}

	#[rstest]
	fn test_visit_expression() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let expr: syn::Expr = syn::parse_quote!(count + 1);
		let expr_ir = ExprIR {
			expr,
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_expression(&expr_ir);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileExpr"));
	}

	#[rstest]
	fn test_visit_watch() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let watch = WatchIR {
			dependencies: vec![],
			body: vec![NodeIR::Text(create_text_ir("Watched"))],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_watch(&watch);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("watch"));
	}

	#[rstest]
	fn test_visit_event() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let handler: syn::Expr = syn::parse_quote!(handle_click);
		let event = EventIR {
			name: "click".to_string(),
			handler,
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_event(&event);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("click"));
		assert!(output_str.contains("handle_click"));
	}

	#[rstest]
	fn test_visit_attribute() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let attr = AttributeIR {
			name: "id".to_string(),
			value: AttrValueIR::Static("main".to_string()),
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_attribute(&attr);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("id"));
	}

	#[rstest]
	fn test_visit_form() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let form = FormIR {
			name: "login_form".to_string(),
			action: FormActionIR::Url("/login".to_string()),
			method: FormMethodIR::Post,
			fields: vec![FieldIR {
				name: "username".to_string(),
				field_type: FieldTypeIR::CharField,
				label: Some("Username".to_string()),
				required: true,
				validators: vec![],
				widget: WidgetIR {
					widget_type: WidgetTypeIR::TextInput,
					attrs: vec![],
				},
				span: Span::call_site(),
			}],
			styling: FormStylingIR {
				class: None,
				attrs: vec![],
			},
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_form(&form);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileForm"));
		assert!(output_str.contains("login_form"));
	}

	#[rstest]
	fn test_visit_field() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let field = FieldIR {
			name: "email".to_string(),
			field_type: FieldTypeIR::EmailField,
			label: Some("Email".to_string()),
			required: true,
			validators: vec![],
			widget: WidgetIR {
				widget_type: WidgetTypeIR::TextInput,
				attrs: vec![],
			},
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_field(&field);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileField"));
		assert!(output_str.contains("email"));
	}

	#[rstest]
	fn test_visit_head() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let head = HeadIR {
			elements: vec![HeadElementIR::Title(TitleIR {
				content: "My App".to_string(),
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_head(&head);

		// Assert
		// Head visitor should produce some output
		let output_str = output.to_string();
		assert!(!output_str.is_empty() || output_str == "()");
	}

	#[rstest]
	fn test_visit_head_element() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);
		let head_element = HeadElementIR::Title(TitleIR {
			content: "Page Title".to_string(),
			span: Span::call_site(),
		});

		// Act
		let output = visitor.visit_head_element(&head_element);

		// Assert
		// Currently returns unit, but should not panic
		let output_str = output.to_string();
		assert!(!output_str.is_empty());
	}
}

// =============================================================================
// IPC Code Generator Tests
// =============================================================================

mod ipc_codegen_tests {
	use super::*;

	#[rstest]
	fn test_ipc_code_generator_creation() {
		// Arrange
		// No setup needed

		// Act
		let generator = IpcCodeGenerator::new();

		// Assert
		// Generator should be created without commands
		let js = generator.generate_js_bindings();
		assert!(js.contains("__REINHARDT_COMMANDS__"));
	}

	#[rstest]
	fn test_ipc_code_generator_default() {
		// Arrange
		// No setup needed

		// Act
		let generator = IpcCodeGenerator::default();

		// Assert
		let js = generator.generate_js_bindings();
		assert!(js.contains("// Generated IPC bindings"));
	}

	#[rstest]
	fn test_add_single_command() {
		// Arrange
		let mut generator = IpcCodeGenerator::new();
		let command = IpcCommandDef {
			name: "greet".to_string(),
			params: vec!["name".to_string()],
			return_type: Some("String".to_string()),
		};

		// Act
		generator.add_command(command);
		let js = generator.generate_js_bindings();

		// Assert
		assert!(js.contains("greet"));
		assert!(js.contains("name"));
		assert!(js.contains("__REINHARDT_IPC__"));
	}

	#[rstest]
	fn test_add_multiple_commands() {
		// Arrange
		let mut generator = IpcCodeGenerator::new();
		let commands = vec![
			IpcCommandDef {
				name: "get_user".to_string(),
				params: vec!["user_id".to_string()],
				return_type: Some("User".to_string()),
			},
			IpcCommandDef {
				name: "update_user".to_string(),
				params: vec!["user_id".to_string(), "data".to_string()],
				return_type: Some("bool".to_string()),
			},
			IpcCommandDef {
				name: "delete_user".to_string(),
				params: vec!["user_id".to_string()],
				return_type: None,
			},
		];

		// Act
		for cmd in commands {
			generator.add_command(cmd);
		}
		let js = generator.generate_js_bindings();

		// Assert
		assert!(js.contains("get_user"));
		assert!(js.contains("update_user"));
		assert!(js.contains("delete_user"));
		assert!(js.contains("user_id"));
		assert!(js.contains("data"));
	}

	#[rstest]
	fn test_generate_handlers() {
		// Arrange
		let mut generator = IpcCodeGenerator::new();
		generator.add_command(IpcCommandDef {
			name: "test_command".to_string(),
			params: vec![],
			return_type: None,
		});

		// Act
		let handlers = generator.generate_handlers();

		// Assert
		let handlers_str = handlers.to_string();
		assert!(handlers_str.contains("register_ipc_handlers"));
		assert!(handlers_str.contains("test_command"));
		assert!(handlers_str.contains("bridge"));
	}

	#[rstest]
	fn test_command_with_no_params() {
		// Arrange
		let mut generator = IpcCodeGenerator::new();
		let command = IpcCommandDef {
			name: "ping".to_string(),
			params: vec![],
			return_type: Some("String".to_string()),
		};

		// Act
		generator.add_command(command);
		let js = generator.generate_js_bindings();

		// Assert
		assert!(js.contains("ping: function()"));
	}

	#[rstest]
	fn test_command_with_multiple_params() {
		// Arrange
		let mut generator = IpcCodeGenerator::new();
		let command = IpcCommandDef {
			name: "calculate".to_string(),
			params: vec!["a".to_string(), "b".to_string(), "c".to_string()],
			return_type: Some("i32".to_string()),
		};

		// Act
		generator.add_command(command);
		let js = generator.generate_js_bindings();

		// Assert
		assert!(js.contains("calculate: function(a, b, c)"));
		assert!(js.contains("a: a"));
		assert!(js.contains("b: b"));
		assert!(js.contains("c: c"));
	}
}

// =============================================================================
// IPC Bridge Runtime Tests
// =============================================================================

mod ipc_bridge_tests {
	use super::*;

	#[rstest]
	fn test_ipc_bridge_creation() {
		// Arrange
		// No setup needed

		// Act
		let bridge = IpcBridge::new();

		// Assert
		assert!(bridge.commands().is_empty());
	}

	#[rstest]
	fn test_ipc_bridge_default() {
		// Arrange
		// No setup needed

		// Act
		let bridge = IpcBridge::default();

		// Assert
		assert!(bridge.commands().is_empty());
	}

	#[rstest]
	fn test_register_command() {
		// Arrange
		let mut bridge = IpcBridge::new();

		// Act
		bridge.register("test", |_| Ok(serde_json::json!({"status": "ok"})));

		// Assert
		assert!(bridge.has_command("test"));
		assert_eq!(bridge.commands().len(), 1);
	}

	#[rstest]
	fn test_handle_registered_command() {
		// Arrange
		let mut bridge = IpcBridge::new();
		bridge.register("echo", |req| {
			let message = req
				.payload
				.get("message")
				.and_then(|v| v.as_str())
				.unwrap_or("default");
			Ok(serde_json::json!({"echo": message}))
		});

		let request = IpcRequest {
			command: "echo".to_string(),
			payload: serde_json::json!({"message": "hello"}),
			request_id: Some("req-1".to_string()),
		};

		// Act
		let response = bridge.handle(request);

		// Assert
		assert!(response.success);
		assert!(response.error.is_none());
		assert_eq!(response.request_id, Some("req-1".to_string()));
		let data = response.data.unwrap();
		assert_eq!(data["echo"], "hello");
	}

	#[rstest]
	fn test_handle_unknown_command() {
		// Arrange
		let bridge = IpcBridge::new();
		let request = IpcRequest {
			command: "nonexistent".to_string(),
			payload: serde_json::json!({}),
			request_id: Some("req-2".to_string()),
		};

		// Act
		let response = bridge.handle(request);

		// Assert
		assert!(!response.success);
		assert!(response.data.is_none());
		assert!(response.error.is_some());
		let error = response.error.unwrap();
		assert!(error.contains("Unknown command"));
	}

	#[rstest]
	fn test_handle_message_valid_json() {
		// Arrange
		let mut bridge = IpcBridge::new();
		bridge.register("ping", |_| Ok(serde_json::json!({"pong": true})));

		let message = r#"{"command":"ping","payload":{},"request_id":"1"}"#;

		// Act
		let response_str = bridge.handle_message(message);

		// Assert
		let response: IpcResponse = serde_json::from_str(&response_str).unwrap();
		assert!(response.success);
		assert!(response.data.is_some());
	}

	#[rstest]
	fn test_handle_message_invalid_json() {
		// Arrange
		let bridge = IpcBridge::new();
		let message = "not valid json";

		// Act
		let response_str = bridge.handle_message(message);

		// Assert
		let response: IpcResponse = serde_json::from_str(&response_str).unwrap();
		assert!(!response.success);
		assert!(response.error.is_some());
		let error = response.error.unwrap();
		assert!(error.contains("Invalid request"));
	}

	#[rstest]
	fn test_multiple_commands_registration() {
		// Arrange
		let mut bridge = IpcBridge::new();

		// Act
		bridge.register("cmd1", |_| Ok(serde_json::json!({"id": 1})));
		bridge.register("cmd2", |_| Ok(serde_json::json!({"id": 2})));
		bridge.register("cmd3", |_| Ok(serde_json::json!({"id": 3})));

		// Assert
		assert!(bridge.has_command("cmd1"));
		assert!(bridge.has_command("cmd2"));
		assert!(bridge.has_command("cmd3"));
		assert!(!bridge.has_command("cmd4"));
		assert_eq!(bridge.commands().len(), 3);
	}

	#[rstest]
	fn test_handler_error_propagation() {
		// Arrange
		let mut bridge = IpcBridge::new();
		bridge.register("fail", |_| {
			Err(MobileError::Ipc("intentional error".to_string()))
		});

		let request = IpcRequest {
			command: "fail".to_string(),
			payload: serde_json::json!({}),
			request_id: None,
		};

		// Act
		let response = bridge.handle(request);

		// Assert
		assert!(!response.success);
		assert!(response.data.is_none());
		assert!(response.error.is_some());
		let error = response.error.unwrap();
		assert!(error.contains("intentional error"));
	}

	#[rstest]
	fn test_request_without_id() {
		// Arrange
		let mut bridge = IpcBridge::new();
		bridge.register("test", |_| Ok(serde_json::json!({})));

		let request = IpcRequest {
			command: "test".to_string(),
			payload: serde_json::json!({}),
			request_id: None,
		};

		// Act
		let response = bridge.handle(request);

		// Assert
		assert!(response.success);
		assert!(response.request_id.is_none());
	}
}

// =============================================================================
// Error Type Tests
// =============================================================================

mod error_tests {
	use super::*;

	#[rstest]
	fn test_mobile_error_display_webview_init() {
		// Arrange
		let error = MobileError::WebViewInit("failed to create".to_string());

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("WebView initialization failed"));
		assert!(display.contains("failed to create"));
	}

	#[rstest]
	fn test_mobile_error_display_ipc() {
		// Arrange
		let error = MobileError::Ipc("connection lost".to_string());

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("IPC error"));
		assert!(display.contains("connection lost"));
	}

	#[rstest]
	fn test_mobile_error_display_platform() {
		// Arrange
		let error = MobileError::Platform {
			platform: "android",
			message: "API not supported".to_string(),
		};

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("Platform error"));
		assert!(display.contains("android"));
		assert!(display.contains("API not supported"));
	}

	#[rstest]
	fn test_mobile_error_display_config() {
		// Arrange
		let error = MobileError::Config("invalid app_id".to_string());

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("Configuration error"));
		assert!(display.contains("invalid app_id"));
	}

	#[rstest]
	fn test_mobile_error_display_build() {
		// Arrange
		let error = MobileError::Build("compilation failed".to_string());

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("Build error"));
		assert!(display.contains("compilation failed"));
	}

	#[rstest]
	fn test_mobile_error_display_ir_processing() {
		// Arrange
		let error = MobileError::IrProcessing("invalid node".to_string());

		// Act
		let display = error.to_string();

		// Assert
		assert!(display.contains("IR processing error"));
		assert!(display.contains("invalid node"));
	}
}

// =============================================================================
// Integration Flow Tests
// =============================================================================

mod integration_flow_tests {
	use super::*;

	#[rstest]
	fn test_complete_mobile_app_setup_flow() {
		// Arrange
		let config = MobileConfig {
			app_name: "TestApp".to_string(),
			app_id: "com.test.app".to_string(),
			version: "1.0.0".to_string(),
			platform: TargetPlatform::Android,
			build: BuildConfig::default(),
			security: SecurityConfig::default(),
		};

		// Act
		// 1. Create platform
		let mut platform = AndroidPlatform::new();

		// 2. Initialize platform
		let init_result = platform.initialize(&config);

		// 3. Create visitor for code generation
		let visitor = MobileVisitor::new(config.clone());

		// 4. Setup IPC
		let mut bridge = IpcBridge::new();
		bridge.register("get_config", move |_| {
			Ok(serde_json::json!({
				"app_name": "TestApp",
				"version": "1.0.0"
			}))
		});

		// Assert
		assert!(init_result.is_ok());
		assert_eq!(platform.name(), "android");
		assert!(visitor.html().is_empty());
		assert!(bridge.has_command("get_config"));
	}

	#[rstest]
	fn test_complete_ios_app_setup_flow() {
		// Arrange
		let config = MobileConfig {
			app_name: "TestApp".to_string(),
			app_id: "com.test.app".to_string(),
			version: "1.0.0".to_string(),
			platform: TargetPlatform::Ios,
			build: BuildConfig {
				release: true,
				min_api_level: 13,
				target_api_level: None,
				experimental: false,
			},
			security: SecurityConfig::default(),
		};

		// Act
		// 1. Create platform
		let mut platform = IosPlatform::new();

		// 2. Initialize platform
		let init_result = platform.initialize(&config);

		// 3. Create visitor for code generation
		let visitor = MobileVisitor::new(config.clone());

		// 4. Setup IPC code generator
		let mut ipc_gen = IpcCodeGenerator::new();
		ipc_gen.add_command(IpcCommandDef {
			name: "native_share".to_string(),
			params: vec!["content".to_string(), "title".to_string()],
			return_type: Some("bool".to_string()),
		});

		// Assert
		assert!(init_result.is_ok());
		assert_eq!(platform.name(), "ios");
		assert_eq!(platform.protocol_scheme(), "wry");
		assert!(visitor.html().is_empty());
		let js = ipc_gen.generate_js_bindings();
		assert!(js.contains("native_share"));
	}

	#[rstest]
	fn test_visitor_with_complex_component_ir() {
		// Arrange
		let config = MobileConfig::default();
		let mut visitor = MobileVisitor::new(config);

		let click_handler: syn::Expr = syn::parse_quote!(on_click);
		let component = ComponentIR {
			props: vec![
				PropIR {
					name: "title".to_string(),
					ty: "String".to_string(),
					span: Span::call_site(),
				},
				PropIR {
					name: "count".to_string(),
					ty: "i32".to_string(),
					span: Span::call_site(),
				},
			],
			body: vec![NodeIR::Element(ElementIR {
				tag: "div".to_string(),
				attributes: vec![AttributeIR {
					name: "class".to_string(),
					value: AttrValueIR::Static("container".to_string()),
					span: Span::call_site(),
				}],
				events: vec![EventIR {
					name: "click".to_string(),
					handler: click_handler,
					span: Span::call_site(),
				}],
				children: vec![
					NodeIR::Element(ElementIR {
						tag: "h1".to_string(),
						attributes: vec![],
						events: vec![],
						children: vec![NodeIR::Text(TextIR {
							content: "Welcome".to_string(),
							span: Span::call_site(),
						})],
						span: Span::call_site(),
					}),
					NodeIR::Element(ElementIR {
						tag: "button".to_string(),
						attributes: vec![],
						events: vec![],
						children: vec![NodeIR::Text(TextIR {
							content: "Click me".to_string(),
							span: Span::call_site(),
						})],
						span: Span::call_site(),
					}),
				],
				span: Span::call_site(),
			})],
			span: Span::call_site(),
		};

		// Act
		let output = visitor.visit_component(&component);

		// Assert
		let output_str = output.to_string();
		assert!(output_str.contains("MobileElement"));
		assert!(output_str.contains("div"));
	}

	#[rstest]
	fn test_ipc_round_trip_communication() {
		// Arrange
		let mut bridge = IpcBridge::new();
		bridge.register("add", |req| {
			let a = req.payload.get("a").and_then(|v| v.as_i64()).unwrap_or(0);
			let b = req.payload.get("b").and_then(|v| v.as_i64()).unwrap_or(0);
			Ok(serde_json::json!({"result": a + b}))
		});

		// Act
		// Simulate JavaScript sending a message
		let message = r#"{"command":"add","payload":{"a":5,"b":3},"request_id":"calc-1"}"#;
		let response_str = bridge.handle_message(message);
		let response: IpcResponse = serde_json::from_str(&response_str).unwrap();

		// Assert
		assert!(response.success);
		assert_eq!(response.request_id, Some("calc-1".to_string()));
		let data = response.data.unwrap();
		assert_eq!(data["result"], 8);
	}
}
