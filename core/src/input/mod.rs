//! Text injection abstraction (Phase 3).
//! Swift implements the actual AX/clipboard; Rust defines the contract.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsertionMethod {
    Accessibility,
    ClipboardFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectionResult {
    Success { method: InsertionMethod },
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertionRequest {
    pub text: String,
    pub bundle_id: Option<String>,
    pub app_name: Option<String>,
}

impl InsertionRequest {
    pub fn new(
        text: impl Into<String>,
        bundle_id: Option<String>,
        app_name: Option<String>,
    ) -> Self {
        Self {
            text: text.into(),
            bundle_id,
            app_name,
        }
    }

    pub fn should_insert(&self, has_ax_permission: bool) -> bool {
        // If text is empty, don't insert
        !self.text.trim().is_empty() && (has_ax_permission || true) // clipboard fallback always available
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insertion_request_empty() {
        let req = InsertionRequest::new("", None, None);
        assert!(!req.should_insert(true));
        assert!(!req.should_insert(false));
    }

    #[test]
    fn insertion_request_valid() {
        let req = InsertionRequest::new("hello", Some("com.apple.TextEdit".into()), None);
        assert!(req.should_insert(false));
    }
}
