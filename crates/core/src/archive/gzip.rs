//! gzip framing around `miniz_oxide`'s raw DEFLATE. `compress_to_vec` emits a
//! bare deflate stream, so this adds the 10-byte gzip header and the CRC-32 +
//! ISIZE trailer needed for `.gz` / `.tar.gz` that any tool will accept.

/// gzip-compress `data` (deflate level 6). Self-contained — no zlib, no C.
pub fn gzip(data: &[u8]) -> Vec<u8> {
    let deflated = miniz_oxide::deflate::compress_to_vec(data, 6);
    let mut out = Vec::with_capacity(deflated.len() + 18);
    // Header: magic, CM=8 (deflate), no flags, mtime=0, XFL=0, OS=255 (unknown).
    out.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00, 0, 0, 0, 0, 0x00, 0xff]);
    out.extend_from_slice(&deflated);
    // Trailer: CRC-32 of the input, then the input length, both little-endian.
    out.extend_from_slice(&crc32(data).to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

/// CRC-32 (ISO-HDLC, polynomial 0xEDB88320) — the variant gzip mandates.
/// Table-free: a few KB of input is trivial and this keeps the wasm small.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
