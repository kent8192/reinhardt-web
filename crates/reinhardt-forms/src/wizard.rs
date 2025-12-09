use crate::form::{Form, FormError};
use std::collections::HashMap;

/// Type alias for wizard session data
type WizardSessionData = HashMap<String, HashMap<String, serde_json::Value>>;

/// Type alias for wizard step condition function
type WizardConditionFn = Box<dyn Fn(&WizardSessionData) -> bool + Send + Sync>;

/// FormWizard manages multi-step forms
pub struct FormWizard {
	steps: Vec<WizardStep>,
	current_step: usize,
	session_data: WizardSessionData,
}

/// A single step in the wizard
pub struct WizardStep {
	pub name: String,
	pub form: Form,
	pub condition: Option<WizardConditionFn>,
}

impl WizardStep {
	/// Create a new wizard step
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{WizardStep, Form};
	///
	/// let form = Form::new();
	/// let step = WizardStep::new("step1".to_string(), form);
	/// assert_eq!(step.name, "step1");
	/// ```
	pub fn new(name: String, form: Form) -> Self {
		Self {
			name,
			form,
			condition: None,
		}
	}
	/// Add a condition for when this step should be available
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{WizardStep, Form};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let form = Form::new();
	/// let step = WizardStep::new("step2".to_string(), form)
	///     .with_condition(|data| {
	///         data.get("step1").map_or(false, |step1_data| {
	///             step1_data.get("age").and_then(|v| v.as_i64()).map_or(false, |age| age >= 18)
	///         })
	///     });
	/// ```
	pub fn with_condition<F>(mut self, condition: F) -> Self
	where
		F: Fn(&WizardSessionData) -> bool + Send + Sync + 'static,
	{
		self.condition = Some(Box::new(condition));
		self
	}
	pub fn is_available(&self, session_data: &WizardSessionData) -> bool {
		if let Some(condition) = &self.condition {
			condition(session_data)
		} else {
			true
		}
	}
}

impl FormWizard {
	/// Create a new form wizard
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::FormWizard;
	///
	/// let wizard = FormWizard::new("wizard".to_string());
	/// assert_eq!(wizard.current_step(), 0);
	/// assert!(wizard.steps().is_empty());
	/// ```
	pub fn new(_prefix: String) -> Self {
		Self {
			steps: vec![],
			current_step: 0,
			session_data: HashMap::new(),
		}
	}

	pub fn steps(&self) -> &Vec<WizardStep> {
		&self.steps
	}
	/// Add a step to the wizard
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form = Form::new();
	/// let step = WizardStep::new("step1".to_string(), form);
	/// wizard.add_step(step);
	/// assert_eq!(wizard.steps().len(), 1);
	/// ```
	pub fn add_step(&mut self, step: WizardStep) {
		self.steps.push(step);
	}
	pub fn current_step(&self) -> usize {
		self.current_step
	}
	pub fn current_step_name(&self) -> Option<&str> {
		self.steps.get(self.current_step).map(|s| s.name.as_str())
	}
	pub fn current_form(&self) -> Option<&Form> {
		self.steps.get(self.current_step).map(|s| &s.form)
	}
	pub fn current_form_mut(&mut self) -> Option<&mut Form> {
		self.steps.get_mut(self.current_step).map(|s| &mut s.form)
	}
	pub fn total_steps(&self) -> usize {
		self.steps.len()
	}
	pub fn is_first_step(&self) -> bool {
		self.current_step == 0
	}
	pub fn is_last_step(&self) -> bool {
		self.current_step + 1 >= self.steps.len()
	}
	/// Move to the next available step
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form1 = Form::new();
	/// let form2 = Form::new();
	/// wizard.add_step(WizardStep::new("step1".to_string(), form1));
	/// wizard.add_step(WizardStep::new("step2".to_string(), form2));
	///
	/// let result = wizard.next_step();
	/// assert!(result.is_ok());
	/// assert_eq!(wizard.current_step(), 1);
	/// ```
	pub fn next_step(&mut self) -> Result<(), String> {
		if self.is_last_step() {
			return Err("Already at last step".to_string());
		}

		// Find next available step
		for i in (self.current_step + 1)..self.steps.len() {
			if self.steps[i].is_available(&self.session_data) {
				self.current_step = i;
				return Ok(());
			}
		}

		Err("No available next step".to_string())
	}
	/// Move to the previous step
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form1 = Form::new();
	/// let form2 = Form::new();
	/// wizard.add_step(WizardStep::new("step1".to_string(), form1));
	/// wizard.add_step(WizardStep::new("step2".to_string(), form2));
	/// wizard.next_step().unwrap(); // Move to step 2
	///
	/// let result = wizard.previous_step();
	/// assert!(result.is_ok());
	/// assert_eq!(wizard.current_step(), 0);
	/// ```
	pub fn previous_step(&mut self) -> Result<(), String> {
		if self.is_first_step() {
			return Err("Already at first step".to_string());
		}

		// Find previous available step
		for i in (0..self.current_step).rev() {
			if self.steps[i].is_available(&self.session_data) {
				self.current_step = i;
				return Ok(());
			}
		}

		Err("No available previous step".to_string())
	}
	/// Go to a specific step by name
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form1 = Form::new();
	/// let form2 = Form::new();
	/// wizard.add_step(WizardStep::new("step1".to_string(), form1));
	/// wizard.add_step(WizardStep::new("step2".to_string(), form2));
	///
	/// let result = wizard.goto_step("step2");
	/// assert!(result.is_ok());
	/// assert_eq!(wizard.current_step(), 1);
	/// ```
	pub fn goto_step(&mut self, name: &str) -> Result<(), String> {
		for (i, step) in self.steps.iter().enumerate() {
			if step.name == name && step.is_available(&self.session_data) {
				self.current_step = i;
				return Ok(());
			}
		}
		Err(format!("Step '{}' not found or not available", name))
	}
	/// Save data for the current step
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form = Form::new();
	/// wizard.add_step(WizardStep::new("step1".to_string(), form));
	///
	/// let mut data = HashMap::new();
	/// data.insert("name".to_string(), json!("John"));
	///
	/// let result = wizard.save_step_data(data);
	/// assert!(result.is_ok());
	/// ```
	pub fn save_step_data(
		&mut self,
		data: HashMap<String, serde_json::Value>,
	) -> Result<(), FormError> {
		if let Some(step) = self.steps.get(self.current_step) {
			self.session_data.insert(step.name.clone(), data);
			Ok(())
		} else {
			Err(FormError::Validation("Invalid step".to_string()))
		}
	}
	pub fn get_all_data(&self) -> &HashMap<String, HashMap<String, serde_json::Value>> {
		&self.session_data
	}
	pub fn get_step_data(&self, step_name: &str) -> Option<&HashMap<String, serde_json::Value>> {
		self.session_data.get(step_name)
	}
	pub fn clear_data(&mut self) {
		self.session_data.clear();
		self.current_step = 0;
	}
	/// Process current step and move to next if valid
	///
	/// # Examples
	///
	/// ```
	/// use reinhardt_forms::{FormWizard, WizardStep, Form};
	/// use std::collections::HashMap;
	/// use serde_json::json;
	///
	/// let mut wizard = FormWizard::new("wizard".to_string());
	/// let form = Form::new();
	/// wizard.add_step(WizardStep::new("step1".to_string(), form));
	///
	/// let mut data = HashMap::new();
	/// data.insert("field".to_string(), json!("value"));
	///
	// Note: process_step() requires form validation
	// let result = wizard.process_step(data);
	/// ```
	pub fn process_step(
		&mut self,
		data: HashMap<String, serde_json::Value>,
	) -> Result<bool, FormError> {
		if let Some(form) = self.current_form_mut() {
			form.bind(data.clone());

			if form.is_valid() {
				self.save_step_data(data)?;

				if !self.is_last_step() {
					self.next_step().map_err(FormError::Validation)?;
					Ok(false) // Not done yet
				} else {
					Ok(true) // Wizard complete
				}
			} else {
				Err(FormError::Validation("Form validation failed".to_string()))
			}
		} else {
			Err(FormError::Validation("Invalid step".to_string()))
		}
	}
	pub fn progress_percentage(&self) -> f32 {
		if self.steps.is_empty() {
			return 0.0;
		}
		((self.current_step + 1) as f32 / self.steps.len() as f32) * 100.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fields::CharField;

	#[test]
	fn test_wizard_basic() {
		let mut wizard = FormWizard::new("registration".to_string());

		let mut form1 = Form::new();
		form1.add_field(Box::new(CharField::new("username".to_string())));
		wizard.add_step(WizardStep::new("account".to_string(), form1));

		let mut form2 = Form::new();
		form2.add_field(Box::new(CharField::new("email".to_string())));
		wizard.add_step(WizardStep::new("contact".to_string(), form2));

		assert_eq!(wizard.total_steps(), 2);
		assert_eq!(wizard.current_step(), 0);
		assert_eq!(wizard.current_step_name(), Some("account"));
		assert!(wizard.is_first_step());
		assert!(!wizard.is_last_step());
	}

	#[test]
	fn test_wizard_navigation() {
		let mut wizard = FormWizard::new("test".to_string());

		for i in 1..=3 {
			let mut form = Form::new();
			form.add_field(Box::new(CharField::new(format!("field{}", i))));
			wizard.add_step(WizardStep::new(format!("step{}", i), form));
		}

		assert_eq!(wizard.current_step(), 0);

		wizard.next_step().unwrap();
		assert_eq!(wizard.current_step(), 1);

		wizard.next_step().unwrap();
		assert_eq!(wizard.current_step(), 2);
		assert!(wizard.is_last_step());

		wizard.previous_step().unwrap();
		assert_eq!(wizard.current_step(), 1);
	}

	#[test]
	fn test_wizard_conditional_step() {
		let mut wizard = FormWizard::new("test".to_string());

		let mut form1 = Form::new();
		form1.add_field(Box::new(CharField::new("type".to_string())));
		wizard.add_step(WizardStep::new("type_selection".to_string(), form1));

		let mut form2 = Form::new();
		form2.add_field(Box::new(CharField::new("premium_field".to_string())));
		let step2 = WizardStep::new("premium".to_string(), form2).with_condition(|data| {
			data.get("type_selection")
				.and_then(|d| d.get("type"))
				.and_then(|v| v.as_str())
				.map(|s| s == "premium")
				.unwrap_or(false)
		});
		wizard.add_step(step2);

		// Initially step 2 is not available
		assert!(!wizard.steps[1].is_available(&wizard.session_data));

		// Add data that makes step 2 available
		let mut data = HashMap::new();
		data.insert("type".to_string(), serde_json::json!("premium"));
		wizard.save_step_data(data).unwrap();

		assert!(wizard.steps[1].is_available(&wizard.session_data));
	}

	#[test]
	fn test_wizard_progress() {
		let mut wizard = FormWizard::new("test".to_string());

		for i in 1..=4 {
			let mut form = Form::new();
			form.add_field(Box::new(CharField::new(format!("field{}", i))));
			wizard.add_step(WizardStep::new(format!("step{}", i), form));
		}

		assert_eq!(wizard.progress_percentage(), 25.0); // Step 1/4

		wizard.next_step().unwrap();
		assert_eq!(wizard.progress_percentage(), 50.0); // Step 2/4

		wizard.next_step().unwrap();
		assert_eq!(wizard.progress_percentage(), 75.0); // Step 3/4

		wizard.next_step().unwrap();
		assert_eq!(wizard.progress_percentage(), 100.0); // Step 4/4
	}

	#[test]
	fn test_wizard_goto_step() {
		let mut wizard = FormWizard::new("test".to_string());

		for i in 1..=3 {
			let mut form = Form::new();
			form.add_field(Box::new(CharField::new(format!("field{}", i))));
			wizard.add_step(WizardStep::new(format!("step{}", i), form));
		}

		wizard.goto_step("step3").unwrap();
		assert_eq!(wizard.current_step(), 2);
		assert_eq!(wizard.current_step_name(), Some("step3"));
	}
}
