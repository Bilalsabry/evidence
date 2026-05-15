//! Deterministic in-process [`LlmBackend`] implementations for tests and
//! smoke checks. None of these talk to the network.

use super::{
    LlmBackend, LlmError, Prompt, RawAnswer, RawSentence, SupportChecker, SupportError,
    SupportVerdict,
};

/// Backend that echoes the top retrieved chunk as a single citing sentence.
/// Useful for end-to-end smoke tests where the goal is to verify the
/// pipeline plumbing, not the model's intelligence.
#[derive(Default)]
pub struct MockBackend;

impl MockBackend {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl LlmBackend for MockBackend {
    fn answer(&self, prompt: &Prompt) -> Result<RawAnswer, LlmError> {
        let top = prompt
            .chunks
            .first()
            .ok_or_else(|| LlmError::Malformed("MockBackend: no chunks supplied".to_string()))?;
        Ok(RawAnswer {
            sentences: vec![RawSentence {
                text: top.text.clone(),
                span_ids: vec![top.span_id_start],
            }],
        })
    }
}

/// Backend that always refuses (returns [`LlmError::Refused`]). Exercises
/// the error-propagation path.
#[derive(Default)]
pub struct RefusingBackend;

impl LlmBackend for RefusingBackend {
    fn answer(&self, _prompt: &Prompt) -> Result<RawAnswer, LlmError> {
        Err(LlmError::Refused)
    }
}

/// Backend that returns a sentence with no citations. Exercises the
/// [`super::QueryError::Uncited`] path of the validator.
#[derive(Default)]
pub struct UncitedBackend;

impl LlmBackend for UncitedBackend {
    fn answer(&self, _prompt: &Prompt) -> Result<RawAnswer, LlmError> {
        Ok(RawAnswer {
            sentences: vec![RawSentence {
                text: "I am very confident about this.".to_string(),
                span_ids: Vec::new(),
            }],
        })
    }
}

/// Backend that cites a span ID that wasn't shown to it. Exercises the
/// [`super::QueryError::OutOfContext`] path.
pub struct OutOfContextBackend {
    pub bogus_span_id: i64,
}

impl LlmBackend for OutOfContextBackend {
    fn answer(&self, _prompt: &Prompt) -> Result<RawAnswer, LlmError> {
        Ok(RawAnswer {
            sentences: vec![RawSentence {
                text: "I'm citing a span you didn't show me.".to_string(),
                span_ids: vec![self.bogus_span_id],
            }],
        })
    }
}

/// [`SupportChecker`] that approves every sentence. Use with the standard
/// pipeline to confirm the validator threads support results through
/// correctly.
#[derive(Default)]
pub struct ApprovingSupport;

impl SupportChecker for ApprovingSupport {
    fn check(
        &self,
        _sentence: &str,
        _cited_texts: &[&str],
    ) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Supports)
    }
}

/// [`SupportChecker`] that returns `Neutral` for every sentence. Exercises
/// the [`super::QueryError::Unsupported`] path.
#[derive(Default)]
pub struct RejectingSupport;

impl SupportChecker for RejectingSupport {
    fn check(
        &self,
        _sentence: &str,
        _cited_texts: &[&str],
    ) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Neutral)
    }
}

/// [`SupportChecker`] that returns `Contradicts` for every sentence.
/// Exercises the [`super::QueryError::Contradicted`] path.
#[derive(Default)]
pub struct ContradictingSupport;

impl SupportChecker for ContradictingSupport {
    fn check(
        &self,
        _sentence: &str,
        _cited_texts: &[&str],
    ) -> Result<SupportVerdict, SupportError> {
        Ok(SupportVerdict::Contradicts)
    }
}
