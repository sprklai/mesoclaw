//! Embedded skills compiled into the binary.
//!
//! This directory is reserved for custom AI skills that you want to compile
//! directly into the application binary. Skills defined here are always available
//! without requiring external files.
//!
//! To add a custom embedded skill:
//! 1. Create a new .md file in this directory (e.g., `my_skill.md`)
//! 2. Add it to the HashMap below using `include_str!("my_skill.md")`
//!
//! For more information about the skills system, see README.md in this directory.

use std::collections::HashMap;

/// Get all embedded skills as (id, content) pairs.
///
/// Returns an empty HashMap by default. Add your custom skills here by:
/// 1. Creating a .md file in this directory
/// 2. Using `include_str!("filename.md")` to embed it at compile time
/// 3. Inserting it into the HashMap with a unique skill ID
pub fn get_embedded_skills() -> HashMap<String, &'static str> {
    // Example of how to add an embedded skill:
    // let mut skills = HashMap::new();
    // skills.insert(
    //     "my-custom-skill".to_string(),
    //     include_str!("my_custom_skill.md"),
    // );
    // skills

    HashMap::new()
}
