//! Text embedders.
//!
//! [`Embedder`] is the interface every retriever uses to project text into a
//! vector space. The shipping implementation is [`BgeSmall`], which wraps
//! `fastembed` and produces 384-dimensional vectors using
//! `bge-small-en-v1.5`. Tests can use any in-tree `impl Embedder` —
//! `tests/retrieval.rs` uses a deterministic byte-hash mock so the fast
//! path doesn't depend on downloading a model.

use std::sync::{Mutex, OnceLock};

use thiserror::Error;

/// Embedding dimension produced by [`BgeSmall`].
pub const BGE_SMALL_DIM: usize = 384;

/// Errors surfaced by an [`Embedder`] implementation.
#[derive(Debug, Error)]
pub enum EmbedError {
    /// The backend failed to initialize (model download, ONNX runtime, …).
    #[error("embedder backend failed to initialize: {0}")]
    Init(String),
    /// The backend rejected an `embed` call (bad input, runtime error, …).
    #[error("embed call failed: {0}")]
    Run(String),
}

/// Project text into a fixed-dimensional vector space.
///
/// All implementations must produce vectors of the same dimension across
/// successive calls; callers depend on this for the `vec0` virtual-table
/// column width.
pub trait Embedder: Send + Sync {
    /// Dimensionality of every vector returned by `embed*`.
    fn dim(&self) -> usize;

    /// Embed a single string.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::Run`] for backend-side failures.
    fn embed(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        self.embed_batch(&[text]).and_then(|mut v| {
            v.pop()
                .ok_or_else(|| EmbedError::Run("backend returned no rows".into()))
        })
    }

    /// Embed a batch in one call. Implementations may parallelize internally.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::Run`] for backend-side failures.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

/// `bge-small-en-v1.5` via `fastembed`. The model and the ONNX runtime are
/// downloaded on first construction and cached in the user's HF cache
/// directory.
///
/// `fastembed::TextEmbedding::embed` takes `&mut self`, so the inner model
/// lives behind a [`Mutex`] — every embed call acquires it briefly. For our
/// workload (one query at a time) the contention is negligible; if it ever
/// matters, the right fix is sharding by model instance, not lock-free
/// fastembed.
pub struct BgeSmall {
    inner: Mutex<fastembed::TextEmbedding>,
}

impl BgeSmall {
    /// Initialize the embedder. The first call downloads the model
    /// (~130 MB) and the ONNX runtime; subsequent calls reuse the cache.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::Init`] if the model download fails or the
    /// ONNX runtime cannot start.
    pub fn new() -> Result<Self, EmbedError> {
        use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
        let inner = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGESmallENV15))
            .map_err(|e| EmbedError::Init(e.to_string()))?;
        Ok(Self {
            inner: Mutex::new(inner),
        })
    }

    /// Process-wide shared instance. Subsequent callers reuse the same
    /// underlying model — avoids repeating the ~10 s warm-up cost.
    ///
    /// # Errors
    ///
    /// Returns [`EmbedError::Init`] if first-call initialization fails.
    pub fn shared() -> Result<&'static Self, EmbedError> {
        static SHARED: OnceLock<Result<BgeSmall, String>> = OnceLock::new();
        SHARED
            .get_or_init(|| Self::new().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(|e| EmbedError::Init(e.clone()))
    }
}

impl Embedder for BgeSmall {
    fn dim(&self) -> usize {
        BGE_SMALL_DIM
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        let owned: Vec<&str> = texts.to_vec();
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| EmbedError::Run(format!("embedder mutex poisoned: {e}")))?;
        guard
            .embed(owned, None)
            .map_err(|e| EmbedError::Run(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic, dependency-free embedder used by the rest of the crate
    /// when the goal is to test geometry rather than the embedding quality.
    pub struct HashEmbedder {
        pub dim: usize,
    }

    impl Embedder for HashEmbedder {
        fn dim(&self) -> usize {
            self.dim
        }

        fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            Ok(texts
                .iter()
                .map(|t| {
                    // Lowest-rent stable embedding: write each byte into one
                    // dimension, mod dim, normalized to [-1, 1].
                    let mut v = vec![0.0f32; self.dim];
                    for (i, b) in t.bytes().enumerate() {
                        v[i % self.dim] += (f32::from(b) - 128.0) / 128.0;
                    }
                    v
                })
                .collect())
        }
    }

    #[test]
    fn hash_embedder_has_stable_dim() {
        let e = HashEmbedder { dim: 16 };
        let v = e.embed("hello").unwrap();
        assert_eq!(v.len(), 16);
    }

    #[test]
    fn hash_embedder_is_deterministic() {
        let e = HashEmbedder { dim: 8 };
        let a = e.embed("evidence").unwrap();
        let b = e.embed("evidence").unwrap();
        assert_eq!(a, b);
    }
}
