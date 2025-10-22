//! Property Override Support for Hybrid Properties
//!
//! Provides a way to override hybrid property behavior in derived types.
//! Since Rust doesn't have traditional class inheritance like Python,
//! we use traits and composition to achieve similar functionality.

use std::marker::PhantomData;

/// Trait for overridable hybrid property behavior
pub trait HybridPropertyOverride<T, R> {
    /// Get instance-level value
    fn get_instance(&self, instance: &T) -> R;

    /// Get SQL expression (if available)
    fn get_expression(&self) -> Option<String> {
        None
    }

    /// Set instance-level value (if setter is defined)
    fn set_instance(&self, instance: &mut T, value: R) {
        let _ = (instance, value);
        // Default: no-op
    }
}

/// Overridable hybrid property wrapper
pub struct OverridableProperty<T, R, O>
where
    O: HybridPropertyOverride<T, R>,
{
    override_impl: O,
    _phantom: PhantomData<(T, R)>,
}

impl<T, R, O> OverridableProperty<T, R, O>
where
    O: HybridPropertyOverride<T, R>,
{
    /// Creates a new overridable property with the given implementation
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::override_property::{OverridableProperty, HybridPropertyOverride};
    ///
    /// struct Person {
    ///     name: String,
    /// }
    ///
    /// struct NameProperty;
    ///
    /// impl HybridPropertyOverride<Person, String> for NameProperty {
    ///     fn get_instance(&self, instance: &Person) -> String {
    ///         instance.name.clone()
    ///     }
    /// }
    ///
    /// let property = OverridableProperty::new(NameProperty);
    /// let person = Person { name: "Alice".to_string() };
    /// assert_eq!(property.get(&person), "Alice");
    /// ```
    pub fn new(override_impl: O) -> Self {
        Self {
            override_impl,
            _phantom: PhantomData,
        }
    }
    /// Gets the property value for the given instance
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::override_property::{OverridableProperty, HybridPropertyOverride};
    ///
    /// struct User {
    ///     first_name: String,
    ///     last_name: String,
    /// }
    ///
    /// struct FullNameProperty;
    ///
    /// impl HybridPropertyOverride<User, String> for FullNameProperty {
    ///     fn get_instance(&self, instance: &User) -> String {
    ///         format!("{} {}", instance.first_name, instance.last_name)
    ///     }
    /// }
    ///
    /// let property = OverridableProperty::new(FullNameProperty);
    /// let user = User {
    ///     first_name: "John".to_string(),
    ///     last_name: "Doe".to_string(),
    /// };
    /// assert_eq!(property.get(&user), "John Doe");
    /// ```
    pub fn get(&self, instance: &T) -> R {
        self.override_impl.get_instance(instance)
    }
    /// Gets the SQL expression for this property, if available
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::override_property::{OverridableProperty, HybridPropertyOverride};
    ///
    /// struct User {
    ///     email: String,
    /// }
    ///
    /// struct LowerEmailProperty;
    ///
    /// impl HybridPropertyOverride<User, String> for LowerEmailProperty {
    ///     fn get_instance(&self, instance: &User) -> String {
    ///         instance.email.to_lowercase()
    ///     }
    ///
    ///     fn get_expression(&self) -> Option<String> {
    ///         Some("LOWER(email)".to_string())
    ///     }
    /// }
    ///
    /// let property = OverridableProperty::new(LowerEmailProperty);
    /// assert_eq!(property.expression(), Some("LOWER(email)".to_string()));
    /// ```
    pub fn expression(&self) -> Option<String> {
        self.override_impl.get_expression()
    }
    /// Sets the property value for the given instance
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_hybrid::override_property::{OverridableProperty, HybridPropertyOverride};
    ///
    /// struct User {
    ///     full_name: String,
    /// }
    ///
    /// struct FullNameProperty;
    ///
    /// impl HybridPropertyOverride<User, String> for FullNameProperty {
    ///     fn get_instance(&self, instance: &User) -> String {
    ///         instance.full_name.clone()
    ///     }
    ///
    ///     fn set_instance(&self, instance: &mut User, value: String) {
    ///         instance.full_name = value;
    ///     }
    /// }
    ///
    /// let property = OverridableProperty::new(FullNameProperty);
    /// let mut user = User { full_name: String::new() };
    /// property.set(&mut user, "Jane Smith".to_string());
    /// assert_eq!(user.full_name, "Jane Smith");
    /// ```
    pub fn set(&self, instance: &mut T, value: R) {
        self.override_impl.set_instance(instance, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct Person {
        first_name: String,
        last_name: String,
    }

    #[derive(Debug, Clone)]
    struct Employee {
        person: Person,
        employee_id: String,
    }

    // Base implementation
    struct PersonNameProperty;

    impl HybridPropertyOverride<Person, String> for PersonNameProperty {
        fn get_instance(&self, instance: &Person) -> String {
            format!("{} {}", instance.first_name, instance.last_name)
        }

        fn get_expression(&self) -> Option<String> {
            Some("CONCAT(first_name, ' ', last_name)".to_string())
        }

        fn set_instance(&self, instance: &mut Person, value: String) {
            let parts: Vec<&str> = value.splitn(2, ' ').collect();
            if parts.len() == 2 {
                instance.first_name = parts[0].to_string();
                instance.last_name = parts[1].to_string();
            }
        }
    }

    // Override implementation for Employee
    struct EmployeeNameProperty;

    impl HybridPropertyOverride<Employee, String> for EmployeeNameProperty {
        fn get_instance(&self, instance: &Employee) -> String {
            format!(
                "{} {} ({})",
                instance.person.first_name, instance.person.last_name, instance.employee_id
            )
        }

        fn get_expression(&self) -> Option<String> {
            Some("CONCAT(first_name, ' ', last_name, ' (', employee_id, ')')".to_string())
        }

        fn set_instance(&self, instance: &mut Employee, value: String) {
            // Parse "First Last (ID)"
            if let Some(paren_pos) = value.rfind('(') {
                let name_part = value[..paren_pos].trim();
                let id_part = value[paren_pos + 1..].trim_end_matches(')');

                let parts: Vec<&str> = name_part.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    instance.person.first_name = parts[0].to_string();
                    instance.person.last_name = parts[1].to_string();
                }
                instance.employee_id = id_part.to_string();
            }
        }
    }

    #[test]
    fn test_base_property() {
        let person = Person {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
        };

        let property = OverridableProperty::new(PersonNameProperty);
        assert_eq!(property.get(&person), "John Doe");
        assert_eq!(
            property.expression(),
            Some("CONCAT(first_name, ' ', last_name)".to_string())
        );
    }

    #[test]
    fn test_override_property_getter_unit() {
        let employee = Employee {
            person: Person {
                first_name: "Jane".to_string(),
                last_name: "Smith".to_string(),
            },
            employee_id: "E123".to_string(),
        };

        let property = OverridableProperty::new(EmployeeNameProperty);
        assert_eq!(property.get(&employee), "Jane Smith (E123)");
    }

    #[test]
    fn test_override_expression() {
        let property = OverridableProperty::new(EmployeeNameProperty);
        assert_eq!(
            property.expression(),
            Some("CONCAT(first_name, ' ', last_name, ' (', employee_id, ')')".to_string())
        );
    }

    #[test]
    fn test_override_property_setter_unit() {
        let mut person = Person {
            first_name: String::new(),
            last_name: String::new(),
        };

        let property = OverridableProperty::new(PersonNameProperty);
        property.set(&mut person, "Alice Johnson".to_string());

        assert_eq!(person.first_name, "Alice");
        assert_eq!(person.last_name, "Johnson");
    }

    #[test]
    fn test_employee_override_setter() {
        let mut employee = Employee {
            person: Person {
                first_name: String::new(),
                last_name: String::new(),
            },
            employee_id: String::new(),
        };

        let property = OverridableProperty::new(EmployeeNameProperty);
        property.set(&mut employee, "Bob Brown (E456)".to_string());

        assert_eq!(employee.person.first_name, "Bob");
        assert_eq!(employee.person.last_name, "Brown");
        assert_eq!(employee.employee_id, "E456");
    }
}
