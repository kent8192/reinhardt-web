//! Security headers middleware

/// Content Security Policy configuration
#[derive(Debug, Clone)]
pub struct ContentSecurityPolicy {
    pub default_src: Vec<String>,
    pub script_src: Vec<String>,
    pub style_src: Vec<String>,
    pub img_src: Vec<String>,
    pub connect_src: Vec<String>,
    pub font_src: Vec<String>,
    pub object_src: Vec<String>,
    pub media_src: Vec<String>,
    pub frame_src: Vec<String>,
}

impl Default for ContentSecurityPolicy {
    fn default() -> Self {
        Self {
            default_src: vec!["'self'".to_string()],
            script_src: vec!["'self'".to_string()],
            style_src: vec!["'self'".to_string()],
            img_src: vec!["'self'".to_string()],
            connect_src: vec!["'self'".to_string()],
            font_src: vec!["'self'".to_string()],
            object_src: vec!["'none'".to_string()],
            media_src: vec!["'self'".to_string()],
            frame_src: vec!["'self'".to_string()],
        }
    }
}

impl ContentSecurityPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_header_value(&self) -> String {
        let mut parts = Vec::new();

        if !self.default_src.is_empty() {
            parts.push(format!("default-src {}", self.default_src.join(" ")));
        }
        if !self.script_src.is_empty() {
            parts.push(format!("script-src {}", self.script_src.join(" ")));
        }
        if !self.style_src.is_empty() {
            parts.push(format!("style-src {}", self.style_src.join(" ")));
        }
        if !self.img_src.is_empty() {
            parts.push(format!("img-src {}", self.img_src.join(" ")));
        }
        if !self.connect_src.is_empty() {
            parts.push(format!("connect-src {}", self.connect_src.join(" ")));
        }
        if !self.font_src.is_empty() {
            parts.push(format!("font-src {}", self.font_src.join(" ")));
        }
        if !self.object_src.is_empty() {
            parts.push(format!("object-src {}", self.object_src.join(" ")));
        }
        if !self.media_src.is_empty() {
            parts.push(format!("media-src {}", self.media_src.join(" ")));
        }
        if !self.frame_src.is_empty() {
            parts.push(format!("frame-src {}", self.frame_src.join(" ")));
        }

        parts.join("; ")
    }
}

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    pub x_content_type_options: bool,
    pub x_frame_options: Option<String>,
    pub x_xss_protection: bool,
    pub strict_transport_security: Option<String>,
    pub content_security_policy: Option<ContentSecurityPolicy>,
    pub referrer_policy: Option<String>,
    pub permissions_policy: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            x_content_type_options: true,
            x_frame_options: Some("DENY".to_string()),
            x_xss_protection: true,
            strict_transport_security: Some("max-age=31536000; includeSubDomains".to_string()),
            content_security_policy: Some(ContentSecurityPolicy::default()),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            permissions_policy: None,
        }
    }
}

/// Security headers middleware
pub struct SecurityHeadersMiddleware {
    config: SecurityHeadersConfig,
}

impl SecurityHeadersMiddleware {
    pub fn new() -> Self {
        Self {
            config: SecurityHeadersConfig::default(),
        }
    }

    pub fn with_config(config: SecurityHeadersConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &SecurityHeadersConfig {
        &self.config
    }
}

impl Default for SecurityHeadersMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SecurityHeadersConfig::default();
        assert!(config.x_content_type_options);
        assert_eq!(config.x_frame_options, Some("DENY".to_string()));
        assert!(config.x_xss_protection);
        assert!(config.strict_transport_security.is_some());
    }

    #[test]
    fn test_csp_to_header() {
        let csp = ContentSecurityPolicy::default();
        let header = csp.to_header_value();
        assert!(header.contains("default-src 'self'"));
        assert!(header.contains("script-src 'self'"));
    }
}
