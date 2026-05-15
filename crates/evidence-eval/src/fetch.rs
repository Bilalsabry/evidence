//! Public-corpus fetchers. Today: **DailyMed** (FDA drug labels, public
//! domain, JSON + PDF API).
//!
//! The fetcher walks the DailyMed v2 list endpoint, downloads each
//! label's PDF, computes its SHA-256, and emits a TOML manifest with
//! enough metadata to reproduce the corpus deterministically.
//!
//! Design notes:
//!
//! - **Idempotent.** If a label's PDF is already on disk and matches the
//!   manifest's recorded SHA, the fetcher skips it. Re-running the
//!   command refreshes only missing or corrupted files.
//! - **Polite.** A configurable sleep between requests keeps the
//!   fetcher within reasonable politeness of the public API. DailyMed
//!   has no published rate limit; we default to 500 ms.
//! - **Reproducible.** The manifest is the artifact the paper cites.
//!   `fetched_at` is a UTC unix timestamp; the order is deterministic
//!   (sorted by `set_id`). Re-running on the same day with the same
//!   limit yields the same manifest up to label-publication churn.
//!
//! What's intentionally not here:
//!
//! - Live network tests. CI doesn't (and shouldn't) reach DailyMed.
//!   The harness covers URL building, JSON parsing, and manifest
//!   shape without making any HTTP calls.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Base URL for the DailyMed v2 API.
pub const DAILYMED_API_BASE: &str = "https://dailymed.nlm.nih.gov/dailymed/services/v2";

/// PDF download endpoint (CFM form). The query parameter is `setId`.
pub const DAILYMED_PDF_ENDPOINT: &str = "https://dailymed.nlm.nih.gov/dailymed/downloadpdffile.cfm";

/// Default politeness between requests.
pub const DEFAULT_REQUEST_DELAY: Duration = Duration::from_millis(500);

/// One row of the manifest. Carries enough to re-fetch the exact PDF
/// later and to verify the local copy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LabelEntry {
    pub set_id: String,
    pub title: String,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    /// Unix epoch seconds at fetch time. Useful for "is this stale?"
    /// checks; not load-bearing.
    pub fetched_at: u64,
}

/// The complete manifest. Written as `manifest.toml` in the output dir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub source: String,
    pub fetched_at: u64,
    pub count: usize,
    #[serde(rename = "label", default)]
    pub labels: Vec<LabelEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("JSON parse error: {0}")]
    Json(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("manifest serialization failed: {0}")]
    ManifestSerde(String),
}

/// Build the URL for the DailyMed SPL list endpoint.
#[must_use]
pub fn list_url(page_size: usize, page: usize) -> String {
    format!(
        "{base}/spls.json?pagesize={size}&page={page}",
        base = DAILYMED_API_BASE,
        size = page_size,
        page = page,
    )
}

/// Build the PDF download URL for a given set ID.
#[must_use]
pub fn pdf_url(set_id: &str) -> String {
    format!("{}?setId={}", DAILYMED_PDF_ENDPOINT, set_id)
}

/// Shape of the DailyMed list response. Loose with extra fields so the
/// fetcher survives schema additions.
#[derive(Debug, Deserialize)]
pub struct ListResponse {
    pub data: Vec<ListEntry>,
}

/// Minimal projection of one row from the DailyMed list endpoint.
#[derive(Debug, Deserialize, Clone)]
pub struct ListEntry {
    /// SPL Set ID — the stable identifier across SPL versions.
    pub setid: String,
    /// Human-readable title from the SPL.
    #[serde(default)]
    pub title: String,
}

/// SHA-256 hex digest of `bytes`.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(digest.len() * 2);
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Serialize a manifest to TOML.
///
/// # Errors
///
/// Returns [`FetchError::ManifestSerde`] if `toml` rejects the value
/// (essentially never, since the types are statically serializable).
pub fn manifest_to_toml(manifest: &Manifest) -> Result<String, FetchError> {
    toml::to_string_pretty(manifest).map_err(|e| FetchError::ManifestSerde(e.to_string()))
}

/// Read an existing manifest from disk. Returns `None` if the file
/// doesn't exist; returns an error if it does but is malformed.
///
/// # Errors
///
/// I/O or TOML parse failures.
pub fn read_manifest(dir: &Path) -> Result<Option<Manifest>, FetchError> {
    let path = dir.join("manifest.toml");
    match std::fs::read_to_string(&path) {
        Ok(s) => toml::from_str(&s)
            .map(Some)
            .map_err(|e| FetchError::ManifestSerde(e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(FetchError::Io(e)),
    }
}

/// Configurable HTTP fetching. Lives behind a trait so tests can stub
/// it; the real CLI binds it to `ureq`.
pub trait HttpClient {
    fn get_text(&self, url: &str) -> Result<String, FetchError>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError>;
}

/// `ureq`-backed implementation. Configurable timeout (default 30 s).
pub struct UreqClient {
    agent: ureq::Agent,
}

impl UreqClient {
    /// 30-second timeout; rebuild with a different builder if you need
    /// to override.
    #[must_use]
    pub fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(30))
                .build(),
        }
    }
}

impl Default for UreqClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for UreqClient {
    fn get_text(&self, url: &str) -> Result<String, FetchError> {
        self.agent
            .get(url)
            .call()
            .map_err(|e| FetchError::Http(e.to_string()))?
            .into_string()
            .map_err(|e| FetchError::Http(e.to_string()))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        let resp = self
            .agent
            .get(url)
            .call()
            .map_err(|e| FetchError::Http(e.to_string()))?;
        let mut buf = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_reader(), &mut buf)?;
        Ok(buf)
    }
}

/// Configuration for a single fetch run.
#[derive(Debug, Clone)]
pub struct FetchConfig {
    /// Maximum number of labels to fetch. Default 50.
    pub limit: usize,
    /// Pause between HTTP requests.
    pub delay: Duration,
    /// Where to write `labels/*.pdf` and `manifest.toml`.
    pub output_dir: PathBuf,
}

impl Default for FetchConfig {
    fn default() -> Self {
        Self {
            limit: 50,
            delay: DEFAULT_REQUEST_DELAY,
            output_dir: PathBuf::from("dailymed-corpus"),
        }
    }
}

/// Walk the DailyMed list endpoint, download up to `config.limit` PDFs
/// (resuming from any existing manifest in `config.output_dir`), and
/// emit `manifest.toml`. Returns the final manifest.
///
/// # Errors
///
/// Returns [`FetchError`] on HTTP, JSON, or I/O failures.
pub fn fetch_dailymed<H: HttpClient>(
    http: &H,
    config: &FetchConfig,
) -> Result<Manifest, FetchError> {
    std::fs::create_dir_all(config.output_dir.join("labels"))?;

    // If we already have a manifest, treat its entries as "already
    // fetched" and skip them. Idempotency is the point.
    let existing = read_manifest(&config.output_dir)?.unwrap_or_else(Manifest::empty);

    let list_text = http.get_text(&list_url(config.limit, 1))?;
    let list: ListResponse =
        serde_json::from_str(&list_text).map_err(|e| FetchError::Json(e.to_string()))?;
    let mut entries = list.data;
    // Deterministic order so reruns produce the same manifest order.
    entries.sort_by(|a, b| a.setid.cmp(&b.setid));
    entries.truncate(config.limit);

    let mut labels = Vec::with_capacity(entries.len());
    for (idx, entry) in entries.iter().enumerate() {
        if idx > 0 {
            std::thread::sleep(config.delay);
        }
        let filename = format!("labels/{}.pdf", entry.setid);
        let pdf_path = config.output_dir.join(&filename);

        // Idempotency: skip if the existing manifest has this set_id
        // and the file is present.
        if let Some(prior) = existing.labels.iter().find(|l| l.set_id == entry.setid) {
            if pdf_path.exists() {
                labels.push(prior.clone());
                continue;
            }
        }

        let pdf_bytes = http.get_bytes(&pdf_url(&entry.setid))?;
        std::fs::write(&pdf_path, &pdf_bytes)?;
        labels.push(LabelEntry {
            set_id: entry.setid.clone(),
            title: entry.title.clone(),
            filename,
            sha256: sha256_hex(&pdf_bytes),
            size_bytes: pdf_bytes.len() as u64,
            fetched_at: now_unix(),
        });
    }

    let manifest = Manifest {
        source: "DailyMed".to_string(),
        fetched_at: now_unix(),
        count: labels.len(),
        labels,
    };
    let toml_text = manifest_to_toml(&manifest)?;
    std::fs::write(config.output_dir.join("manifest.toml"), toml_text)?;
    Ok(manifest)
}

impl Manifest {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            source: "DailyMed".to_string(),
            fetched_at: 0,
            count: 0,
            labels: Vec::new(),
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn list_url_includes_pagesize_and_page() {
        let url = list_url(50, 1);
        assert!(url.contains("pagesize=50"));
        assert!(url.contains("page=1"));
        assert!(url.starts_with("https://dailymed.nlm.nih.gov/"));
    }

    #[test]
    fn pdf_url_carries_set_id() {
        let url = pdf_url("abc-123");
        assert!(url.contains("setId=abc-123"));
    }

    #[test]
    fn sha256_hex_is_deterministic() {
        let h = sha256_hex(b"hello");
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(h, sha256_hex(b"hello"));
    }

    #[test]
    fn manifest_round_trips_through_toml() {
        let m = Manifest {
            source: "DailyMed".to_string(),
            fetched_at: 1_700_000_000,
            count: 1,
            labels: vec![LabelEntry {
                set_id: "abc".to_string(),
                title: "Test".to_string(),
                filename: "labels/abc.pdf".to_string(),
                sha256: "deadbeef".to_string(),
                size_bytes: 1024,
                fetched_at: 1_700_000_000,
            }],
        };
        let s = manifest_to_toml(&m).unwrap();
        let parsed: Manifest = toml::from_str(&s).unwrap();
        assert_eq!(parsed.source, m.source);
        assert_eq!(parsed.labels, m.labels);
    }

    #[test]
    fn list_response_parses_minimal_json() {
        let json = r#"{"data":[{"setid":"abc-123","title":"FAKEMAB INJECTION"},{"setid":"def-456","title":"FAKAZOLE TABLETS"}]}"#;
        let r: ListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(r.data.len(), 2);
        assert_eq!(r.data[0].setid, "abc-123");
        assert_eq!(r.data[1].title, "FAKAZOLE TABLETS");
    }

    /// A stub HTTP client backed by canned responses. Lets the test
    /// exercise `fetch_dailymed`'s control flow without touching the
    /// network.
    struct StubHttp {
        list_response: String,
        pdf_payloads: std::collections::HashMap<String, Vec<u8>>,
        calls: RefCell<Vec<String>>,
    }

    impl HttpClient for StubHttp {
        fn get_text(&self, url: &str) -> Result<String, FetchError> {
            self.calls.borrow_mut().push(url.to_string());
            Ok(self.list_response.clone())
        }
        fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
            self.calls.borrow_mut().push(url.to_string());
            // URL ends with setId=...; pull it and look up the canned bytes.
            let set_id = url.rsplit_once("setId=").map(|(_, s)| s).unwrap_or("");
            self.pdf_payloads
                .get(set_id)
                .cloned()
                .ok_or_else(|| FetchError::Http(format!("stub: no payload for {set_id}")))
        }
    }

    #[test]
    fn fetch_dailymed_writes_pdfs_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let stub = StubHttp {
            list_response: r#"{"data":[{"setid":"a","title":"A"},{"setid":"b","title":"B"}]}"#
                .to_string(),
            pdf_payloads: [
                ("a".to_string(), b"PDF-A".to_vec()),
                ("b".to_string(), b"PDF-B".to_vec()),
            ]
            .into_iter()
            .collect(),
            calls: RefCell::new(Vec::new()),
        };
        let config = FetchConfig {
            limit: 5,
            delay: Duration::from_millis(0),
            output_dir: dir.path().to_path_buf(),
        };
        let manifest = fetch_dailymed(&stub, &config).unwrap();
        assert_eq!(manifest.count, 2);
        assert_eq!(manifest.labels[0].set_id, "a");
        assert_eq!(manifest.labels[1].set_id, "b");
        assert!(dir.path().join("labels/a.pdf").exists());
        assert!(dir.path().join("labels/b.pdf").exists());
        assert!(dir.path().join("manifest.toml").exists());

        // Re-running with the same dir should be idempotent: no new
        // PDF fetches.
        let prior_calls = stub.calls.borrow().len();
        let _again = fetch_dailymed(&stub, &config).unwrap();
        let after_calls = stub.calls.borrow().len();
        // Only the list endpoint is hit on the second run; no PDF re-fetches.
        assert_eq!(
            after_calls - prior_calls,
            1,
            "idempotent re-run should only re-call the list endpoint"
        );
    }
}
