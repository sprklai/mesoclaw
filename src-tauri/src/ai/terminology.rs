//! Database-agnostic terminology for AI prompts.
//!
//! COMMENTED OUT: This module is database-specific and has been removed from the boilerplate.
//! Re-implement if you need database paradigm-specific terminology.

/*
// All terminology functionality commented out - database-specific
// Re-implement for database-specific use cases

use crate::database::metadata::DatabaseParadigm;

#[derive(Debug, Clone)]
pub struct Terminology {
    paradigm: DatabaseParadigm,
}

impl Terminology {
    pub fn for_paradigm(paradigm: DatabaseParadigm) -> Self {
        todo!("Database-specific terminology removed")
    }

    pub fn container(&self) -> &str { todo!() }
    pub fn containers(&self) -> &str { todo!() }
    pub fn item(&self) -> &str { todo!() }
    pub fn items(&self) -> &str { todo!() }
    pub fn attribute(&self) -> &str { todo!() }
    pub fn attributes(&self) -> &str { todo!() }
    pub fn reference(&self) -> &str { todo!() }
    pub fn references(&self) -> &str { todo!() }
}
*/
