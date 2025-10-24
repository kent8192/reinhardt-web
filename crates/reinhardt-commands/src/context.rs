//! Command execution context

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CommandContext {
    pub args: Vec<String>,
    pub options: HashMap<String, Vec<String>>,
    pub verbosity: u8,
}

impl CommandContext {
    pub fn new(args: Vec<String>) -> Self {
        Self {
            args,
            options: HashMap::new(),
            verbosity: 0,
        }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_options(mut self, options: HashMap<String, Vec<String>>) -> Self {
        self.options = options;
        self
    }

    pub fn arg(&self, index: usize) -> Option<&String> {
        self.args.get(index)
    }

    pub fn option(&self, key: &str) -> Option<&String> {
        self.options.get(key).and_then(|v| v.first())
    }

    pub fn option_values(&self, key: &str) -> Option<Vec<String>> {
        self.options.get(key).cloned()
    }

    pub fn has_option(&self, key: &str) -> bool {
        self.options.contains_key(key)
    }

    pub fn info(&self, message: &str) {
        println!("[INFO] {}", message);
    }

    pub fn success(&self, message: &str) {
        println!("[SUCCESS] {}", message);
    }

    pub fn warning(&self, message: &str) {
        eprintln!("[WARNING] {}", message);
    }

    pub fn verbose(&self, message: &str) {
        println!("[VERBOSE] {}", message);
    }

    pub fn set_option(&mut self, key: String, value: String) {
        self.options.insert(key, vec![value]);
    }

    pub fn set_option_multi(&mut self, key: String, values: Vec<String>) {
        self.options.insert(key, values);
    }

    /// Check if system checks should be skipped
    ///
    /// Returns true if either `skip_checks` or `skip-checks` option is present.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use reinhardt_commands::CommandContext;
    ///
    /// let mut ctx = CommandContext::new(vec![]);
    /// ctx.set_option("skip-checks".to_string(), "true".to_string());
    ///
    /// assert!(ctx.should_skip_checks());
    /// ```
    pub fn should_skip_checks(&self) -> bool {
        self.has_option("skip_checks") || self.has_option("skip-checks")
    }

    /// Add an argument to the context
    pub fn add_arg(&mut self, arg: String) {
        self.args.push(arg);
    }

    /// Set the verbosity level
    pub fn set_verbosity(&mut self, level: u8) {
        self.verbosity = level;
    }

    /// Get the verbosity level
    pub fn verbosity(&self) -> u8 {
        self.verbosity
    }
}

impl Default for CommandContext {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
