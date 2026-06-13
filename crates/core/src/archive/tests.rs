use super::{gzip, targz, Tar};

/// gzip's CRC-32 and ISIZE trailer match the canonical CRC-32 check value.
#[test]
fn gzip_trailer_check_value() {
    let g = gzip(b"123456789");
    assert_eq!(&g[..3], &[0x1f, 0x8b, 0x08]); // gzip magic + deflate
    let crc = u32::from_le_bytes(g[g.len() - 8..g.len() - 4].try_into().unwrap());
    assert_eq!(crc, 0xCBF4_3926); // the standard CRC-32 test vector
    let isize = u32::from_le_bytes(g[g.len() - 4..].try_into().unwrap());
    assert_eq!(isize, 9);
}

/// gzip output inflates back to the input (header/trailer stripped).
#[test]
fn gzip_roundtrip() {
    let data = b"the quick brown fox jumps over the lazy dog\n".repeat(50);
    let g = gzip(&data);
    let raw = miniz_oxide::inflate::decompress_to_vec(&g[10..g.len() - 8]).unwrap();
    assert_eq!(raw, data);
}

/// A one-file tar carries the ustar magic, an octal size, the regular-file
/// typeflag, a block-padded body, and a self-consistent header checksum.
#[test]
fn tar_one_file() {
    let mut t = Tar::new();
    t.file("dir/hello.txt", b"hi");
    let out = t.finish();

    assert_eq!(out.len(), 512 + 512 + 1024); // header + padded body + 2 end blocks
    assert_eq!(&out[0..13], b"dir/hello.txt");
    assert_eq!(&out[124..136], b"00000000002\0"); // size = 2, octal
    assert_eq!(out[156], b'0'); // regular file
    assert_eq!(&out[257..263], b"ustar\0");
    assert_eq!(&out[512..514], b"hi");

    // Recompute the checksum with the field blanked and compare.
    let stored = u32::from_str_radix(std::str::from_utf8(&out[148..154]).unwrap(), 8).unwrap();
    let mut hdr = out[..512].to_vec();
    for b in &mut hdr[148..156] {
        *b = b' ';
    }
    let sum: u32 = hdr.iter().map(|&b| b as u32).sum();
    assert_eq!(stored, sum);
}

/// Over-long paths split across the name and prefix fields and reconstruct.
#[test]
fn tar_long_path_prefix() {
    let deep = format!("{}file.rs", "abc/".repeat(40)); // 167 bytes, > 100
    let mut t = Tar::new();
    t.file(&deep, b"x");
    let out = t.finish();

    let name_end = out[0..100].iter().position(|&b| b == 0).unwrap_or(100);
    let name = std::str::from_utf8(&out[0..name_end]).unwrap();
    let pre_end = out[345..500].iter().position(|&b| b == 0).map_or(500, |i| 345 + i);
    let prefix = std::str::from_utf8(&out[345..pre_end]).unwrap();
    assert!(!prefix.is_empty(), "prefix field should be used for long paths");
    assert_eq!(format!("{}/{}", prefix, name), deep);
}

/// `targz` lays entries down in order and inflates to a tar containing them.
#[test]
fn targz_builds_archive() {
    let g = targz([("a.txt", &b"alpha"[..]), ("b.txt", &b"beta"[..])]);
    assert_eq!(&g[..3], &[0x1f, 0x8b, 0x08]);
    let tar = miniz_oxide::inflate::decompress_to_vec(&g[10..g.len() - 8]).unwrap();
    assert_eq!(tar.len(), (512 + 512) * 2 + 1024);
    assert_eq!(&tar[0..5], b"a.txt"); // first header
    assert_eq!(&tar[1024..1029], b"b.txt"); // second header
}
