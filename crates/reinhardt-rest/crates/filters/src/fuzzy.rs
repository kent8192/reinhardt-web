//! Fuzzy search filter
//!
//! Provides fuzzy string matching with configurable similarity algorithms.

use std::marker::PhantomData;

/// Fuzzy search algorithm
///
/// Determines how similarity is calculated between strings.
///
/// # Examples
///
/// ```
/// use reinhardt_filters::FuzzyAlgorithm;
///
/// let levenshtein = FuzzyAlgorithm::Levenshtein;
/// let jaro_winkler = FuzzyAlgorithm::JaroWinkler;
/// let trigram = FuzzyAlgorithm::Trigram;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FuzzyAlgorithm {
    /// Levenshtein distance
    ///
    /// Measures the minimum number of single-character edits
    /// (insertions, deletions, or substitutions) required to
    /// change one string into another.
    Levenshtein,

    /// Damerau-Levenshtein distance
    ///
    /// Extension of Levenshtein that also considers transpositions
    /// (swapping of two adjacent characters).
    DamerauLevenshtein,

    /// Jaro-Winkler similarity
    ///
    /// Measures similarity based on matching characters and transpositions,
    /// with a prefix scale giving more favorable ratings to strings with
    /// common prefixes.
    JaroWinkler,

    /// Trigram similarity
    ///
    /// Measures similarity based on the number of common three-character
    /// sequences (trigrams) between strings.
    Trigram,

    /// Soundex
    ///
    /// Phonetic algorithm for indexing names by sound.
    /// Useful for matching names despite spelling variations.
    Soundex,

    /// Metaphone
    ///
    /// More sophisticated phonetic algorithm than Soundex.
    /// Better handles consonant sounds and pronunciation.
    Metaphone,
}

impl Default for FuzzyAlgorithm {
    fn default() -> Self {
        FuzzyAlgorithm::Levenshtein
    }
}

/// Fuzzy search filter
///
/// Provides fuzzy string matching for inexact queries.
///
/// # Type Parameters
///
/// * `M` - The model type being searched
///
/// # Examples
///
/// ```
/// use reinhardt_filters::{FuzzySearchFilter, FuzzyAlgorithm};
///
/// #[derive(Clone)]
/// struct User {
///     id: i64,
///     name: String,
///     email: String,
/// }
///
/// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
///     .query("Jon Doe")
///     .field("name")
///     .threshold(0.8)
///     .algorithm(FuzzyAlgorithm::JaroWinkler);
/// ```
#[derive(Debug, Clone)]
pub struct FuzzySearchFilter<M> {
    /// The search query
    pub query: String,
    /// Field to search in
    pub field: String,
    /// Similarity threshold (0.0 - 1.0)
    pub threshold: f64,
    /// Fuzzy matching algorithm
    pub algorithm: FuzzyAlgorithm,
    /// Maximum edit distance for Levenshtein-based algorithms
    pub max_distance: Option<usize>,
    /// Case sensitivity
    pub case_sensitive: bool,
    /// Prefix length for Jaro-Winkler
    pub prefix_length: usize,
    _phantom: PhantomData<M>,
}

impl<M> FuzzySearchFilter<M> {
    /// Create a new fuzzy search filter
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct Article {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<Article> = FuzzySearchFilter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            query: String::new(),
            field: String::new(),
            threshold: 0.8,
            algorithm: FuzzyAlgorithm::default(),
            max_distance: None,
            case_sensitive: false,
            prefix_length: 4,
            _phantom: PhantomData,
        }
    }

    /// Set the search query
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .query("Jon Doe");
    /// assert_eq!(filter.query(), "Jon Doe");
    /// ```
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Set the field to search in
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .field("name");
    /// assert_eq!(filter.field(), "name");
    /// ```
    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = field.into();
        self
    }

    /// Set the similarity threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .threshold(0.9);
    /// assert_eq!(filter.threshold(), 0.9);
    /// ```
    pub fn threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Set the fuzzy matching algorithm
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::{FuzzySearchFilter, FuzzyAlgorithm};
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .algorithm(FuzzyAlgorithm::JaroWinkler);
    /// assert_eq!(filter.algorithm(), FuzzyAlgorithm::JaroWinkler);
    /// ```
    pub fn algorithm(mut self, algorithm: FuzzyAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Set the maximum edit distance
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .max_distance(2);
    /// assert_eq!(filter.max_distance(), Some(2));
    /// ```
    pub fn max_distance(mut self, distance: usize) -> Self {
        self.max_distance = Some(distance);
        self
    }

    /// Set case sensitivity
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .case_sensitive(true);
    /// assert!(filter.is_case_sensitive());
    /// ```
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Set the prefix length for Jaro-Winkler
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .prefix_length(6);
    /// assert_eq!(filter.prefix_length(), 6);
    /// ```
    pub fn prefix_length(mut self, length: usize) -> Self {
        self.prefix_length = length;
        self
    }

    /// Get the search query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Get the field to search in
    pub fn field(&self) -> &str {
        &self.field
    }

    /// Get the similarity threshold
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Get the fuzzy matching algorithm
    pub fn algorithm(&self) -> FuzzyAlgorithm {
        self.algorithm
    }

    /// Get the maximum edit distance
    pub fn max_distance(&self) -> Option<usize> {
        self.max_distance
    }

    /// Check if case sensitive
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Get the prefix length for Jaro-Winkler
    pub fn prefix_length(&self) -> usize {
        self.prefix_length
    }

    /// Calculate similarity between two strings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::{FuzzySearchFilter, FuzzyAlgorithm};
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .algorithm(FuzzyAlgorithm::Levenshtein);
    ///
    /// let similarity = filter.calculate_similarity("kitten", "sitting");
    /// assert!(similarity >= 0.0 && similarity <= 1.0);
    /// ```
    pub fn calculate_similarity(&self, a: &str, b: &str) -> f64 {
        let a = if self.case_sensitive {
            a.to_string()
        } else {
            a.to_lowercase()
        };
        let b = if self.case_sensitive {
            b.to_string()
        } else {
            b.to_lowercase()
        };

        match self.algorithm {
            FuzzyAlgorithm::Levenshtein => self.levenshtein_similarity(&a, &b),
            FuzzyAlgorithm::DamerauLevenshtein => self.damerau_levenshtein_similarity(&a, &b),
            FuzzyAlgorithm::JaroWinkler => self.jaro_winkler_similarity(&a, &b),
            FuzzyAlgorithm::Trigram => self.trigram_similarity(&a, &b),
            FuzzyAlgorithm::Soundex => self.soundex_similarity(&a, &b),
            FuzzyAlgorithm::Metaphone => self.metaphone_similarity(&a, &b),
        }
    }

    /// Check if two strings match within the threshold
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_filters::FuzzySearchFilter;
    ///
    /// #[derive(Clone)]
    /// struct User {
    ///     id: i64,
    /// }
    ///
    /// let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
    ///     .threshold(0.8);
    ///
    /// assert!(filter.matches("John Doe", "Jon Doe"));
    /// ```
    pub fn matches(&self, a: &str, b: &str) -> bool {
        let similarity = self.calculate_similarity(a, b);
        similarity >= self.threshold
    }

    fn levenshtein_similarity(&self, a: &str, b: &str) -> f64 {
        let distance = self.levenshtein_distance(a, b);
        let max_len = a.len().max(b.len());
        if max_len == 0 {
            return 1.0;
        }
        1.0 - (distance as f64 / max_len as f64)
    }

    fn levenshtein_distance(&self, a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 {
            return b_len;
        }
        if b_len == 0 {
            return a_len;
        }

        let mut prev_row: Vec<usize> = (0..=b_len).collect();
        let mut curr_row = vec![0; b_len + 1];

        for i in 1..=a_len {
            curr_row[0] = i;
            for j in 1..=b_len {
                let cost = if a_chars[i - 1] == b_chars[j - 1] {
                    0
                } else {
                    1
                };
                curr_row[j] = (curr_row[j - 1] + 1)
                    .min(prev_row[j] + 1)
                    .min(prev_row[j - 1] + cost);
            }
            std::mem::swap(&mut prev_row, &mut curr_row);
        }

        prev_row[b_len]
    }

    fn damerau_levenshtein_similarity(&self, a: &str, b: &str) -> f64 {
        // Simplified: uses Levenshtein for now
        // Full implementation would consider transpositions
        self.levenshtein_similarity(a, b)
    }

    fn jaro_winkler_similarity(&self, a: &str, b: &str) -> f64 {
        let jaro = self.jaro_similarity(a, b);
        let prefix_len = self.common_prefix_length(a, b).min(self.prefix_length);
        jaro + (prefix_len as f64 * 0.1 * (1.0 - jaro))
    }

    fn jaro_similarity(&self, a: &str, b: &str) -> f64 {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        let a_len = a_chars.len();
        let b_len = b_chars.len();

        if a_len == 0 && b_len == 0 {
            return 1.0;
        }
        if a_len == 0 || b_len == 0 {
            return 0.0;
        }

        let match_window = (a_len.max(b_len) / 2).saturating_sub(1);
        let mut a_matches = vec![false; a_len];
        let mut b_matches = vec![false; b_len];
        let mut matches = 0.0;
        let mut transpositions = 0.0;

        for i in 0..a_len {
            let start = i.saturating_sub(match_window);
            let end = (i + match_window + 1).min(b_len);
            for j in start..end {
                if !b_matches[j] && a_chars[i] == b_chars[j] {
                    a_matches[i] = true;
                    b_matches[j] = true;
                    matches += 1.0;
                    break;
                }
            }
        }

        if matches == 0.0 {
            return 0.0;
        }

        let mut k = 0;
        for i in 0..a_len {
            if a_matches[i] {
                while !b_matches[k] {
                    k += 1;
                }
                if a_chars[i] != b_chars[k] {
                    transpositions += 1.0;
                }
                k += 1;
            }
        }

        (matches / a_len as f64
            + matches / b_len as f64
            + (matches - transpositions / 2.0) / matches)
            / 3.0
    }

    fn trigram_similarity(&self, a: &str, b: &str) -> f64 {
        let a_trigrams = self.get_trigrams(a);
        let b_trigrams = self.get_trigrams(b);

        if a_trigrams.is_empty() && b_trigrams.is_empty() {
            return 1.0;
        }
        if a_trigrams.is_empty() || b_trigrams.is_empty() {
            return 0.0;
        }

        let common = a_trigrams
            .iter()
            .filter(|t| b_trigrams.contains(t))
            .count();
        let total = a_trigrams.len() + b_trigrams.len();

        (2.0 * common as f64) / total as f64
    }

    fn get_trigrams(&self, s: &str) -> Vec<String> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 3 {
            return vec![];
        }
        (0..=chars.len() - 3)
            .map(|i| chars[i..i + 3].iter().collect())
            .collect()
    }

    fn soundex_similarity(&self, a: &str, b: &str) -> f64 {
        if self.soundex_code(a) == self.soundex_code(b) {
            1.0
        } else {
            0.0
        }
    }

    fn soundex_code(&self, s: &str) -> String {
        let chars: Vec<char> = s.chars().filter(|c| c.is_alphabetic()).collect();
        if chars.is_empty() {
            return String::from("0000");
        }

        let first = chars[0].to_uppercase().next().unwrap();
        let mut code = String::from(first);
        let mut prev_code = self.soundex_digit(first);

        for &c in &chars[1..] {
            let digit = self.soundex_digit(c);
            if digit != '0' && digit != prev_code {
                code.push(digit);
                if code.len() == 4 {
                    break;
                }
            }
            if digit != '0' {
                prev_code = digit;
            }
        }

        while code.len() < 4 {
            code.push('0');
        }

        code
    }

    fn soundex_digit(&self, c: char) -> char {
        match c.to_uppercase().next().unwrap() {
            'B' | 'F' | 'P' | 'V' => '1',
            'C' | 'G' | 'J' | 'K' | 'Q' | 'S' | 'X' | 'Z' => '2',
            'D' | 'T' => '3',
            'L' => '4',
            'M' | 'N' => '5',
            'R' => '6',
            _ => '0',
        }
    }

    fn metaphone_similarity(&self, a: &str, b: &str) -> f64 {
        // Simplified: uses Soundex for now
        // Full implementation would use the Metaphone algorithm
        self.soundex_similarity(a, b)
    }

    fn common_prefix_length(&self, a: &str, b: &str) -> usize {
        let a_chars: Vec<char> = a.chars().collect();
        let b_chars: Vec<char> = b.chars().collect();
        a_chars
            .iter()
            .zip(b_chars.iter())
            .take_while(|(a, b)| a == b)
            .count()
    }
}

impl<M> Default for FuzzySearchFilter<M> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct User {
        id: i64,
        name: String,
    }

    #[test]
    fn test_fuzzy_algorithm_default() {
        assert_eq!(FuzzyAlgorithm::default(), FuzzyAlgorithm::Levenshtein);
    }

    #[test]
    fn test_filter_creation() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        assert_eq!(filter.query(), "");
        assert_eq!(filter.field(), "");
        assert_eq!(filter.threshold(), 0.8);
        assert_eq!(filter.algorithm(), FuzzyAlgorithm::Levenshtein);
        assert_eq!(filter.max_distance(), None);
        assert!(!filter.is_case_sensitive());
    }

    #[test]
    fn test_filter_builder() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .query("Jon Doe")
            .field("name")
            .threshold(0.9)
            .algorithm(FuzzyAlgorithm::JaroWinkler)
            .max_distance(2)
            .case_sensitive(true)
            .prefix_length(6);

        assert_eq!(filter.query(), "Jon Doe");
        assert_eq!(filter.field(), "name");
        assert_eq!(filter.threshold(), 0.9);
        assert_eq!(filter.algorithm(), FuzzyAlgorithm::JaroWinkler);
        assert_eq!(filter.max_distance(), Some(2));
        assert!(filter.is_case_sensitive());
        assert_eq!(filter.prefix_length(), 6);
    }

    #[test]
    fn test_threshold_clamping() {
        let filter1: FuzzySearchFilter<User> = FuzzySearchFilter::new().threshold(1.5);
        assert_eq!(filter1.threshold(), 1.0);

        let filter2: FuzzySearchFilter<User> = FuzzySearchFilter::new().threshold(-0.5);
        assert_eq!(filter2.threshold(), 0.0);
    }

    #[test]
    fn test_levenshtein_distance() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        assert_eq!(filter.levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(filter.levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(filter.levenshtein_distance("", "abc"), 3);
        assert_eq!(filter.levenshtein_distance("abc", ""), 3);
    }

    #[test]
    fn test_levenshtein_similarity() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .algorithm(FuzzyAlgorithm::Levenshtein);

        assert_eq!(filter.calculate_similarity("abc", "abc"), 1.0);
        assert!(filter.calculate_similarity("kitten", "sitting") > 0.5);
    }

    #[test]
    fn test_jaro_winkler_similarity() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .algorithm(FuzzyAlgorithm::JaroWinkler);

        assert_eq!(filter.calculate_similarity("abc", "abc"), 1.0);
        assert!(filter.calculate_similarity("martha", "marhta") > 0.9);
        assert!(filter.calculate_similarity("dixon", "dicksonx") > 0.7);
    }

    #[test]
    fn test_trigram_similarity() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .algorithm(FuzzyAlgorithm::Trigram);

        assert_eq!(filter.calculate_similarity("abc", "abc"), 1.0);
        assert!(filter.calculate_similarity("hello", "hallo") > 0.5);
    }

    #[test]
    fn test_soundex_code() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        assert_eq!(filter.soundex_code("Smith"), "S530");
        assert_eq!(filter.soundex_code("Smythe"), "S530");
        assert_eq!(filter.soundex_code("Johnson"), "J525");
        assert_eq!(filter.soundex_code("Jonson"), "J525");
    }

    #[test]
    fn test_soundex_similarity() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .algorithm(FuzzyAlgorithm::Soundex);

        assert_eq!(filter.calculate_similarity("Smith", "Smythe"), 1.0);
        assert_eq!(filter.calculate_similarity("Johnson", "Jonson"), 1.0);
        assert_eq!(filter.calculate_similarity("Smith", "Johnson"), 0.0);
    }

    #[test]
    fn test_matches() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .threshold(0.8)
            .algorithm(FuzzyAlgorithm::Levenshtein);

        assert!(filter.matches("hello", "hello"));
        assert!(filter.matches("hello", "helo"));
        assert!(!filter.matches("hello", "world"));
    }

    #[test]
    fn test_case_sensitivity() {
        let sensitive: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .case_sensitive(true)
            .algorithm(FuzzyAlgorithm::Levenshtein);

        let insensitive: FuzzySearchFilter<User> = FuzzySearchFilter::new()
            .case_sensitive(false)
            .algorithm(FuzzyAlgorithm::Levenshtein);

        assert!(sensitive.calculate_similarity("Hello", "hello") < 1.0);
        assert_eq!(insensitive.calculate_similarity("Hello", "hello"), 1.0);
    }

    #[test]
    fn test_get_trigrams() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        let trigrams = filter.get_trigrams("hello");
        assert_eq!(trigrams.len(), 3);
        assert!(trigrams.contains(&"hel".to_string()));
        assert!(trigrams.contains(&"ell".to_string()));
        assert!(trigrams.contains(&"llo".to_string()));
    }

    #[test]
    fn test_common_prefix_length() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        assert_eq!(filter.common_prefix_length("hello", "help"), 3);
        assert_eq!(filter.common_prefix_length("abc", "def"), 0);
        assert_eq!(filter.common_prefix_length("test", "test"), 4);
    }

    #[test]
    fn test_empty_strings() {
        let filter: FuzzySearchFilter<User> = FuzzySearchFilter::new();
        assert_eq!(filter.calculate_similarity("", ""), 1.0);
        assert_eq!(filter.calculate_similarity("", "abc"), 0.0);
        assert_eq!(filter.calculate_similarity("abc", ""), 0.0);
    }
}
