use reinhardt_pages::{ClientForm, use_form};

#[derive(Clone)]
struct TenantId(&'static str);

fn tenant_default() -> TenantId {
	TenantId("default-tenant")
}

#[derive(Clone, ClientForm)]
struct SettingsRequest {
	name: String,
	#[serde(skip_serializing, default = "tenant_default")]
	tenant: TenantId,
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let form = SettingsRequestClientForm::new().with_defaults(SettingsRequest {
			name: "demo".to_string(),
			tenant: TenantId("custom-tenant"),
		});
		let runtime = use_form(&form).build();
		let _request = SettingsRequestClientForm::to_request(&runtime);
	});
}
