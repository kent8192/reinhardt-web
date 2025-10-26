//! Credit card validator with Luhn algorithm and card type detection

use crate::{ValidationError, ValidationResult, Validator};

/// Credit card types supported by the validator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardType {
    /// Visa cards (starting with 4)
    Visa,
    /// MasterCard (starting with 51-55 or 2221-2720)
    MasterCard,
    /// American Express (starting with 34 or 37)
    AmericanExpress,
    /// Discover cards (starting with 6011, 622126-622925, 644-649, or 65)
    Discover,
    /// JCB cards (starting with 3528-3589)
    JCB,
    /// Diners Club (starting with 300-305, 36, or 38)
    DinersClub,
}

impl CardType {
    /// Returns the display name of the card type
    pub fn as_str(&self) -> &'static str {
        match self {
            CardType::Visa => "Visa",
            CardType::MasterCard => "MasterCard",
            CardType::AmericanExpress => "American Express",
            CardType::Discover => "Discover",
            CardType::JCB => "JCB",
            CardType::DinersClub => "Diners Club",
        }
    }
}

impl std::fmt::Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Credit card number validator
///
/// This validator checks credit card numbers using the Luhn algorithm
/// and can optionally restrict accepted card types.
///
/// # Examples
///
/// ```
/// use reinhardt_validators::{CreditCardValidator, CardType};
///
/// // Accept any valid card
/// let validator = CreditCardValidator::new();
/// assert!(validator.validate("4532015112830366").is_ok()); // Valid Visa
///
/// // Restrict to specific card types
/// let validator = CreditCardValidator::new()
///     .allow_types(vec![CardType::Visa, CardType::MasterCard]);
/// assert!(validator.validate("4532015112830366").is_ok()); // Visa allowed
/// ```
pub struct CreditCardValidator {
    allowed_types: Option<Vec<CardType>>,
    message: Option<String>,
}

impl CreditCardValidator {
    /// Creates a new credit card validator that accepts any valid card type
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::CreditCardValidator;
    ///
    /// let validator = CreditCardValidator::new();
    /// ```
    pub fn new() -> Self {
        Self {
            allowed_types: None,
            message: None,
        }
    }

    /// Creates a validator that only accepts specific card types
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{CreditCardValidator, CardType};
    ///
    /// let validator = CreditCardValidator::with_card_types(vec![
    ///     CardType::Visa,
    ///     CardType::MasterCard,
    /// ]);
    /// ```
    pub fn with_card_types(types: Vec<CardType>) -> Self {
        Self {
            allowed_types: Some(types),
            message: None,
        }
    }

    /// Restricts accepted card types (builder pattern)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{CreditCardValidator, CardType};
    ///
    /// let validator = CreditCardValidator::new()
    ///     .allow_types(vec![CardType::Visa, CardType::MasterCard]);
    /// ```
    pub fn allow_types(mut self, types: Vec<CardType>) -> Self {
        self.allowed_types = Some(types);
        self
    }

    /// Sets a custom error message
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::CreditCardValidator;
    ///
    /// let validator = CreditCardValidator::new()
    ///     .with_message("Please enter a valid credit card number");
    /// ```
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Validates a credit card number and returns its type
    ///
    /// This method:
    /// 1. Removes dashes and spaces from the input
    /// 2. Checks if the result contains only digits
    /// 3. Validates using the Luhn algorithm
    /// 4. Detects the card type
    /// 5. Checks if the card type is allowed (if type restrictions are set)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_validators::{CreditCardValidator, CardType};
    ///
    /// let validator = CreditCardValidator::new();
    /// let card_type = validator.validate("4532-0151-1283-0366").unwrap();
    /// assert_eq!(card_type, CardType::Visa);
    /// ```
    pub fn validate(&self, value: &str) -> Result<CardType, ValidationError> {
        // Remove dashes and spaces
        let cleaned: String = value
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect();

        // Check if contains only digits
        if !cleaned.chars().all(|c| c.is_ascii_digit()) {
            return Err(self.error_for_value(value));
        }

        // Validate using Luhn algorithm
        if !Self::luhn_check(&cleaned) {
            return Err(self.error_for_value(value));
        }

        // Detect card type
        let card_type = Self::detect_card_type(&cleaned)
            .ok_or_else(|| ValidationError::InvalidCreditCard("Unknown card type".to_string()))?;

        // Check if card type is allowed
        if let Some(ref allowed) = self.allowed_types {
            if !allowed.contains(&card_type) {
                let allowed_str = allowed
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(ValidationError::CardTypeNotAllowed {
                    card_type: card_type.to_string(),
                    allowed_types: allowed_str,
                });
            }
        }

        Ok(card_type)
    }

    /// Validates a credit card number using the Luhn algorithm
    ///
    /// The Luhn algorithm (also known as the modulus 10 algorithm) is a simple
    /// checksum formula used to validate identification numbers such as credit
    /// card numbers.
    ///
    /// Algorithm steps:
    /// 1. Starting from the rightmost digit (check digit) and moving left,
    ///    double every second digit
    /// 2. If the result of doubling is greater than 9, subtract 9
    /// 3. Sum all the digits
    /// 4. If the total modulo 10 equals 0, the number is valid
    fn luhn_check(card_number: &str) -> bool {
        let digits: Vec<u32> = card_number.chars().filter_map(|c| c.to_digit(10)).collect();

        if digits.is_empty() {
            return false;
        }

        let mut sum = 0;
        let parity = digits.len() % 2;

        for (i, &digit) in digits.iter().enumerate() {
            let mut d = digit;
            if i % 2 == parity {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            sum += d;
        }

        sum % 10 == 0
    }

    /// Detects the credit card type based on the number prefix
    ///
    /// Card type detection is based on the Industry Identification Number (IIN),
    /// also known as the Bank Identification Number (BIN), which is the first
    /// 6 digits of the card number.
    ///
    /// Rules:
    /// - Visa: starts with 4
    /// - MasterCard: starts with 51-55 or 2221-2720
    /// - American Express: starts with 34 or 37
    /// - Discover: starts with 6011, 622126-622925, 644-649, or 65
    /// - JCB: starts with 3528-3589
    /// - Diners Club: starts with 300-305, 36, or 38
    fn detect_card_type(card_number: &str) -> Option<CardType> {
        if card_number.is_empty() {
            return None;
        }

        // Check single digit prefix
        if card_number.starts_with('4') {
            return Some(CardType::Visa);
        }

        // Check two-digit prefixes
        if card_number.len() >= 2 {
            let prefix2: u32 = card_number[0..2].parse().unwrap_or(0);

            if prefix2 == 34 || prefix2 == 37 {
                return Some(CardType::AmericanExpress);
            }

            if (51..=55).contains(&prefix2) {
                return Some(CardType::MasterCard);
            }

            if prefix2 == 36 || prefix2 == 38 {
                return Some(CardType::DinersClub);
            }

            if prefix2 == 65 {
                return Some(CardType::Discover);
            }
        }

        // Check three-digit prefixes
        if card_number.len() >= 3 {
            let prefix3: u32 = card_number[0..3].parse().unwrap_or(0);

            if (300..=305).contains(&prefix3) {
                return Some(CardType::DinersClub);
            }

            if (644..=649).contains(&prefix3) {
                return Some(CardType::Discover);
            }
        }

        // Check four-digit prefixes
        if card_number.len() >= 4 {
            let prefix4: u32 = card_number[0..4].parse().unwrap_or(0);

            if prefix4 == 6011 {
                return Some(CardType::Discover);
            }

            if (2221..=2720).contains(&prefix4) {
                return Some(CardType::MasterCard);
            }

            if (3528..=3589).contains(&prefix4) {
                return Some(CardType::JCB);
            }
        }

        // Check six-digit prefixes for Discover 622126-622925
        if card_number.len() >= 6 {
            let prefix6: u32 = card_number[0..6].parse().unwrap_or(0);
            if (622126..=622925).contains(&prefix6) {
                return Some(CardType::Discover);
            }
        }

        None
    }

    /// Gets the appropriate error for the given value
    fn error_for_value(&self, value: &str) -> ValidationError {
        if let Some(ref msg) = self.message {
            ValidationError::Custom(msg.clone())
        } else {
            ValidationError::InvalidCreditCard(value.to_string())
        }
    }
}

impl Default for CreditCardValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator<String> for CreditCardValidator {
    fn validate(&self, value: &String) -> ValidationResult<()> {
        self.validate(value.as_str()).map(|_| ())
    }
}

impl Validator<str> for CreditCardValidator {
    fn validate(&self, value: &str) -> ValidationResult<()> {
        self.validate(value).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luhn_algorithm_valid() {
        // Valid credit card numbers (using Luhn algorithm)
        assert!(CreditCardValidator::luhn_check("4532015112830366")); // Visa
        assert!(CreditCardValidator::luhn_check("5425233430109903")); // MasterCard
        assert!(CreditCardValidator::luhn_check("374245455400126")); // AmEx
        assert!(CreditCardValidator::luhn_check("6011111111111117")); // Discover
        assert!(CreditCardValidator::luhn_check("3530111333300000")); // JCB
    }

    #[test]
    fn test_luhn_algorithm_invalid() {
        // Invalid credit card numbers (failing Luhn check)
        assert!(!CreditCardValidator::luhn_check("4532015112830367")); // Last digit wrong
        assert!(!CreditCardValidator::luhn_check("1234567890123456")); // Random numbers
        assert!(!CreditCardValidator::luhn_check("1111111111111111")); // All ones (invalid)
        assert!(!CreditCardValidator::luhn_check("9999999999999999")); // All nines (invalid)
    }

    #[test]
    fn test_card_type_detection_visa() {
        let card_type = CreditCardValidator::detect_card_type("4532015112830366");
        assert_eq!(card_type, Some(CardType::Visa));
    }

    #[test]
    fn test_card_type_detection_mastercard() {
        // Old MasterCard range (51-55)
        assert_eq!(
            CreditCardValidator::detect_card_type("5425233430109903"),
            Some(CardType::MasterCard)
        );

        // New MasterCard range (2221-2720)
        assert_eq!(
            CreditCardValidator::detect_card_type("2221000000000009"),
            Some(CardType::MasterCard)
        );
        assert_eq!(
            CreditCardValidator::detect_card_type("2720999999999996"),
            Some(CardType::MasterCard)
        );
    }

    #[test]
    fn test_card_type_detection_amex() {
        assert_eq!(
            CreditCardValidator::detect_card_type("374245455400126"),
            Some(CardType::AmericanExpress)
        );
        assert_eq!(
            CreditCardValidator::detect_card_type("340000000000009"),
            Some(CardType::AmericanExpress)
        );
    }

    #[test]
    fn test_card_type_detection_discover() {
        // 6011 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("6011111111111117"),
            Some(CardType::Discover)
        );

        // 65 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("6500000000000002"),
            Some(CardType::Discover)
        );

        // 644-649 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("6440000000000004"),
            Some(CardType::Discover)
        );
    }

    #[test]
    fn test_card_type_detection_jcb() {
        assert_eq!(
            CreditCardValidator::detect_card_type("3530111333300000"),
            Some(CardType::JCB)
        );
        assert_eq!(
            CreditCardValidator::detect_card_type("3589111333300003"),
            Some(CardType::JCB)
        );
    }

    #[test]
    fn test_card_type_detection_diners_club() {
        // 300-305 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("30000000000004"),
            Some(CardType::DinersClub)
        );
        assert_eq!(
            CreditCardValidator::detect_card_type("30500000000003"),
            Some(CardType::DinersClub)
        );

        // 36 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("36000000000008"),
            Some(CardType::DinersClub)
        );

        // 38 prefix
        assert_eq!(
            CreditCardValidator::detect_card_type("38000000000006"),
            Some(CardType::DinersClub)
        );
    }

    #[test]
    fn test_validator_with_valid_cards() {
        let validator = CreditCardValidator::new();

        // Test various valid card numbers
        assert!(validator.validate("4532015112830366").is_ok()); // Visa
        assert!(validator.validate("5425233430109903").is_ok()); // MasterCard
        assert!(validator.validate("374245455400126").is_ok()); // AmEx
        assert!(validator.validate("6011111111111117").is_ok()); // Discover
        assert!(validator.validate("3530111333300000").is_ok()); // JCB
    }

    #[test]
    fn test_validator_with_formatted_cards() {
        let validator = CreditCardValidator::new();

        // Test with dashes
        assert!(validator.validate("4532-0151-1283-0366").is_ok());

        // Test with spaces
        assert!(validator.validate("4532 0151 1283 0366").is_ok());

        // Test with mixed formatting
        assert!(validator.validate("4532 0151-1283 0366").is_ok());
    }

    #[test]
    fn test_validator_with_invalid_cards() {
        let validator = CreditCardValidator::new();

        // Invalid Luhn check
        assert!(validator.validate("4532015112830367").is_err());

        // Contains non-digits
        assert!(validator.validate("4532015112830366a").is_err());
        assert!(validator.validate("abc4532015112830366").is_err());

        // Empty string
        assert!(validator.validate("").is_err());
    }

    #[test]
    fn test_validator_with_allowed_types() {
        let validator =
            CreditCardValidator::new().allow_types(vec![CardType::Visa, CardType::MasterCard]);

        // Visa should pass
        assert!(validator.validate("4532015112830366").is_ok());

        // MasterCard should pass
        assert!(validator.validate("5425233430109903").is_ok());

        // AmEx should fail (not in allowed types)
        match validator.validate("374245455400126") {
            Err(ValidationError::CardTypeNotAllowed { .. }) => {}
            _ => panic!("Expected CardTypeNotAllowed error"),
        }
    }

    #[test]
    fn test_validator_returns_card_type() {
        let validator = CreditCardValidator::new();

        assert_eq!(
            validator.validate("4532015112830366").unwrap(),
            CardType::Visa
        );
        assert_eq!(
            validator.validate("5425233430109903").unwrap(),
            CardType::MasterCard
        );
        assert_eq!(
            validator.validate("374245455400126").unwrap(),
            CardType::AmericanExpress
        );
    }

    #[test]
    fn test_validator_with_custom_message() {
        let validator = CreditCardValidator::new().with_message("Custom error message");

        match validator.validate("invalid") {
            Err(ValidationError::Custom(msg)) => {
                assert_eq!(msg, "Custom error message");
            }
            _ => panic!("Expected Custom error with custom message"),
        }
    }

    #[test]
    fn test_validator_trait_implementation() {
        let validator = CreditCardValidator::new();

        // Test Validator<str> trait
        assert!(Validator::<str>::validate(&validator, "4532015112830366").is_ok());
        assert!(Validator::<str>::validate(&validator, "invalid").is_err());

        // Test Validator<String> trait
        let card_string = String::from("4532015112830366");
        assert!(Validator::<String>::validate(&validator, &card_string).is_ok());

        let invalid_string = String::from("invalid");
        assert!(Validator::<String>::validate(&validator, &invalid_string).is_err());
    }

    #[test]
    fn test_card_type_display() {
        assert_eq!(CardType::Visa.to_string(), "Visa");
        assert_eq!(CardType::MasterCard.to_string(), "MasterCard");
        assert_eq!(CardType::AmericanExpress.to_string(), "American Express");
        assert_eq!(CardType::Discover.to_string(), "Discover");
        assert_eq!(CardType::JCB.to_string(), "JCB");
        assert_eq!(CardType::DinersClub.to_string(), "Diners Club");
    }

    #[test]
    fn test_card_type_as_str() {
        assert_eq!(CardType::Visa.as_str(), "Visa");
        assert_eq!(CardType::MasterCard.as_str(), "MasterCard");
        assert_eq!(CardType::AmericanExpress.as_str(), "American Express");
    }

    #[test]
    fn test_default_validator() {
        let validator = CreditCardValidator::default();
        assert!(validator.validate("4532015112830366").is_ok());
    }

    #[test]
    fn test_card_type_equality() {
        assert_eq!(CardType::Visa, CardType::Visa);
        assert_ne!(CardType::Visa, CardType::MasterCard);
    }

    #[test]
    fn test_error_messages() {
        let validator = CreditCardValidator::new();

        // Test invalid card error
        match validator.validate("1234567890123456") {
            Err(ValidationError::InvalidCreditCard(card)) => {
                assert_eq!(card, "1234567890123456");
            }
            _ => panic!("Expected InvalidCreditCard error"),
        }

        // Test card type not allowed error
        let restricted_validator = CreditCardValidator::new().allow_types(vec![CardType::Visa]);
        match restricted_validator.validate("5425233430109903") {
            Err(ValidationError::CardTypeNotAllowed {
                card_type,
                allowed_types,
            }) => {
                assert_eq!(card_type, "MasterCard");
                assert_eq!(allowed_types, "Visa");
            }
            _ => panic!("Expected CardTypeNotAllowed error"),
        }
    }

    // Edge cases
    #[test]
    fn test_edge_cases() {
        let validator = CreditCardValidator::new();

        // All zeros (valid by Luhn but unknown card type)
        assert!(validator.validate("0000000000000000").is_err());

        // Very short valid number (unknown card type)
        assert!(validator.validate("0").is_err()); // Single 0 passes Luhn but unknown type

        // Only dashes and spaces
        assert!(validator.validate("---   ").is_err());
    }

    #[test]
    fn test_builder_pattern() {
        let validator = CreditCardValidator::new()
            .allow_types(vec![CardType::Visa])
            .with_message("Please enter a valid Visa card");

        match validator.validate("5425233430109903") {
            Err(ValidationError::CardTypeNotAllowed { .. }) => {}
            _ => panic!("Expected CardTypeNotAllowed error"),
        }

        match validator.validate("invalid") {
            Err(ValidationError::Custom(msg)) => {
                assert_eq!(msg, "Please enter a valid Visa card");
            }
            _ => panic!("Expected Custom error"),
        }
    }
}
