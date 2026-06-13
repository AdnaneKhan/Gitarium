//! Minimal in-memory ustar (POSIX tar) writer. Appends regular-file entries
//! into a growing `Vec<u8>`; `finish` seals it with the two zero blocks that
//! mark end-of-archive. Numeric fields and the header checksum follow the
//! ustar layout, and over-long paths split across the name/prefix fields.

/// Accumulates ustar entries into an in-memory archive.
#[derive(Default)]
pub struct Tar {
    out: Vec<u8>,
}

impl Tar {
    pub fn new() -> Self {
        Tar { out: Vec::new() }
    }

    /// Append a regular file with the default `0644` mode.
    pub fn file(&mut self, path: &str, data: &[u8]) {
        self.file_mode(path, data, 0o644);
    }

    /// Append a regular file with an explicit unix mode (e.g. `0o755`).
    pub fn file_mode(&mut self, path: &str, data: &[u8], mode: u32) {
        self.header(path, data.len(), mode);
        self.out.extend_from_slice(data);
        // Pad the body out to the next 512-byte block boundary.
        let rem = data.len() % 512;
        if rem != 0 {
            self.out.resize(self.out.len() + (512 - rem), 0);
        }
    }

    /// Seal the archive with the two trailing zero blocks and return its bytes.
    pub fn finish(mut self) -> Vec<u8> {
        self.out.resize(self.out.len() + 1024, 0);
        self.out
    }

    /// Emit one 512-byte ustar header for a regular file at `path`.
    fn header(&mut self, path: &str, size: usize, mode: u32) {
        let start = self.out.len();
        self.out.resize(start + 512, 0);
        let h = &mut self.out[start..start + 512];
        let (name, prefix) = split_name(path);
        h[..name.len()].copy_from_slice(name.as_bytes());
        octal_field(&mut h[100..108], mode as u64); // mode
        octal_field(&mut h[108..116], 0); // uid
        octal_field(&mut h[116..124], 0); // gid
        octal_field(&mut h[124..136], size as u64); // size
        octal_field(&mut h[136..148], 0); // mtime (epoch → deterministic output)
        h[156] = b'0'; // typeflag: regular file
        h[257..263].copy_from_slice(b"ustar\0");
        h[263..265].copy_from_slice(b"00");
        if !prefix.is_empty() {
            h[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        }
        // Checksum: the unsigned sum of every header byte, with the 8 checksum
        // bytes themselves counted as spaces. Stored as 6 octal digits, a NUL,
        // then a space — the canonical ustar encoding.
        for b in h[148..156].iter_mut() {
            *b = b' ';
        }
        let sum: u32 = h.iter().map(|&b| b as u32).sum();
        h[148..154].copy_from_slice(format!("{:06o}", sum).as_bytes());
        h[154] = 0;
        h[155] = b' ';
    }
}

/// Write `value` as zero-padded octal into all but the last byte of `field`
/// (which is left NUL — the ustar convention for numeric fields). On overflow,
/// the low-order digits are kept.
fn octal_field(field: &mut [u8], value: u64) {
    let w = field.len() - 1;
    let s = format!("{:0width$o}", value, width = w);
    let b = s.as_bytes();
    let take = b.len().saturating_sub(w);
    field[..w].copy_from_slice(&b[take..]);
    field[w] = 0;
}

/// Split `path` into `(name ≤ 100, prefix ≤ 155)` per ustar. Short paths use
/// the name field alone; longer ones split on the `/` that fills the prefix
/// the most while keeping the name within 100 bytes. A single component longer
/// than 100 bytes (vanishingly rare for repo paths) keeps its trailing bytes.
fn split_name(path: &str) -> (&str, &str) {
    if path.len() <= 100 {
        return (path, "");
    }
    let mut split = None;
    for (i, &b) in path.as_bytes().iter().enumerate() {
        if b == b'/' && i <= 155 && path.len() - i - 1 <= 100 {
            split = Some(i);
        }
    }
    match split {
        Some(i) => (&path[i + 1..], &path[..i]),
        None => {
            let cut = path.len() - 100;
            let start = path.char_indices().map(|(i, _)| i).find(|&i| i >= cut).unwrap_or(0);
            (&path[start..], "")
        }
    }
}
