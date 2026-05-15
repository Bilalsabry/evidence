//! Lexical-support check for citations.
//!
//! The validator in [`super::answer_query`] enforces two rules out of the
//! box:
//!
//! 1. Every cited span exists in the database.
//! 2. Every cited span was in the chunks shown to the model.
//!
//! This module adds the third rule from [`docs/DESIGN.md`]: **the cited
//! spans must lexically support the claim**. The shipping implementation
//! uses a cross-encoder ([`super::super::retrieval::Reranker`]) as a proxy
//! for entailment — score above [`SUPPORTED_THRESHOLD`] is treated as
//! "supports", below as "rejects". A proper NLI model is a v0.3 follow-up
//! once we have eval data to set the threshold from.

use thiserror::Error;

use super::nli::NliError;
use crate::retrieval::{RerankError, Reranker};

/// Default score threshold above which a span is taken to support a
/// sentence. Picked conservatively from informal trials with
/// `bge-reranker-base`; tune from eval data when available.
pub const SUPPORTED_THRESHOLD: f32 = -2.0;

/// The verdict for one (sentence, supporting-spans) pair.
///
/// Three states, mirroring textbook NLI labels. `RerankerSupportChecker`
/// (the v0.2 proxy) emits only `Supports` and `Neutral` — it can't tell
/// `Contradicts` apart. The NLI cross-encoder shipped in v0.3 produces all
/// three. The validator refuses on either `Neutral` or `Contradicts`, but
/// keeps the two distinct so callers can render a more useful refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportVerdict {
    /// At least one cited span entails the sentence.
    Supports,
    /// No cited span entails the sentence, but none contradicts it either.
    Neutral,
    /// At least one cited span contradicts the sentence.
    Contradicts,
}

/// Errors surfaced by a [`SupportChecker`].
#[derive(Debug, Error)]
pub enum SupportError {
    #[error(transparent)]
    Rerank(#[from] RerankError),
    #[error(transparent)]
    Nli(#[from] NliError),
}

/// Decide whether a sentence is supported by the spans cited for it.
pub trait SupportChecker: Send + Sync {
    /// Returns the verdict for `sentence` given `cited_texts` — the literal
    /// text of every span cited for that sentence. An empty `cited_texts`
    /// **must** return [`SupportVerdict::Neutral`].
    ///
    /// # Errors
    ///
    /// Returns [`SupportError`] for any backend failure (e.g., reranker
    /// initialization or batch run failure).
    fn check(&self, sentence: &str, cited_texts: &[&str]) -> Result<SupportVerdict, SupportError>;
}

/// Default [`SupportChecker`] that scores each (sentence, span) pair with a
/// cross-encoder and votes "supports" if any score clears the threshold.
pub struct RerankerSupportChecker<'a> {
    reranker: &'a dyn Reranker,
    threshold: f32,
}

impl<'a> RerankerSupportChecker<'a> {
    /// Build a checker over `reranker` with the default
    /// [`SUPPORTED_THRESHOLD`].
    #[must_use]
    pub fn new(reranker: &'a dyn Reranker) -> Self {
        Self {
            reranker,
            threshold: SUPPORTED_THRESHOLD,
        }
    }

    /// Build a checker with a custom score threshold.
    #[must_use]
    pub fn with_threshold(reranker: &'a dyn Reranker, threshold: f32) -> Self {
        Self {
            reranker,
            threshold,
        }
    }
}

impl<'a> SupportChecker for RerankerSupportChecker<'a> {
    /// Cross-encoder relevance has no notion of contradiction — it only
    /// scores "how related is this?". So this implementation collapses the
    /// 3-state verdict into `Supports` or `Neutral` and never returns
    /// `Contradicts`. For real contradiction detection use the NLI-backed
    /// `NliSupportChecker` from [`super::nli`].
    fn check(&self, sentence: &str, cited_texts: &[&str]) -> Result<SupportVerdict, SupportError> {
        if cited_texts.is_empty() {
            return Ok(SupportVerdict::Neutral);
        }
        let hits = self.reranker.rerank(sentence, cited_texts)?;
        let supported = hits.iter().any(|h| h.score >= self.threshold);
        Ok(if supported {
            SupportVerdict::Supports
        } else {
            SupportVerdict::Neutral
        })
    }
}
