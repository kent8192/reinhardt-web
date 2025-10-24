//! App configuration for {{ app_name }}

use reinhardt_core::AppConfig;

pub fn app_config() -> AppConfig {
    AppConfig {
        name: "{{ app_name }}".to_string(),
        label: "{{ app_name }}".to_string(),
        verbose_name: Some("{{ app_name|title }}".to_string()),
        path: Some(env!("CARGO_MANIFEST_DIR").to_string()),
        default_auto_field: Some("BigAutoField".to_string()),
        models_ready: false,
    }
}
