#![cfg(not(target_arch = "wasm32"))]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use reinhardt_pages::{
	CustomWidgetContext, CustomWidgetRawValue, FieldError, FormEvent, FormWidgetAdapter,
	FormWidgetError, FormWidgetValueKind, Page, ResetOnDeps, RevalidateOn, UseFormSubmitOutcome,
	form, use_form,
};

thread_local! {
	static LAST_CUSTOM_WIDGET_PROPS: RefCell<Option<DateRangeProps>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct DateRangeProps {
	value: String,
	disabled: bool,
	touched: bool,
	error: Option<FieldError>,
	on_raw_change: Rc<dyn Fn(CustomWidgetRawValue) -> Result<(), FormWidgetError>>,
}

fn capturing_date_range_picker(props: DateRangeProps) -> Page {
	LAST_CUSTOM_WIDGET_PROPS.with(|slot| {
		*slot.borrow_mut() = Some(props);
	});
	Page::Fragment(Vec::new())
}

fn take_last_custom_widget_props() -> DateRangeProps {
	LAST_CUSTOM_WIDGET_PROPS.with(|slot| {
		slot.borrow_mut()
			.take()
			.expect("custom widget props should be captured")
	})
}

struct DateRangeAdapter;

impl FormWidgetAdapter<String> for DateRangeAdapter {
	type ComponentProps = DateRangeProps;

	fn value_kind() -> FormWidgetValueKind {
		FormWidgetValueKind::Value
	}

	fn props(ctx: CustomWidgetContext<String>) -> Self::ComponentProps {
		DateRangeProps {
			value: ctx.value,
			disabled: ctx.disabled,
			touched: ctx.touched,
			error: ctx.error,
			on_raw_change: ctx.on_raw_change,
		}
	}

	fn parse(raw: CustomWidgetRawValue) -> Result<String, FormWidgetError> {
		match raw {
			CustomWidgetRawValue::String(value) if value.contains("..") => Ok(value),
			CustomWidgetRawValue::String(_) => Err(FormWidgetError::new("invalid date range")),
			_ => Err(FormWidgetError::new("expected string value")),
		}
	}

	fn format(value: &String) -> CustomWidgetRawValue {
		CustomWidgetRawValue::String(value.clone())
	}
}

#[test]
fn use_form_builds_runtime_from_generated_form_contract() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
				required,
			}
			bio: TextField {
				initial: "Compiler engineer",
			}
		}
	};

	let runtime = use_form(&profile).build();

	assert_eq!(runtime.get_values().display_name, "Ada".to_string());
	assert_eq!(runtime.get_values().bio, "Compiler engineer".to_string());
	assert!(!runtime.form_state().is_dirty.get());
	assert!(
		!runtime
			.get_field_state(profile.display_name_field())
			.is_dirty
	);
}

#[test]
fn use_form_tracks_field_updates_errors_and_resets() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
				required,
			}
			bio: TextField {
				initial: "Compiler engineer",
			}
		}
	};

	let runtime = use_form(&profile).build();

	runtime.set_value(profile.display_name_field(), "Grace".to_string());
	runtime.set_error(
		profile.display_name_field(),
		FieldError::new("display name is already taken"),
	);

	assert_eq!(runtime.get_values().display_name, "Grace".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(runtime.form_state().is_touched.get());
	assert_eq!(
		runtime
			.get_field_state(profile.display_name_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("display name is already taken")
	);

	runtime.clear_field_error(profile.display_name_field());
	assert!(
		runtime
			.get_field_state(profile.display_name_field())
			.error
			.is_none()
	);

	runtime.reset_field(profile.display_name_field());
	assert_eq!(runtime.get_values().display_name, "Ada".to_string());
	assert!(
		!runtime
			.get_field_state(profile.display_name_field())
			.is_dirty
	);

	runtime.set_value(profile.bio_field(), "COBOL pioneer".to_string());
	runtime.reset_default_values();
	assert_eq!(runtime.default_values().bio, "COBOL pioneer".to_string());
	assert!(!runtime.form_state().is_dirty.get());

	runtime.set_value(profile.bio_field(), "Compiler engineer".to_string());
	assert!(runtime.form_state().is_dirty.get());

	runtime.reset();
	assert_eq!(runtime.get_values().bio, "COBOL pioneer".to_string());
	assert!(!runtime.form_state().is_dirty.get());
}

#[test]
fn use_form_tracks_month_and_week_as_strings() {
	let schedule = form! {
		name: ScheduleForm,
		action: "/schedule",
		fields: {
			billing_month: CharField {
				widget: MonthInput,
				initial: "2026-06".to_string(),
			}
			sprint_week: CharField {
				widget: WeekInput,
				initial: "2026-W24".to_string(),
			}
		}
	};

	let runtime = use_form(&schedule).build();
	let billing_month = runtime.watch_field::<String>(schedule.billing_month_field());
	let sprint_week = runtime.watch_field::<String>(schedule.sprint_week_field());

	assert_eq!(runtime.get_values().billing_month, "2026-06".to_string());
	assert_eq!(runtime.get_values().sprint_week, "2026-W24".to_string());
	assert_eq!(billing_month.get(), "2026-06".to_string());
	assert_eq!(sprint_week.get(), "2026-W24".to_string());
	assert!(!runtime.form_state().is_dirty.get());
	assert!(
		!runtime
			.get_field_state(schedule.billing_month_field())
			.is_dirty
	);
	assert!(
		!runtime
			.get_field_state(schedule.sprint_week_field())
			.is_dirty
	);

	runtime.set_value(schedule.billing_month_field(), "2026-07".to_string());

	assert_eq!(runtime.get_values().billing_month, "2026-07".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(
		runtime
			.get_field_state(schedule.billing_month_field())
			.is_dirty
	);

	runtime.reset();

	assert_eq!(runtime.get_values().billing_month, "2026-06".to_string());
	assert_eq!(runtime.get_values().sprint_week, "2026-W24".to_string());
	assert!(!runtime.form_state().is_dirty.get());
	assert!(
		!runtime
			.get_field_state(schedule.billing_month_field())
			.is_dirty
	);
	assert!(
		!runtime
			.get_field_state(schedule.sprint_week_field())
			.is_dirty
	);
}

#[test]
fn use_form_can_sync_after_native_reset() {
	let profile = form! {
		name: NativeResetForm,
		action: "/profile",
		fields: {
			name: CharField {
				initial: "before".to_string(),
			}
			reset: ResetButton {
				label: "Reset",
			}
		}
	};

	let runtime = use_form(&profile).build();

	runtime.set_value(profile.name_field(), "after".to_string());
	runtime.set_error(profile.name_field(), FieldError::new("name is invalid"));

	assert_eq!(runtime.get_values().name, "after".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(runtime.form_state().is_touched.get());
	assert!(runtime.get_field_state(profile.name_field()).is_dirty);
	assert!(runtime.get_field_state(profile.name_field()).is_touched);
	assert_eq!(
		runtime
			.get_field_state(profile.name_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("name is invalid")
	);

	profile.name().set("before".to_string());
	runtime.sync_after_native_reset();

	assert_eq!(runtime.get_values().name, "before".to_string());
	assert!(!runtime.form_state().is_dirty.get());
	assert!(!runtime.form_state().is_touched.get());
	assert!(!runtime.get_field_state(profile.name_field()).is_dirty);
	assert!(!runtime.get_field_state(profile.name_field()).is_touched);
	assert!(
		runtime
			.get_field_state(profile.name_field())
			.error
			.is_none()
	);

	profile.name().set("native value".to_string());
	runtime.set_value(profile.name_field(), "runtime touched".to_string());
	profile.name().set("native value".to_string());
	runtime.set_error(profile.name_field(), FieldError::new("native error"));

	assert_eq!(runtime.get_values().name, "native value".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(runtime.form_state().is_touched.get());
	assert!(runtime.get_field_state(profile.name_field()).is_dirty);
	assert!(runtime.get_field_state(profile.name_field()).is_touched);
	assert_eq!(
		runtime
			.get_field_state(profile.name_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("native error")
	);

	runtime.sync_after_native_reset();
	let name_state = runtime.get_field_state(profile.name_field());

	assert_eq!(runtime.get_values().name, "native value".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(name_state.is_dirty);
	assert_eq!(runtime.form_state().is_dirty.get(), name_state.is_dirty);
	assert!(!runtime.form_state().is_touched.get());
	assert!(!name_state.is_touched);
	assert!(name_state.error.is_none());
	assert!(runtime.form_state().error.get().is_none());
}

#[test]
fn custom_widget_bridge_parse_error_sets_runtime_field_error() {
	let booking = form! {
		name: CustomWidgetRuntimeForm,
		action: "/booking",
		fields: {
			date_range: CharField {
				initial: "2026-06-01..2026-06-02".to_string(),
				disabled,
				widget: CustomWidget(capturing_date_range_picker) {
					experimental,
					adapter: DateRangeAdapter,
				},
			}
		}
	};
	let runtime = use_form(&booking).build();
	let _page = booking.clone().into_page();
	let props = take_last_custom_widget_props();

	assert_eq!(props.value, "2026-06-01..2026-06-02");
	assert!(props.disabled);
	assert!(!props.touched);
	assert!(props.error.is_none());

	let invalid_result = (props.on_raw_change)(CustomWidgetRawValue::String("invalid".to_string()));

	assert!(invalid_result.is_err());
	assert_eq!(
		invalid_result.err().as_ref().map(FormWidgetError::message),
		Some("invalid date range")
	);
	assert_eq!(
		runtime
			.get_field_state(booking.date_range_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("invalid date range")
	);
	assert_eq!(
		runtime.get_values().date_range,
		"2026-06-01..2026-06-02".to_string()
	);
	assert!(runtime.trigger().is_err());
	assert_eq!(
		runtime.handle_submit(),
		UseFormSubmitOutcome::ValidationFailed
	);

	let _page = booking.clone().into_page();
	let props = take_last_custom_widget_props();
	assert!(props.disabled);
	assert!(props.touched);
	assert_eq!(
		props.error.as_ref().map(FieldError::message),
		Some("invalid date range")
	);

	let valid_result = (props.on_raw_change)(CustomWidgetRawValue::String(
		"2026-06-03..2026-06-04".to_string(),
	));

	assert!(valid_result.is_ok());
	assert_eq!(
		runtime.get_values().date_range,
		"2026-06-03..2026-06-04".to_string()
	);
	assert!(
		runtime
			.get_field_state(booking.date_range_field())
			.error
			.is_none()
	);

	let _page = booking.clone().into_page();
	let props = take_last_custom_widget_props();
	assert_eq!(props.value, "2026-06-03..2026-06-04");
	assert!(props.disabled);
	assert!(props.touched);
	assert!(props.error.is_none());
}

#[test]
fn use_form_keeps_generated_initial_defaults_separate_from_live_signals() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
	};

	profile.display_name().set("Grace".to_string());
	let runtime = use_form(&profile).build();

	assert_eq!(runtime.default_values().display_name, "Ada".to_string());
	assert_eq!(runtime.get_values().display_name, "Grace".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(
		runtime
			.get_field_state(profile.display_name_field())
			.is_dirty
	);
}

#[test]
fn use_form_maps_generated_validator_errors_to_field_state() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
		validators: {
			display_name: [|v| v == "Ada" =>"display name must remain Ada", ],
		}
	};
	let runtime = use_form(&profile).build();

	runtime.set_value(profile.display_name_field(), "Grace".to_string());
	let result = runtime.trigger();

	assert!(result.is_err());
	assert_eq!(runtime.form_state().form_error.get(), None);
	assert_eq!(
		runtime
			.get_field_state(profile.display_name_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("display name must remain Ada")
	);
}

#[test]
fn use_form_syncs_direct_generated_signal_changes() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
		validators: {
			display_name: [|v| v == "Ada" =>"display name must remain Ada", ],
		}
	};
	let runtime = use_form(&profile)
		.revalidate_on(RevalidateOn::Change)
		.build();
	let values = runtime.watch();
	let event_count = Rc::new(Cell::new(0));
	let event_count_for_subscription = Rc::clone(&event_count);
	let _subscription = runtime.subscribe(move |event| match event {
		FormEvent::ValueChanged { .. } | FormEvent::Validated => {
			event_count_for_subscription.set(event_count_for_subscription.get() + 1);
		}
		_ => {}
	});

	profile.display_name().set("Grace".to_string());

	assert_eq!(runtime.get_values().display_name, "Grace".to_string());
	assert_eq!(values.get().display_name, "Grace".to_string());
	assert!(runtime.form_state().is_dirty.get());
	assert!(runtime.form_state().is_touched.get());
	assert!(
		runtime
			.get_field_state(profile.display_name_field())
			.is_touched
	);
	assert_eq!(
		runtime
			.get_field_state(profile.display_name_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("display name must remain Ada")
	);
	assert_eq!(event_count.get(), 2);
}

#[test]
fn use_form_accepts_json_field_runtime_contracts() {
	let settings = form! {
		name: SettingsForm,
		action: "/settings",
		fields: {
			payload: JsonField<::serde_json::Value> {
				initial: ::serde_json::json!( {
					"theme": "dark"
				}),
			}
		}
	};

	let runtime = use_form(&settings).build();

	assert_eq!(
		runtime.get_values().payload,
		::serde_json::json!({"theme": "dark"})
	);
}

#[test]
fn deps_reconciliation_keeps_dirty_values_and_updates_pristine_values() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
				required,
			}
			bio: TextField {
				initial: "Compiler engineer",
			}
		}
	};

	let runtime = use_form(&profile)
		.deps((1_u64,))
		.reset_on_deps(ResetOnDeps::KeepDirtyValues)
		.keep_errors(false)
		.revalidate_on(RevalidateOn::DepsChange)
		.build();

	runtime.set_value(profile.display_name_field(), "Grace".to_string());

	let mut refreshed = runtime.default_values();
	refreshed.display_name = "Katherine".to_string();
	refreshed.bio = "NASA mathematician".to_string();

	runtime.reconcile_defaults(refreshed, (2_u64,));

	assert_eq!(runtime.get_values().display_name, "Grace".to_string());
	assert_eq!(runtime.get_values().bio, "NASA mathematician".to_string());
	assert_eq!(
		runtime.default_values().display_name,
		"Katherine".to_string()
	);
}

#[test]
fn subscriptions_receive_value_and_submit_events() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
	};
	let runtime = use_form(&profile).build();
	let event_count = Rc::new(Cell::new(0));
	let event_count_for_subscription = Rc::clone(&event_count);
	let profile_for_subscription = profile.clone();
	let _subscription = runtime.subscribe(move |event| match event {
		FormEvent::ValueChanged { field } => {
			assert_eq!(field, profile_for_subscription.display_name_field());
			event_count_for_subscription.set(event_count_for_subscription.get() + 1);
		}
		FormEvent::Submitted => {
			event_count_for_subscription.set(event_count_for_subscription.get() + 1);
		}
		_ => {}
	});

	runtime.set_value(profile.display_name_field(), "Grace".to_string());
	assert_eq!(runtime.handle_submit(), UseFormSubmitOutcome::Submitted);
	assert_eq!(event_count.get(), 2);
}

#[test]
fn watch_field_tracks_runtime_value_changes() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
	};
	let runtime = use_form(&profile).build();
	let display_name = runtime.watch_field::<String>(profile.display_name_field());

	assert_eq!(display_name.get(), "Ada".to_string());

	runtime.set_value(profile.display_name_field(), "Grace".to_string());

	assert_eq!(display_name.get(), "Grace".to_string());
	assert_eq!(runtime.watch().get().display_name, "Grace".to_string());
}

#[test]
fn validation_failure_sets_form_error_and_submit_failure_state() {
	let signup = form! {
		name: SignupForm,
		action: "/signup",
		fields: {
			email: CharField {
				initial: "",
				required,
			}
		}
	};
	let runtime = use_form(&signup).build();

	assert!(runtime.trigger().is_err());
	assert!(runtime.form_state().form_error.get().is_none());
	assert_eq!(
		runtime
			.get_field_state(signup.email_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("email is required")
	);
	assert_eq!(
		runtime.handle_submit(),
		UseFormSubmitOutcome::ValidationFailed
	);
	assert!(!runtime.form_state().is_submitting.get());
	assert!(!runtime.form_state().is_submit_successful.get());
	assert!(runtime.form_state().error.get().is_some());

	runtime.set_value(signup.email_field(), "ada@example.com".to_string());

	assert!(runtime.trigger_field(signup.email_field()).is_ok());
	assert!(runtime.form_state().error.get().is_none());
}

#[test]
fn use_form_accepts_file_field_runtime_contracts() {
	let upload = form! {
		name: UploadForm,
		action: "/upload",
		fields: {
			document: FileField {
				required,
			}
			avatar: ImageField {}
		}
	};

	let runtime = use_form(&upload).build();
	let document = runtime.watch_field::<Option<web_sys::File>>(upload.document_field());
	let avatar = runtime.watch_field::<Option<web_sys::File>>(upload.avatar_field());

	assert!(runtime.get_values().document.is_none());
	assert!(runtime.get_values().avatar.is_none());
	assert!(document.get().is_none());
	assert!(avatar.get().is_none());
	assert!(!runtime.form_state().is_dirty.get());
	assert!(!runtime.get_field_state(upload.document_field()).is_dirty);

	let result = runtime.trigger();

	assert!(result.is_err());
	assert_eq!(
		runtime
			.get_field_state(upload.document_field())
			.error
			.as_ref()
			.map(FieldError::message),
		Some("document is required")
	);

	runtime.set_value(upload.document_field(), None::<web_sys::File>);

	assert!(runtime.get_field_state(upload.document_field()).is_touched);
	assert!(!runtime.get_field_state(upload.document_field()).is_dirty);
}

#[test]
fn submit_callbacks_run_in_order_after_dependencies_are_configured() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
	};
	let order = Rc::new(Cell::new(0));
	let start_order = Rc::clone(&order);
	let success_order = Rc::clone(&order);
	let runtime = use_form(&profile)
		.deps(("tenant-a",))
		.on_submit_start(move |handle| {
			assert_eq!(start_order.get(), 0);
			assert_eq!(handle.get_values().display_name, "Ada".to_string());
			start_order.set(1);
		})
		.on_submit_success(move |handle| {
			assert_eq!(success_order.get(), 1);
			assert!(handle.form_state().is_submit_successful.get());
			success_order.set(2);
		})
		.build();

	assert_eq!(runtime.handle_submit(), UseFormSubmitOutcome::Submitted);
	assert_eq!(order.get(), 2);
}

#[test]
fn submit_callbacks_survive_deps_configured_after_callback_registration() {
	let profile = form! {
		name: ProfileForm,
		action: "/profile",
		fields: {
			display_name: CharField {
				initial: "Ada",
			}
		}
	};
	let success_count = Rc::new(Cell::new(0));
	let success_count_for_callback = Rc::clone(&success_count);

	let runtime = use_form(&profile)
		.on_submit_success(move |handle| {
			assert_eq!(handle.get_values().display_name, "Ada".to_string());
			success_count_for_callback.set(success_count_for_callback.get() + 1);
		})
		.deps(("tenant-a",))
		.build();

	assert_eq!(runtime.handle_submit(), UseFormSubmitOutcome::Submitted);
	assert_eq!(success_count.get(), 1);
}
