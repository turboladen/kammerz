//! Embedded-asset responses with ETag/304 revalidation (kammerz-jax).
//!
//! `main.rs` embeds the SvelteKit build via `rust-embed` and serves it through
//! `serve_spa`. Content-hashed bundle files under `_app/immutable/*` ship with a
//! one-year `immutable` `Cache-Control`, so the browser never revalidates them.
//! Everything else — `index.html`, the favicon, and the ~72KB of self-hosted
//! `woff2` fonts copied verbatim from `static/` — is *not* content-hashed, so it
//! shipped with bare `no-cache` and **no validator**. Without a validator the
//! browser can't make a conditional request, so every cold load re-downloaded
//! those bytes in full. woff2 is already compressed, so the gzip/brotli layer
//! doesn't help here — a 304 is the only win.
//!
//! [`asset_response`] attaches a strong ETag derived from the file's sha256 (which
//! `rust-embed` already computes per embedded file) to *every* asset, and answers
//! a matching `If-None-Match` with `304 Not Modified` (empty body, validator +
//! `Cache-Control` retained per RFC 9110 §15.4.5). The ETag is over the identity
//! bytes; that's correct alongside the compression layer, which varies the wire
//! encoding by `Accept-Encoding` (and sends `Vary`) — and a 304 carries no body
//! for that layer to encode anyway.
//!
//! The logic lives here, not in `main.rs`, so the binary's `serve_spa` and the
//! integration tests share one definition (mirroring `kammerz::compression`).
//! Keeping it a pure function of `(headers, path, bytes, sha256, mime)` also makes
//! it testable without depending on the embed's contents, which are wiped from a
//! fresh tree.

use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, Response, StatusCode};

/// `Cache-Control` for the content-hashed bundle under `_app/immutable/*`: the
/// filename changes whenever the bytes do, so the browser may cache for a year
/// and never revalidate.
const IMMUTABLE_CACHE: &str = "public, max-age=31536000, immutable";
/// `Cache-Control` for assets whose URL is stable across content changes
/// (`index.html`, favicon, fonts). `no-cache` does *not* mean "don't store" — it
/// means "store, but revalidate before reuse", which is exactly what the ETag
/// below enables.
const REVALIDATE_CACHE: &str = "no-cache";

/// Render the strong ETag for an asset: the lowercase hex of its sha256, wrapped
/// in the double quotes RFC 9110 §8.8.3 requires for an entity-tag.
fn etag_for(sha256: &[u8; 32]) -> String {
    let mut tag = String::with_capacity(2 + 64);
    tag.push('"');
    for byte in sha256 {
        // {:02x} — fixed two lowercase hex digits per byte.
        use std::fmt::Write;
        let _ = write!(tag, "{byte:02x}");
    }
    tag.push('"');
    tag
}

/// Does an `If-None-Match` header value match `etag`?
///
/// `etag` is our strong, quoted tag (e.g. `"abc123"`). Per RFC 9110 §13.1.2,
/// `If-None-Match` is either `*` (matches any current representation) or a
/// comma-separated list of entity-tags, each optionally weak-prefixed with `W/`.
/// We serve only strong tags, but a cache may echo back a weak form, so we strip
/// a leading `W/` before comparing and use the **weak comparison** the spec
/// mandates for `If-None-Match` (compare opaque tag values, ignore weakness).
///
/// A plain substring search would be wrong (a short tag could match inside a
/// longer one), so we split on commas and compare each entry exactly. This is
/// sufficient for a single-binary server that emits exactly one strong tag per
/// asset.
fn if_none_match_matches(header_value: &str, etag: &str) -> bool {
    let trimmed = header_value.trim();
    if trimmed == "*" {
        return true;
    }
    trimmed.split(',').any(|candidate| {
        let candidate = candidate.trim();
        // Weak comparison: drop an optional `W/` weakness flag from the client's
        // tag before comparing. Our own `etag` is always strong (see `etag_for`),
        // so only the candidate side can carry the prefix.
        let candidate = candidate.strip_prefix("W/").unwrap_or(candidate);
        candidate == etag
    })
}

/// Build the HTTP response for an embedded asset, with ETag/304 revalidation.
///
/// `path` is the requested asset path (used only to pick the `Cache-Control`
/// policy: `immutable` for the content-hashed `_app/immutable/*` bundle, otherwise
/// `no-cache`/revalidate). `bytes` and `sha256` are the file's content and the
/// hash `rust-embed` precomputed for it; `mime` is the resolved content type.
///
/// When the request's `If-None-Match` matches the asset's ETag, returns `304 Not
/// Modified` with an empty body, retaining the `ETag` and `Cache-Control` headers
/// (RFC 9110 §15.4.5) so the cache can refresh its freshness metadata. Otherwise
/// returns `200 OK` with the full body and `Content-Type`, `Cache-Control`, and
/// `ETag` headers.
pub fn asset_response(
    request_headers: &HeaderMap,
    path: &str,
    bytes: std::borrow::Cow<'static, [u8]>,
    sha256: &[u8; 32],
    mime: &str,
) -> Response<Body> {
    let etag = etag_for(sha256);
    let cache = if path.starts_with("_app/immutable/") {
        IMMUTABLE_CACHE
    } else {
        REVALIDATE_CACHE
    };

    let not_modified = request_headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|inm| if_none_match_matches(inm, &etag));

    // ETag and Cache-Control ride on both the 200 and the 304 (the latter per
    // RFC 9110 §15.4.5 so the client can update freshness without a body).
    let mut builder = Response::builder()
        .header(header::CACHE_CONTROL, cache)
        .header(header::ETAG, &etag);

    if not_modified {
        return builder
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .expect("static 304 response is always valid");
    }

    // mime is a runtime string (mime_guess output); fall back to octet-stream
    // rather than panicking if it somehow isn't a valid header value.
    let content_type = HeaderValue::from_str(mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    builder = builder.header(header::CONTENT_TYPE, content_type);

    builder
        .status(StatusCode::OK)
        .body(Body::from(bytes.into_owned()))
        .expect("static 200 response is always valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etag_is_quoted_lowercase_hex_of_sha256() {
        let mut hash = [0u8; 32];
        hash[0] = 0xab;
        hash[31] = 0x0f;
        let tag = etag_for(&hash);
        assert!(tag.starts_with('"') && tag.ends_with('"'));
        // 32 bytes -> 64 hex chars, plus the two quotes.
        assert_eq!(tag.len(), 66);
        assert!(tag.starts_with("\"ab"));
        assert!(tag.ends_with("0f\""));
        // No uppercase hex.
        assert_eq!(tag, tag.to_lowercase());
    }

    #[test]
    fn if_none_match_exact_and_star_and_weak_and_list() {
        let etag = "\"deadbeef\"";
        assert!(if_none_match_matches("\"deadbeef\"", etag));
        assert!(if_none_match_matches("*", etag));
        assert!(if_none_match_matches("W/\"deadbeef\"", etag));
        // Multiple comma-separated tags, ours in the middle.
        assert!(if_none_match_matches(
            "\"aaa\", W/\"deadbeef\", \"bbb\"",
            etag
        ));
        // A different tag must not match.
        assert!(!if_none_match_matches("\"cafe\"", etag));
        // A short tag must not match a longer one by substring.
        assert!(!if_none_match_matches("\"dead\"", etag));
    }
}
