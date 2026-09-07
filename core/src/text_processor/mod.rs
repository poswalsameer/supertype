//! Local LLM extension point (Phase 4).
//! Production pipeline remains: ASR → DeterministicFormatter → insertion
//! This establishes TextProcessor abstraction for optional local LLM.

use std::collections::HashMap;

pub trait TextProcessor: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, text: &str, context: &ProcessorContext) -> String;
}

pub struct ProcessorContext {
    pub app_name: Option<String>,
    pub bundle_id: Option<String>,
    pub dictionary: HashMap<String, String>,
}

/// Deterministic formatter (default)
pub struct DeterministicProcessor;

impl TextProcessor for DeterministicProcessor {
    fn name(&self) -> &str { "deterministic" }
    fn process(&self, text: &str, _ctx: &ProcessorContext) -> String {
        let formatted = crate::formatting::format_transcript(text);
        crate::formatting::apply_dictionary(&formatted, &_ctx.dictionary)
    }
}

/// Optional local LLM processor (stub, disabled by default)
/// Would wrap a local LLM (e.g. Qwen 0.5B quantized) for grammar correction.
pub struct LocalLLMProcessor {
    model_path: Option<std::path::PathBuf>,
}

impl LocalLLMProcessor {
    pub fn new(model_path: Option<std::path::PathBuf>) -> Self { Self { model_path } }
    pub fn is_available(&self) -> bool { self.model_path.as_ref().map(|p| p.exists()).unwrap_or(false) }
}

impl TextProcessor for LocalLLMProcessor {
    fn name(&self) -> &str { "local_llm" }
    fn process(&self, text: &str, ctx: &ProcessorContext) -> String {
        // If LLM not available, fallback to deterministic
        if !self.is_available() {
            return DeterministicProcessor.process(text, ctx);
        }
        // Stub: would call LLM with prompt, but we just apply deterministic + marker
        let base = DeterministicProcessor.process(text, ctx);
        format!("{} [llm-enhanced]", base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn deterministic_processes() {
        let p = DeterministicProcessor;
        let ctx = ProcessorContext { app_name: None, bundle_id: None, dictionary: HashMap::new() };
        assert_eq!(p.process("hello comma world", &ctx), "Hello, world");
    }

    #[test]
    fn llm_fallback_when_no_model() {
        let p = LocalLLMProcessor::new(None);
        let ctx = ProcessorContext { app_name: None, bundle_id: None, dictionary: HashMap::new() };
        assert!(!p.is_available());
        assert_eq!(p.process("hello", &ctx), "Hello");
    }

    #[test]
    fn llm_available_check() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("llm.bin");
        std::fs::write(&path, b"fake").unwrap();
        let p = LocalLLMProcessor::new(Some(path.clone()));
        assert!(p.is_available());
    }
}
