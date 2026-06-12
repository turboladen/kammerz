//! Response compression layer (kammerz-5n3).
//!
//! Single source of truth for the `tower-http` `CompressionLayer` the server
//! mounts, so `main.rs` and the integration tests exercise the *same* predicate
//! rather than two hand-copied expressions that could silently drift.

use tower_http::compression::predicate::{And, NotForContentType, Predicate};
use tower_http::compression::{CompressionLayer, DefaultPredicate};

/// The compression predicate: tower-http's `DefaultPredicate` (skips images,
/// already-compressed bodies, and anything under its `SizeAbove(32)` floor) with
/// `font/woff2` additionally excluded.
///
/// woff2 is itself a compressed container — brotli over our DM Sans face yields
/// 36984 bytes from 36980 (gzip 36912), i.e. zero-to-negative gain for the CPU —
/// and the default predicate does *not* cover it, so we exclude it explicitly.
pub type CompressionPredicate = And<DefaultPredicate, NotForContentType>;

/// Build the gzip/brotli `CompressionLayer` the server wraps its router in.
///
/// gzip/brotli are negotiated from `Accept-Encoding`; a client that omits the
/// header (bare `curl`, the deploy health grep) gets an untouched identity
/// response. See [`CompressionPredicate`] for what stays uncompressed.
pub fn compression_layer() -> CompressionLayer<CompressionPredicate> {
    CompressionLayer::new()
        .compress_when(DefaultPredicate::new().and(NotForContentType::const_new("font/woff2")))
}
