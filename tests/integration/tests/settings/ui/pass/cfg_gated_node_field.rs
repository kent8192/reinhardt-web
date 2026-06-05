#![allow(dead_code)]

use reinhardt_conf::settings::fragment::SettingsFragment;
use reinhardt_conf::settings::schema::HasSettingsSchema;
use reinhardt_macros::settings;

#[settings(fragment = true, section = "cfg_gated")]
struct CfgGatedSettings {
	pub enabled: String,
	// This field must not leak into generated schema code when its cfg is disabled.
	#[cfg(any())]
	#[setting(node)]
	pub disabled: Option<DisabledSettings>,
}

#[settings(cfg_gated: CfgGatedSettings)]
struct ProjectSettings;

fn main() {
	let schema = ProjectSettings::schema();
	let _ = schema.cfg_gated.enabled.path();
	let _ = CfgGatedSettings::field_policies();
}
