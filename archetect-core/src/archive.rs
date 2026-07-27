//! Archive construction, in memory.
//!
//! Archives are built from what the render *wrote*, not from what happens to be
//! on disk. That is deliberate: `WriteFile` is a script→client message, so in
//! server mode the rendered tree never touches the server's filesystem and a
//! disk-reading archiver would package an empty directory. Building from the
//! write journal gives one behaviour in both modes, and lets the finished
//! archive travel back out as an ordinary `WriteFile` — so a client needs no
//! archiving capability of its own.

use std::collections::BTreeSet;
use std::io::{self, Write};

/// One file destined for an archive, addressed relative to the archive root.
pub struct ArchiveEntry {
    pub path: String,
    pub contents: Vec<u8>,
}

/// Build a ZIP archive whose members are nested under `root`.
pub fn build_zip(root: &str, entries: &[ArchiveEntry]) -> io::Result<Vec<u8>> {
    use std::io::Cursor;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    zip.add_directory(format!("{}/", root), options)?;

    // We only hold file contents, but readers expect explicit directory
    // members, so synthesize one for every ancestor path.
    let mut seen = BTreeSet::new();
    for entry in entries {
        let segments: Vec<&str> = entry.path.split('/').collect();
        let mut ancestor = String::new();
        for segment in &segments[..segments.len().saturating_sub(1)] {
            ancestor.push_str(segment);
            ancestor.push('/');
            if seen.insert(ancestor.clone()) {
                zip.add_directory(format!("{}/{}", root, ancestor), options)?;
            }
        }
    }

    for entry in entries {
        zip.start_file(format!("{}/{}", root, entry.path), options)?;
        zip.write_all(&entry.contents)?;
    }

    Ok(zip.finish()?.into_inner())
}

/// Build a tar archive (optionally gzipped) whose members are nested under `root`.
pub fn build_tar(root: &str, entries: &[ArchiveEntry], compress: bool) -> io::Result<Vec<u8>> {
    let tarball = write_tar(root, entries)?;
    if !compress {
        return Ok(tarball);
    }
    let mut encoder =
        flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&tarball)?;
    encoder.finish()
}

fn write_tar(root: &str, entries: &[ArchiveEntry]) -> io::Result<Vec<u8>> {
    use tar::{Builder, Header};

    let mut builder = Builder::new(Vec::new());
    for entry in entries {
        let mut header = Header::new_gnu();
        header.set_size(entry.contents.len() as u64);
        header.set_mode(0o644);
        builder.append_data(
            &mut header,
            format!("{}/{}", root, entry.path),
            entry.contents.as_slice(),
        )?;
    }
    builder.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries() -> Vec<ArchiveEntry> {
        vec![
            ArchiveEntry { path: "README.md".to_string(), contents: b"# orders\n".to_vec() },
            ArchiveEntry { path: "src/main.rs".to_string(), contents: b"fn main() {}\n".to_vec() },
        ]
    }

    #[test]
    fn zip_nests_members_under_the_root_and_keeps_contents() {
        let bytes = build_zip("orders", &entries()).expect("zip builds");
        let mut archive =
            zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("zip reads back");

        let names: Vec<String> = archive.file_names().map(String::from).collect();
        assert!(names.contains(&"orders/README.md".to_string()), "{:?}", names);
        assert!(names.contains(&"orders/src/main.rs".to_string()), "{:?}", names);
        assert!(names.contains(&"orders/src/".to_string()), "ancestor dir: {:?}", names);

        let mut file = archive.by_name("orders/src/main.rs").expect("member present");
        let mut contents = String::new();
        io::Read::read_to_string(&mut file, &mut contents).expect("readable");
        assert_eq!(contents, "fn main() {}\n");
    }

    #[test]
    fn tar_gz_round_trips() {
        let bytes = build_tar("orders", &entries(), true).expect("tar.gz builds");
        let decoder = flate2::read::GzDecoder::new(std::io::Cursor::new(bytes));
        let mut archive = tar::Archive::new(decoder);
        let paths: Vec<String> = archive
            .entries()
            .expect("entries")
            .map(|e| e.expect("entry").path().expect("path").display().to_string())
            .collect();
        assert!(paths.contains(&"orders/README.md".to_string()), "{:?}", paths);
        assert!(paths.contains(&"orders/src/main.rs".to_string()), "{:?}", paths);
    }

    #[test]
    fn an_empty_render_still_produces_a_readable_archive() {
        let bytes = build_zip("orders", &[]).expect("zip builds");
        let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("reads back");
        assert_eq!(archive.len(), 1, "just the root directory member");
    }
}
