//! Debugging tools for templates
//!
//! Provides utilities for debugging template rendering:
//! - Variable inspection (debug filter)
//! - Template profiling (render time tracking)
//! - Context dumping (view all available variables)

use std::collections::HashMap;
use std::fmt;
use std::time::{Duration, Instant};

/// Template debug filter for printing variables
///
/// # Examples
///
/// ```
/// use reinhardt_templates::debug_filter;
///
/// let value = "test";
/// let result = debug_filter(&value);
/// assert!(result.contains("test"));
/// ```
pub fn debug_filter<T: fmt::Debug>(value: &T) -> String {
    format!("{:?}", value)
}

/// Template context for debugging
#[derive(Debug, Clone)]
pub struct TemplateContext {
    /// Available variables and their values
    pub variables: HashMap<String, String>,
}

impl TemplateContext {
    /// Create a new template context
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateContext;
    ///
    /// let context = TemplateContext::new();
    /// assert!(context.variables.is_empty());
    /// ```
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Add a variable to the context
    pub fn add_variable(&mut self, name: impl Into<String>, value: impl fmt::Display) {
        self.variables.insert(name.into(), value.to_string());
    }

    /// Dump all variables as a formatted string
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateContext;
    ///
    /// let mut context = TemplateContext::new();
    /// context.add_variable("name", "Alice");
    /// context.add_variable("age", 30);
    ///
    /// let dump = context.dump();
    /// assert!(dump.contains("name = Alice"));
    /// assert!(dump.contains("age = 30"));
    /// ```
    pub fn dump(&self) -> String {
        let mut output = String::from("Template Context:\n");
        for (key, value) in &self.variables {
            output.push_str(&format!("  {} = {}\n", key, value));
        }
        output
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Template profiling information
#[derive(Debug, Clone)]
pub struct TemplateProfile {
    /// Template name
    pub template_name: String,
    /// Render start time
    start_time: Option<Instant>,
    /// Render duration
    pub duration: Option<Duration>,
    /// Number of variables accessed
    pub variable_accesses: usize,
    /// Number of filters applied
    pub filters_applied: usize,
}

impl TemplateProfile {
    /// Create a new template profile
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateProfile;
    ///
    /// let profile = TemplateProfile::new("user_list.html");
    /// assert_eq!(profile.template_name, "user_list.html");
    /// ```
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            start_time: None,
            duration: None,
            variable_accesses: 0,
            filters_applied: 0,
        }
    }

    /// Start timing the render
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Stop timing the render
    pub fn stop(&mut self) {
        if let Some(start) = self.start_time {
            self.duration = Some(start.elapsed());
        }
    }

    /// Record a variable access
    pub fn record_variable_access(&mut self) {
        self.variable_accesses += 1;
    }

    /// Record a filter application
    pub fn record_filter(&mut self) {
        self.filters_applied += 1;
    }

    /// Get a summary of the profile
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateProfile;
    ///
    /// let mut profile = TemplateProfile::new("test.html");
    /// profile.start();
    /// profile.record_variable_access();
    /// profile.record_filter();
    /// profile.stop();
    ///
    /// let summary = profile.summary();
    /// assert!(summary.contains("test.html"));
    /// assert!(summary.contains("Variables accessed: 1"));
    /// ```
    pub fn summary(&self) -> String {
        let mut output = format!("Template: {}\n", self.template_name);

        if let Some(duration) = self.duration {
            output.push_str(&format!("Render time: {:?}\n", duration));
        }

        output.push_str(&format!("Variables accessed: {}\n", self.variable_accesses));
        output.push_str(&format!("Filters applied: {}\n", self.filters_applied));

        output
    }
}

/// Debug panel for templates
#[derive(Debug, Clone)]
pub struct DebugPanel {
    /// Whether debug mode is enabled
    pub enabled: bool,
    /// Template profiles
    pub profiles: Vec<TemplateProfile>,
    /// Template contexts
    pub contexts: HashMap<String, TemplateContext>,
}

impl DebugPanel {
    /// Create a new debug panel
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::DebugPanel;
    ///
    /// let panel = DebugPanel::new();
    /// assert!(!panel.enabled);
    /// ```
    pub fn new() -> Self {
        Self {
            enabled: false,
            profiles: Vec::new(),
            contexts: HashMap::new(),
        }
    }

    /// Enable debug mode
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable debug mode
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Add a profile
    pub fn add_profile(&mut self, profile: TemplateProfile) {
        self.profiles.push(profile);
    }

    /// Add a context
    pub fn add_context(&mut self, name: impl Into<String>, context: TemplateContext) {
        self.contexts.insert(name.into(), context);
    }

    /// Get summary of all profiles
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::{DebugPanel, TemplateProfile};
    ///
    /// let mut panel = DebugPanel::new();
    /// panel.enable();
    ///
    /// let mut profile = TemplateProfile::new("test.html");
    /// profile.start();
    /// profile.stop();
    /// panel.add_profile(profile);
    ///
    /// let summary = panel.summary();
    /// assert!(summary.contains("Templates Rendered: 1"));
    /// ```
    pub fn summary(&self) -> String {
        let mut output = String::from("=== Template Debug Panel ===\n\n");

        output.push_str(&format!("Templates Rendered: {}\n\n", self.profiles.len()));

        for profile in &self.profiles {
            output.push_str(&profile.summary());
            output.push('\n');
        }

        output
    }

    /// Get context dump for a template
    pub fn get_context(&self, template: &str) -> Option<String> {
        self.contexts.get(template).map(|ctx| ctx.dump())
    }
}

impl Default for DebugPanel {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::{Mutex, OnceLock};

/// Global debug panel (for development use)
static DEBUG_PANEL: OnceLock<Mutex<DebugPanel>> = OnceLock::new();

/// Initialize the global debug panel
///
/// # Examples
///
/// ```
/// use reinhardt_templates::init_debug_panel;
///
/// init_debug_panel();
/// ```
pub fn init_debug_panel() {
    DEBUG_PANEL.get_or_init(|| Mutex::new(DebugPanel::new()));
}

/// Get a reference to the global debug panel
pub fn get_debug_panel() -> Option<std::sync::MutexGuard<'static, DebugPanel>> {
    DEBUG_PANEL.get().and_then(|m| m.lock().ok())
}

/// Get a mutable reference to the global debug panel (same as get_debug_panel)
pub fn get_debug_panel_mut() -> Option<std::sync::MutexGuard<'static, DebugPanel>> {
    DEBUG_PANEL.get().and_then(|m| m.lock().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_filter() {
        let value = "test";
        let result = debug_filter(&value);
        assert_eq!(result, "\"test\"");
    }

    #[test]
    fn test_template_context_new() {
        let context = TemplateContext::new();
        assert_eq!(context.variables.len(), 0);
    }

    #[test]
    fn test_template_context_add_variable() {
        let mut context = TemplateContext::new();
        context.add_variable("name", "Alice");
        context.add_variable("age", 30);

        assert_eq!(context.variables.get("name"), Some(&"Alice".to_string()));
        assert_eq!(context.variables.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_template_context_dump() {
        let mut context = TemplateContext::new();
        context.add_variable("name", "Alice");
        context.add_variable("age", 30);

        let dump = context.dump();
        assert_eq!(dump.matches("Template Context:").count(), 1);
        assert_eq!(dump.matches("name = Alice").count(), 1);
        assert_eq!(dump.matches("age = 30").count(), 1);
    }

    #[test]
    fn test_template_profile_new() {
        let profile = TemplateProfile::new("test.html");
        assert_eq!(profile.template_name, "test.html");
        assert_eq!(profile.variable_accesses, 0);
        assert_eq!(profile.filters_applied, 0);
    }

    #[test]
    fn test_template_profile_timing() {
        let mut profile = TemplateProfile::new("test.html");
        profile.start();
        std::thread::sleep(std::time::Duration::from_millis(10));
        profile.stop();

        assert_eq!(profile.duration.is_some(), true);
        // NOTE: Using range assertion because timing is system-dependent
        // Verify that duration is at least 10ms but not absurdly large
        let duration_ms = profile.duration.unwrap().as_millis();
        assert_eq!(duration_ms >= 10, true);
        assert_eq!(duration_ms < 1000, true); // Should complete in less than 1 second
    }

    #[test]
    fn test_template_profile_record() {
        let mut profile = TemplateProfile::new("test.html");
        profile.record_variable_access();
        profile.record_variable_access();
        profile.record_filter();

        assert_eq!(profile.variable_accesses, 2);
        assert_eq!(profile.filters_applied, 1);
    }

    #[test]
    fn test_template_profile_summary() {
        let mut profile = TemplateProfile::new("test.html");
        profile.start();
        profile.record_variable_access();
        profile.stop();

        let summary = profile.summary();
        assert_eq!(summary.matches("test.html").count(), 1);
        assert_eq!(summary.matches("Variables accessed: 1").count(), 1);
    }

    #[test]
    fn test_debug_panel_new() {
        let panel = DebugPanel::new();
        assert_eq!(panel.enabled, false);
        assert_eq!(panel.profiles.len(), 0);
        assert_eq!(panel.contexts.len(), 0);
    }

    #[test]
    fn test_debug_panel_enable_disable() {
        let mut panel = DebugPanel::new();
        panel.enable();
        assert_eq!(panel.enabled, true);

        panel.disable();
        assert_eq!(panel.enabled, false);
    }

    #[test]
    fn test_debug_panel_add_profile() {
        let mut panel = DebugPanel::new();
        let profile = TemplateProfile::new("test.html");
        panel.add_profile(profile);

        assert_eq!(panel.profiles.len(), 1);
    }

    #[test]
    fn test_debug_panel_add_context() {
        let mut panel = DebugPanel::new();
        let mut context = TemplateContext::new();
        context.add_variable("name", "Alice");
        panel.add_context("test.html", context);

        assert_eq!(panel.contexts.len(), 1);
    }

    #[test]
    fn test_debug_panel_summary() {
        let mut panel = DebugPanel::new();
        let mut profile = TemplateProfile::new("test.html");
        profile.start();
        profile.stop();
        panel.add_profile(profile);

        let summary = panel.summary();
        assert_eq!(summary.matches("Templates Rendered: 1").count(), 1);
        assert_eq!(summary.matches("test.html").count(), 1);
    }

    #[test]
    fn test_debug_panel_get_context() {
        let mut panel = DebugPanel::new();
        let mut context = TemplateContext::new();
        context.add_variable("name", "Alice");
        panel.add_context("test.html", context);

        let dump = panel.get_context("test.html");
        assert_eq!(dump.is_some(), true);
        let dump_content = dump.unwrap();
        assert_eq!(dump_content.matches("name = Alice").count(), 1);
    }
}

// ============================================================================
// Extended debugging features
// ============================================================================

/// Template execution trace
#[derive(Debug, Clone)]
pub struct TemplateTrace {
    /// Template name
    pub template_name: String,
    /// Events in the trace
    pub events: Vec<TraceEvent>,
}

/// Template trace event
#[derive(Debug, Clone)]
pub enum TraceEvent {
    /// Variable access
    VariableAccess {
        name: String,
        value: Option<String>,
        line: usize,
    },
    /// Filter application
    FilterApplied {
        filter: String,
        input: String,
        output: String,
        line: usize,
    },
    /// Block entry
    BlockEntered {
        block_name: String,
        line: usize,
    },
    /// Block exit
    BlockExited {
        block_name: String,
        line: usize,
    },
    /// Include
    IncludeTemplate {
        template: String,
        line: usize,
    },
}

impl TemplateTrace {
    /// Create a new template trace
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::TemplateTrace;
    ///
    /// let trace = TemplateTrace::new("user.html");
    /// assert_eq!(trace.template_name, "user.html");
    /// ```
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            events: Vec::new(),
        }
    }

    /// Add an event to the trace
    pub fn add_event(&mut self, event: TraceEvent) {
        self.events.push(event);
    }

    /// Get a summary of the trace
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::{TemplateTrace, TraceEvent};
    ///
    /// let mut trace = TemplateTrace::new("test.html");
    /// trace.add_event(TraceEvent::VariableAccess {
    ///     name: "username".to_string(),
    ///     value: Some("alice".to_string()),
    ///     line: 10,
    /// });
    ///
    /// let summary = trace.summary();
    /// assert!(summary.contains("test.html"));
    /// assert!(summary.contains("Variable access"));
    /// ```
    pub fn summary(&self) -> String {
        let mut output = format!("Trace for template: {}\n", self.template_name);
        output.push_str(&format!("Total events: {}\n\n", self.events.len()));

        for (i, event) in self.events.iter().enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, event.format()));
        }

        output
    }
}

impl TraceEvent {
    /// Format the event for display
    pub fn format(&self) -> String {
        match self {
            TraceEvent::VariableAccess { name, value, line } => {
                if let Some(val) = value {
                    format!("Variable access: {} = {} (line {})", name, val, line)
                } else {
                    format!("Variable access: {} = undefined (line {})", name, line)
                }
            }
            TraceEvent::FilterApplied {
                filter,
                input,
                output,
                line,
            } => {
                format!("Filter '{}': {} -> {} (line {})", filter, input, output, line)
            }
            TraceEvent::BlockEntered { block_name, line } => {
                format!("Entered block '{}' (line {})", block_name, line)
            }
            TraceEvent::BlockExited { block_name, line } => {
                format!("Exited block '{}' (line {})", block_name, line)
            }
            TraceEvent::IncludeTemplate { template, line } => {
                format!("Included template '{}' (line {})", template, line)
            }
        }
    }
}

/// Performance metrics for template rendering
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Template name
    pub template_name: String,
    /// Total render time
    pub total_time: std::time::Duration,
    /// Time spent in filters
    pub filter_time: std::time::Duration,
    /// Time spent in includes
    pub include_time: std::time::Duration,
    /// Number of variables accessed
    pub variable_count: usize,
    /// Number of filters applied
    pub filter_count: usize,
}

impl PerformanceMetrics {
    /// Create new performance metrics
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::PerformanceMetrics;
    /// use std::time::Duration;
    ///
    /// let metrics = PerformanceMetrics::new("test.html");
    /// assert_eq!(metrics.template_name, "test.html");
    /// ```
    pub fn new(template_name: impl Into<String>) -> Self {
        Self {
            template_name: template_name.into(),
            total_time: std::time::Duration::ZERO,
            filter_time: std::time::Duration::ZERO,
            include_time: std::time::Duration::ZERO,
            variable_count: 0,
            filter_count: 0,
        }
    }

    /// Get a formatted report
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_templates::PerformanceMetrics;
    /// use std::time::Duration;
    ///
    /// let mut metrics = PerformanceMetrics::new("test.html");
    /// metrics.total_time = Duration::from_millis(100);
    /// metrics.variable_count = 5;
    ///
    /// let report = metrics.report();
    /// assert!(report.contains("test.html"));
    /// assert!(report.contains("Variables: 5"));
    /// ```
    pub fn report(&self) -> String {
        let mut output = format!("Performance Metrics: {}\n", self.template_name);
        output.push_str(&format!("Total time: {:?}\n", self.total_time));
        output.push_str(&format!("Filter time: {:?}\n", self.filter_time));
        output.push_str(&format!("Include time: {:?}\n", self.include_time));
        output.push_str(&format!("Variables: {}\n", self.variable_count));
        output.push_str(&format!("Filters: {}\n", self.filter_count));
        output
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn test_template_trace_new() {
        let trace = TemplateTrace::new("test.html");
        assert_eq!(trace.template_name, "test.html");
        assert_eq!(trace.events.len(), 0);
    }

    #[test]
    fn test_template_trace_add_event() {
        let mut trace = TemplateTrace::new("test.html");
        trace.add_event(TraceEvent::VariableAccess {
            name: "username".to_string(),
            value: Some("alice".to_string()),
            line: 10,
        });

        assert_eq!(trace.events.len(), 1);
    }

    #[test]
    fn test_template_trace_summary() {
        let mut trace = TemplateTrace::new("test.html");
        trace.add_event(TraceEvent::VariableAccess {
            name: "username".to_string(),
            value: Some("alice".to_string()),
            line: 10,
        });

        let summary = trace.summary();
        assert_eq!(summary.matches("test.html").count(), 1);
        assert_eq!(summary.matches("Variable access").count(), 1);
        assert_eq!(summary.matches("username").count(), 1);
    }

    #[test]
    fn test_trace_event_format() {
        let event = TraceEvent::VariableAccess {
            name: "test".to_string(),
            value: Some("value".to_string()),
            line: 5,
        };
        let formatted = event.format();
        assert_eq!(formatted, "Variable access: test = value (line 5)");
    }

    #[test]
    fn test_performance_metrics_new() {
        let metrics = PerformanceMetrics::new("test.html");
        assert_eq!(metrics.template_name, "test.html");
        assert_eq!(metrics.variable_count, 0);
        assert_eq!(metrics.filter_count, 0);
    }

    #[test]
    fn test_performance_metrics_report() {
        let mut metrics = PerformanceMetrics::new("test.html");
        metrics.total_time = std::time::Duration::from_millis(100);
        metrics.variable_count = 5;

        let report = metrics.report();
        assert_eq!(report.matches("test.html").count(), 1);
        assert_eq!(report.matches("Variables: 5").count(), 1);
    }
}
