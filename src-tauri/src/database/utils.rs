//! Database utility functions for SQLite type conversions
//!
//! SQLite stores booleans as integers (0/1), so we need conversion functions.

/// Convert a boolean to an integer (for SQLite storage)
pub fn bool_to_int(b: bool) -> i32 {
    if b { 1 } else { 0 }
}

/// Convert an integer to a boolean (from SQLite storage)
pub fn int_to_bool(i: i32) -> bool {
    i != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool_to_int() {
        assert_eq!(bool_to_int(true), 1);
        assert_eq!(bool_to_int(false), 0);
    }

    #[test]
    fn test_int_to_bool() {
        assert!(int_to_bool(1));
        assert!(!int_to_bool(0));
        assert!(int_to_bool(42)); // Any non-zero is true
        assert!(int_to_bool(-1)); // Negative is also true
    }
}
