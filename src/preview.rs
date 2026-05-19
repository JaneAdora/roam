use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const SNIFF_BYTES: usize = 4096;
pub const MAX_PREVIEW: usize = 64 * 1024;

pub enum Preview {
    Text(String),
    Binary { size: u64 },
    Unreadable,
}

pub fn read(path: &Path) -> Preview {
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Preview::Unreadable,
    };
    let size = f.metadata().map(|m| m.len()).unwrap_or(0);

    let mut sniff = vec![0u8; SNIFF_BYTES.min(size as usize).max(1)];
    let n = match f.read(&mut sniff) {
        Ok(n) => n,
        Err(_) => return Preview::Unreadable,
    };
    sniff.truncate(n);

    if sniff.contains(&0) {
        return Preview::Binary { size };
    }
    match std::str::from_utf8(&sniff) {
        Ok(_) => {}
        Err(_) => return Preview::Binary { size },
    }

    let mut buf = sniff;
    let to_read = MAX_PREVIEW.saturating_sub(buf.len());
    if to_read > 0 {
        let mut rest = vec![0u8; to_read];
        if let Ok(n) = f.read(&mut rest) {
            rest.truncate(n);
            buf.extend(rest);
        }
    }

    match String::from_utf8(buf) {
        Ok(s) => Preview::Text(s),
        Err(e) => {
            let valid = String::from_utf8_lossy(e.as_bytes()).into_owned();
            Preview::Text(valid)
        }
    }
}
