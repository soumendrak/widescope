//! Self-contained share-link compression.
//!
//! A trace's raw JSON is compressed with raw DEFLATE, seeded with a *preset
//! dictionary* embedded in the WASM binary (`share-dict.bin`) and built from
//! representative OTLP / Jaeger / OpenInference traces. Both the sender and the
//! recipient carry the same dictionary, so it supplies the shared context a
//! single small payload otherwise lacks — without uploading anything. The
//! result is packed into a URL `#fragment` by the caller.
//!
//! DEFLATE (via the pure-Rust `zlib-rs` backend) is used rather than a heavier
//! codec so the compressor adds little to the WASM bundle. Its 32 KiB window
//! also bounds the useful dictionary size — see `share-dict.bin`.
//!
//! ## Blob format
//!
//! The byte string this module produces (before base64url encoding) is:
//!
//! ```text
//! [1-byte format tag][raw DEFLATE stream]
//! ```
//!
//! The tag lets the dictionary be retrained later without breaking old links:
//! a new tag value pins a new dictionary. Links created before this module
//! existed are gzip and start with the gzip magic byte `0x1f`; the caller
//! detects those and decodes them separately, so `0x1f` must never be used as
//! a tag here.

use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};

use crate::errors::WideError;

/// Dictionary embedded at build time. Changing its contents (e.g. retraining
/// on more fixtures) **requires** introducing a new format tag so links made
/// with the old dictionary still decode — see [`TAG_DEFLATE_DICT_V1`].
const SHARE_DICT: &[u8] = include_bytes!("../share-dict.bin");

/// Leading blob byte: raw DEFLATE seeded with dictionary **v1**.
const TAG_DEFLATE_DICT_V1: u8 = 0x01;

/// DEFLATE compression level (0–9). 9 is the maximum ratio; trace blobs are
/// small and DEFLATE is fast, so the extra effort is free.
const LEVEL: u32 = 9;

/// Raw-DEFLATE window size, log2. 15 → the 32 KiB maximum.
const WINDOW_BITS: u8 = 15;

/// Upper bound on decompressed output, guarding against a decompression-bomb
/// link. Sits above the app's 20 MB upload limit with headroom.
const MAX_DECOMPRESSED: usize = 64 * 1024 * 1024;

/// Compress trace JSON into a tagged, self-contained share blob.
///
/// The returned bytes are `[TAG_DEFLATE_DICT_V1] + deflate(json)`; the caller
/// base64url-encodes them for the URL fragment.
pub fn compress_share(json: &str) -> Vec<u8> {
    let input = json.as_bytes();
    let mut compress = Compress::new_with_window_bits(
        Compression::new(LEVEL),
        false, // raw DEFLATE — no zlib header
        WINDOW_BITS,
    );
    // A preset dictionary on a fresh raw-DEFLATE stream cannot fail.
    compress
        .set_dictionary(SHARE_DICT)
        .expect("preset dictionary rejected on a fresh stream");

    let mut out: Vec<u8> = Vec::with_capacity(input.len() / 2 + 64);
    out.push(TAG_DEFLATE_DICT_V1);
    loop {
        if out.len() == out.capacity() {
            out.reserve(out.capacity().max(256));
        }
        let consumed = compress.total_in() as usize;
        let status = compress
            .compress_vec(&input[consumed..], &mut out, FlushCompress::Finish)
            .expect("DEFLATE of an in-memory buffer cannot fail");
        if status == Status::StreamEnd {
            break;
        }
    }
    out
}

/// Reverse [`compress_share`]: decode a tagged share blob back to trace JSON.
///
/// # Errors
///
/// Returns [`WideError::InvalidShareLink`] if the tag is unknown, the DEFLATE
/// stream is corrupt or truncated, the output exceeds [`MAX_DECOMPRESSED`], or
/// the result is not valid UTF-8.
pub fn decompress_share(blob: &[u8]) -> Result<String, WideError> {
    let (&tag, frame) = blob
        .split_first()
        .ok_or_else(|| invalid("empty share blob"))?;
    if tag != TAG_DEFLATE_DICT_V1 {
        return Err(invalid(format!(
            "unsupported share-link format tag 0x{tag:02x}"
        )));
    }

    let mut decompress = Decompress::new_with_window_bits(false, WINDOW_BITS);
    decompress
        .set_dictionary(SHARE_DICT)
        .map_err(|e| invalid(format!("preset dictionary rejected: {e}")))?;

    let mut out: Vec<u8> = Vec::with_capacity(frame.len() * 4 + 256);
    loop {
        if out.len() > MAX_DECOMPRESSED {
            return Err(invalid("decompressed share data exceeds the size limit"));
        }
        if out.len() == out.capacity() {
            out.reserve(out.capacity().max(256));
        }
        let in_before = decompress.total_in();
        let out_before = decompress.total_out();
        let consumed = in_before as usize;
        let status = decompress
            .decompress_vec(&frame[consumed..], &mut out, FlushDecompress::Finish)
            .map_err(|e| invalid(format!("corrupt share-link data: {e}")))?;
        if status == Status::StreamEnd {
            break;
        }
        // No input consumed and no output produced, with output space free,
        // means the stream ended early — truncated or corrupt.
        if decompress.total_in() == in_before && decompress.total_out() == out_before {
            return Err(invalid("truncated or corrupt share-link data"));
        }
    }

    String::from_utf8(out).map_err(|_| invalid("share-link data is not valid UTF-8"))
}

fn invalid(message: impl Into<String>) -> WideError {
    WideError::InvalidShareLink {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &[&str] = &[
        include_str!("../../../test-fixtures/otlp/sample_llm_pipeline.json"),
        include_str!("../../../test-fixtures/jaeger/sample_llm_pipeline.json"),
        include_str!("../../../test-fixtures/openinference/sample_llm_pipeline.json"),
        // Not part of the embedded dictionary — an honest, non-overfit sample.
        include_str!(
            "../../../test-fixtures/otlp/upload-samples/04-mixed-attributes-warnings.json"
        ),
    ];

    #[test]
    fn round_trips_every_fixture() {
        for json in FIXTURES {
            let blob = compress_share(json);
            assert_eq!(blob[0], TAG_DEFLATE_DICT_V1);
            let restored = decompress_share(&blob).expect("decode");
            assert_eq!(&restored, json);
        }
    }

    #[test]
    fn compresses_smaller_than_the_input() {
        for json in FIXTURES {
            let blob = compress_share(json);
            assert!(
                blob.len() < json.len(),
                "blob ({}) should be smaller than input ({})",
                blob.len(),
                json.len()
            );
        }
    }

    #[test]
    fn round_trips_a_large_input() {
        let big = format!("{{\"spans\":[{}\"end\"]}}", "\"x\",".repeat(300_000));
        let restored = decompress_share(&compress_share(&big)).expect("decode");
        assert_eq!(restored, big);
    }

    #[test]
    fn rejects_an_unknown_tag() {
        let err = decompress_share(&[0x1f, 0x8b, 0x00]).unwrap_err();
        assert!(matches!(err, WideError::InvalidShareLink { .. }));
    }

    #[test]
    fn rejects_an_empty_blob() {
        assert!(decompress_share(&[]).is_err());
    }

    #[test]
    fn rejects_a_corrupt_frame() {
        let mut blob = compress_share("{\"trace\":\"x\"}");
        *blob.last_mut().unwrap() ^= 0xff;
        blob.push(0xff);
        assert!(decompress_share(&blob).is_err());
    }
}

/// The failure modes a share link hits in the wild: truncation, tampering, and
/// payloads big enough to make the codec grow its buffer.
#[cfg(test)]
mod codec_edge_tests {
    use super::*;

    #[test]
    fn a_payload_larger_than_the_initial_buffer_still_round_trips() {
        // Forces both the compress and decompress loops to reserve more room.
        let json = format!(
            r#"{{"spans":[{}]}}"#,
            "\"x\",".repeat(200_000).trim_end_matches(',')
        );
        let blob = compress_share(&json);
        assert_eq!(decompress_share(&blob).unwrap(), json);
    }

    #[test]
    fn a_truncated_frame_is_rejected_rather_than_returning_a_partial_trace() {
        let blob = compress_share(r#"{"resourceSpans":[]}"#);
        let truncated = &blob[..blob.len() / 2];
        let err = decompress_share(truncated).unwrap_err();
        assert!(matches!(err, WideError::InvalidShareLink { .. }));
    }

    #[test]
    fn a_tampered_frame_is_rejected() {
        let mut blob = compress_share(r#"{"resourceSpans":[]}"#);
        let last = blob.len() - 1;
        blob[last] ^= 0xff;
        blob[last / 2] ^= 0xff;
        assert!(decompress_share(&blob).is_err());
    }

    #[test]
    fn non_utf8_output_is_reported_as_such() {
        // Compress raw bytes that are valid DEFLATE input but not valid UTF-8,
        // by hand-building a frame the same way compress_share does.
        use flate2::{Compress, Compression, FlushCompress, Status};
        let payload: Vec<u8> = vec![0xff, 0xfe, 0xfd];
        let mut compress = Compress::new_with_window_bits(Compression::best(), false, WINDOW_BITS);
        compress.set_dictionary(SHARE_DICT).unwrap();
        let mut frame = vec![TAG_DEFLATE_DICT_V1];
        loop {
            if frame.len() == frame.capacity() {
                frame.reserve(64);
            }
            let consumed = compress.total_in() as usize;
            let status = compress
                .compress_vec(&payload[consumed..], &mut frame, FlushCompress::Finish)
                .unwrap();
            if status == Status::StreamEnd {
                break;
            }
        }
        let err = decompress_share(&frame).unwrap_err();
        match err {
            WideError::InvalidShareLink { message } => {
                assert!(message.contains("UTF-8"), "{message}");
            }
            other => panic!("expected InvalidShareLink, got {other:?}"),
        }
    }
}
