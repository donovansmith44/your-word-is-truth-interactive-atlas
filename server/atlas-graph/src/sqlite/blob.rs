//! The committed section blob: zstd-compressed, named by its LOGICAL hash and verified on unpack
//! by its TRANSPORT hash, which varies with the zstd version and thread count and so guards the
//! copy, never the content -- only the logical hash is an identity.

use std::io::Write;
use std::path::Path;

use atlas_graph_types::sha256::sha256;

use super::SqliteError;

/// 100 MiB, the ceiling a committed blob must stay under; the writer refuses a larger one.
pub const BLOB_CEILING: u64 = 104_857_600;
pub const ZSTD_LEVEL: i32 = 19;

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// SHA-256 of the file's bytes, as 64 lowercase hex.
pub fn sha256_hex_of_file(path: &Path) -> Result<String, SqliteError> {
    Ok(hex(&sha256(&std::fs::read(path)?)))
}

fn threads() -> u32 {
    std::thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1).min(4)
}

fn zerr(e: std::io::Error) -> SqliteError {
    SqliteError(format!("zstd: {e}"))
}

/// Compresses `src` to `dst` through `dst.zst.tmp`, renamed on success, so a partial file is
/// never mistaken for a blob. Returns the SHA-256 hex of `dst` and its byte size.
pub fn compress_file(src: &Path, dst: &Path) -> Result<(String, u64), SqliteError> {
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dst.with_extension("zst.tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        let mut input = std::io::BufReader::new(std::fs::File::open(src)?);
        let out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        let mut enc = zstd::stream::Encoder::new(out, ZSTD_LEVEL).map_err(zerr)?;
        enc.multithread(threads()).map_err(zerr)?;
        std::io::copy(&mut input, &mut enc)?;
        let mut out = enc.finish().map_err(zerr)?;
        out.flush()?;
    }
    let _ = std::fs::remove_file(dst);
    std::fs::rename(&tmp, dst)?;
    let bytes = std::fs::metadata(dst)?.len();
    Ok((sha256_hex_of_file(dst)?, bytes))
}

/// Verifies the blob's transport hash BEFORE unpacking: a mismatch names both hashes and writes
/// nothing, and a failed unpack removes its partial file. Returns the unpacked size.
pub fn decompress_verified(blob: &Path, expected_sha256: &str, dst: &Path) -> Result<u64, SqliteError> {
    let compressed = std::fs::read(blob)?;
    let actual = hex(&sha256(&compressed));
    if actual != expected_sha256 {
        return Err(SqliteError(format!(
            "transport hash mismatch for {}: manifest {expected_sha256}, file {actual}",
            blob.display()
        )));
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = dst.with_extension("sqlite.tmp");
    let _ = std::fs::remove_file(&tmp);
    let unpack = || -> Result<(), SqliteError> {
        let mut dec = zstd::stream::Decoder::new(&compressed[..]).map_err(zerr)?;
        let mut out = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        std::io::copy(&mut dec, &mut out)?;
        out.flush()?;
        Ok(())
    };
    if let Err(e) = unpack() {
        let _ = std::fs::remove_file(&tmp);
        return Err(SqliteError(format!("unpacking {}: {e}", blob.display())));
    }
    let _ = std::fs::remove_file(dst);
    std::fs::rename(&tmp, dst)?;
    Ok(std::fs::metadata(dst)?.len())
}
