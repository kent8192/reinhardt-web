//! App configuration for {{ app_name }}

use reinhardt_core::AppConfig;

pub struct {{ camel_case_app_name }}Config;

impl {{ camel_case_app_name }}Config {
    pub fn new() -> AppConfig {
        AppConfig::new("{{ app_name }}", "{{ app_name }}")
            .with_verbose_name("{{ camel_case_app_name }}")
    }
}

impl Default for {{ camel_case_app_name }}Config {
    fn default() -> Self {
        Self
    }
}
