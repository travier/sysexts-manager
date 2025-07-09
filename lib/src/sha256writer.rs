use sha2::{Digest, Sha256};
use std::io::{Result as IoResult, Write};

// From https://users.rust-lang.org/t/read-and-hash-sha1-at-the-same-time/54458
pub struct Sha256Writer<W> {
    writer: W,
    hasher: Sha256,
}

impl<W> Sha256Writer<W> {
    pub fn new(writer: W) -> Self {
        Sha256Writer {
            writer,
            hasher: Sha256::new(),
        }
    }

    pub fn digest(self) -> String {
        hex::encode(self.hasher.finalize())
    }
}

impl<W: Write> Write for Sha256Writer<W> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.hasher.update(buf);
        self.writer.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.writer.flush()
    }
}
