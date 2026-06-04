//! `sct serve` - a FHIR R4 terminology server over the SQLite artefact.
//!
//! Phase 1: `/metadata` (CapabilityStatement), `CodeSystem/$lookup`,
//! `$validate-code`, `$subsumes`, and `ValueSet/$expand` (text filter + full
//! ECL via [`crate::ecl`]). See `specs/commands/serve.md`. The operation logic
//! lives in [`ops`] as pure functions; the handlers here are thin transport.

pub mod fhir;
pub mod ops;

use anyhow::{Context, Result};
use axum::{
    extract::{RawQuery, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use clap::Parser;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;

use fhir::FhirError;

#[derive(Parser, Debug)]
pub struct Args {
    /// SNOMED CT SQLite database produced by `sct sqlite`. Discovered via the
    /// usual path-resolution chain when omitted (see `docs/path-resolution.md`).
    #[arg(long)]
    pub db: Option<PathBuf>,

    /// TCP port to listen on.
    #[arg(long, default_value_t = 8080)]
    pub port: u16,

    /// Host/address to bind.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,

    /// FHIR base path. Set to `/fhir` for Ontoserver-compatible URLs.
    #[arg(long, default_value = "/")]
    pub fhir_base: String,

    /// Refuse write operations (always true; the server is read-only).
    #[arg(long, default_value_t = true)]
    pub read_only: bool,
}

#[derive(Clone)]
struct AppState {
    db: Arc<PathBuf>,
    impl_url: Arc<String>,
}

pub fn run(args: Args) -> Result<()> {
    let db = crate::paths::resolve_db(args.db.as_deref())?.path;
    // Open once up front so a bad/missing DB fails before we bind the port, and
    // nudge the user about the transitive-closure table while we're here.
    {
        let conn = crate::commands::open_db_readonly(&db, None)
            .with_context(|| format!("opening database {}", db.display()))?;
        crate::ecl::warn_if_no_tct(&conn);
    }

    let addr = format!("{}:{}", args.host, args.port);
    let listener = std::net::TcpListener::bind(&addr).with_context(|| format!("binding {addr}"))?;
    let base = normalise_base(&args.fhir_base);
    eprintln!(
        "sct serve: FHIR R4 terminology server on http://{addr}{base}\n  database: {}\n  try: curl 'http://{addr}{base}/metadata'",
        db.display()
    );
    serve_listener(db, &args.fhir_base, listener)
}

/// Serve the FHIR router on an already-bound std listener, blocking. Shared by
/// `run` and by integration tests (which bind an ephemeral port first).
#[doc(hidden)]
pub fn serve_listener(db: PathBuf, fhir_base: &str, listener: std::net::TcpListener) -> Result<()> {
    let base = normalise_base(fhir_base);
    let addr = listener.local_addr().context("listener address")?;
    let impl_url = format!("http://{addr}{base}");
    let state = AppState {
        db: Arc::new(db),
        impl_url: Arc::new(impl_url),
    };

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;

    rt.block_on(async move {
        listener.set_nonblocking(true).context("set_nonblocking")?;
        let listener = tokio::net::TcpListener::from_std(listener).context("from_std")?;
        let app = build_router(state, &base);
        axum::serve(listener, app).await.context("serving")?;
        Ok::<_, anyhow::Error>(())
    })
}

fn normalise_base(base: &str) -> String {
    let b = base.trim_end_matches('/');
    if b.is_empty() {
        String::new()
    } else if b.starts_with('/') {
        b.to_string()
    } else {
        format!("/{b}")
    }
}

fn build_router(state: AppState, base: &str) -> Router {
    let app = Router::new()
        .route("/metadata", get(metadata))
        .route("/CodeSystem/$lookup", get(lookup).post(lookup))
        .route(
            "/CodeSystem/$validate-code",
            get(validate_code).post(validate_code),
        )
        .route("/CodeSystem/$subsumes", get(subsumes).post(subsumes))
        .route("/ValueSet/$expand", get(expand).post(expand))
        .with_state(state);
    if base.is_empty() {
        app
    } else {
        Router::new().nest(base, app)
    }
}

// --- handlers ---------------------------------------------------------------

async fn metadata(State(st): State<AppState>, headers: HeaderMap) -> Response {
    if let Some(r) = reject_xml(&headers) {
        return r;
    }
    fhir_ok(fhir::capability_statement(
        env!("CARGO_PKG_VERSION"),
        &st.impl_url,
    ))
}

async fn lookup(State(st): State<AppState>, headers: HeaderMap, RawQuery(q): RawQuery) -> Response {
    if let Some(r) = reject_xml(&headers) {
        return r;
    }
    let params = parse_query(q.as_deref().unwrap_or(""));
    let Some(code) = param(&params, "code").map(str::to_string) else {
        return fhir_err(FhirError::invalid("missing required parameter 'code'"));
    };
    let props = params_all(&params, "property");
    run_db(&st, move |c| ops::lookup(c, &code, &props)).await
}

async fn validate_code(
    State(st): State<AppState>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Response {
    if let Some(r) = reject_xml(&headers) {
        return r;
    }
    let params = parse_query(q.as_deref().unwrap_or(""));
    let Some(code) = param(&params, "code").map(str::to_string) else {
        return fhir_err(FhirError::invalid("missing required parameter 'code'"));
    };
    let display = param(&params, "display").map(str::to_string);
    run_db(&st, move |c| {
        ops::validate_code(c, &code, display.as_deref())
    })
    .await
}

async fn subsumes(
    State(st): State<AppState>,
    headers: HeaderMap,
    RawQuery(q): RawQuery,
) -> Response {
    if let Some(r) = reject_xml(&headers) {
        return r;
    }
    let params = parse_query(q.as_deref().unwrap_or(""));
    let (Some(a), Some(b)) = (
        param(&params, "codeA").map(str::to_string),
        param(&params, "codeB").map(str::to_string),
    ) else {
        return fhir_err(FhirError::invalid(
            "missing required parameters 'codeA' and 'codeB'",
        ));
    };
    run_db(&st, move |c| ops::subsumes(c, &a, &b)).await
}

async fn expand(State(st): State<AppState>, headers: HeaderMap, RawQuery(q): RawQuery) -> Response {
    if let Some(r) = reject_xml(&headers) {
        return r;
    }
    let params = parse_query(q.as_deref().unwrap_or(""));
    let ecl = param(&params, "url").and_then(parse_implicit_ecl);
    let filter = param(&params, "filter").map(str::to_string);
    let count = param(&params, "count")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100usize);
    let offset = param(&params, "offset")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0usize);
    let include_designations = param(&params, "includeDesignations")
        .map(|s| s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    run_db(&st, move |c| {
        ops::expand(
            c,
            ecl.as_deref(),
            filter.as_deref(),
            count,
            offset,
            include_designations,
        )
    })
    .await
}

// --- helpers ----------------------------------------------------------------

/// Run a DB operation on a blocking thread with a fresh read-only connection,
/// turning the `Result<Value, FhirError>` into an HTTP response.
async fn run_db<F>(st: &AppState, f: F) -> Response
where
    F: FnOnce(&Connection) -> Result<serde_json::Value, FhirError> + Send + 'static,
{
    let db = st.db.clone();
    let joined = tokio::task::spawn_blocking(move || {
        let conn = crate::commands::open_db_readonly(db.as_path(), None)
            .map_err(|e| FhirError::exception(format!("opening database: {e}")))?;
        f(&conn)
    })
    .await;
    match joined {
        Ok(Ok(value)) => fhir_ok(value),
        Ok(Err(e)) => fhir_err(e),
        Err(e) => fhir_err(FhirError::exception(format!("internal task error: {e}"))),
    }
}

fn fhir_ok(body: serde_json::Value) -> Response {
    fhir_response(StatusCode::OK, &body)
}

fn fhir_err(e: FhirError) -> Response {
    let status = StatusCode::from_u16(e.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    fhir_response(status, &e.outcome())
}

fn fhir_response(status: StatusCode, body: &serde_json::Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/fhir+json")],
        serde_json::to_string(body).unwrap_or_else(|_| "{}".into()),
    )
        .into_response()
}

/// 406 if the client asks exclusively for XML (not supported).
fn reject_xml(headers: &HeaderMap) -> Option<Response> {
    let accept = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok())?;
    let a = accept.to_lowercase();
    if a.contains("xml") && !a.contains("json") && !a.contains("*/*") {
        Some(fhir_err(FhirError {
            status: 406,
            code: "not-supported",
            diagnostics: "XML is not supported; request application/fhir+json".into(),
        }))
    } else {
        None
    }
}

/// Extract an ECL expression from a FHIR implicit SNOMED ValueSet `url`, e.g.
/// `http://snomed.info/sct?fhir_vs=ecl/<<73211009`. Returns `None` for the
/// "all concepts" form (`?fhir_vs` with no value) or a non-ECL url.
fn parse_implicit_ecl(url: &str) -> Option<String> {
    let after = url.split("fhir_vs=").nth(1)?;
    let after = after.split('&').next().unwrap_or(after);
    let ecl = after.strip_prefix("ecl/")?;
    if ecl.is_empty() {
        None
    } else {
        Some(ecl.to_string())
    }
}

/// Parse a raw query string into key/value pairs, percent-decoding both sides
/// (and `+` → space). Handles repeated keys (FHIR uses `property=` repeatedly).
fn parse_query(raw: &str) -> Vec<(String, String)> {
    raw.split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let mut it = pair.splitn(2, '=');
            let k = pct_decode(it.next().unwrap_or(""));
            let v = pct_decode(it.next().unwrap_or(""));
            (k, v)
        })
        .collect()
}

fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn params_all(params: &[(String, String)], key: &str) -> Vec<String> {
    params
        .iter()
        .filter(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .collect()
}

fn pct_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => match u8::from_str_radix(&s[i + 1..i + 3], 16) {
                Ok(byte) => {
                    out.push(byte);
                    i += 3;
                }
                Err(_) => {
                    out.push(b'%');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeated_and_encoded_params() {
        let params =
            parse_query("code=22298006&property=parent&property=child&filter=heart+attack");
        assert_eq!(param(&params, "code"), Some("22298006"));
        assert_eq!(params_all(&params, "property"), vec!["parent", "child"]);
        assert_eq!(param(&params, "filter"), Some("heart attack"));
    }

    #[test]
    fn extracts_ecl_from_implicit_url() {
        assert_eq!(
            parse_implicit_ecl("http://snomed.info/sct?fhir_vs=ecl/<<73211009"),
            Some("<<73211009".to_string())
        );
        assert_eq!(parse_implicit_ecl("http://snomed.info/sct?fhir_vs"), None);
        assert_eq!(
            parse_implicit_ecl("http://snomed.info/sct?fhir_vs=ecl/"),
            None
        );
    }

    #[test]
    fn normalises_fhir_base() {
        assert_eq!(normalise_base("/"), "");
        assert_eq!(normalise_base("/fhir"), "/fhir");
        assert_eq!(normalise_base("fhir/"), "/fhir");
    }
}
