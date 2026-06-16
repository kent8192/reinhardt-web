use reinhardt_pages::{
	CustomWidgetContext, CustomWidgetRawValue, FieldError, FormWidgetAdapter, FormWidgetError,
	FormWidgetValueKind, Page, form,
};

fn date_range_picker(_props: DateRangeProps) -> Page {
	Page::Fragment(Vec::new())
}

#[derive(Clone)]
struct DateRangeProps {
	value: String,
	disabled: bool,
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
		}
	}

	fn parse(raw: CustomWidgetRawValue) -> Result<String, FormWidgetError> {
		match raw {
			CustomWidgetRawValue::String(value) => Ok(value),
			_ => Err(FormWidgetError::new("expected string value")),
		}
	}

	fn format(value: &String) -> CustomWidgetRawValue {
		CustomWidgetRawValue::String(value.clone())
	}
}

fn main() {
	let _form = form! {
		name: CustomWidgetForm,
		action: "/custom",
		fields: {
			date_range: CharField {
				widget: CustomWidget(date_range_picker) {
					experimental,
					adapter: DateRangeAdapter,
				},
			}
		}
	};

	let _field_error = FieldError::new("example");
}
