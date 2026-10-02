use reinhardt_pages::{ClientForm, use_form};

#[derive(Clone)]
struct TenantId(&'static str);

#[derive(Clone, ClientForm)]
struct SettingsRequest {
	name: String,
	#[serde(skip_serializing, default = "Self::tenant_default")]
	tenant: TenantId,
}

impl SettingsRequest {
	fn tenant_default() -> TenantId {
		TenantId("default-tenant")
	}
}

fn main() {
	reinhardt_core::reactive::ReactiveScope::run(|| {
		let form = SettingsRequestClientForm::new();
		let runtime = use_form(&form).build();
		let _request = SettingsRequestClientForm::to_request(&runtime);
	});
}
