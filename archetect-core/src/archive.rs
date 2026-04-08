use std::fs::{self, File};
use std::io::{self, Write};

use camino::{Utf8Path, Utf8PathBuf};

/// Create a zip archive from a directory.
pub fn create_zip_archive(source_dir: &Utf8Path, dest_file: &Utf8Path) -> io::Result<()> {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    let file = File::create(dest_file)?;
    let mut zip = ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    let dir_name = source_dir
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source directory"))?;

    zip.add_directory(format!("{}/", dir_name), options)?;
    add_directory_to_zip(&mut zip, source_dir, source_dir, dir_name, &options)?;
    zip.finish()?;
    Ok(())
}

fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<File>,
    base_path: &Utf8Path,
    dir_path: &Utf8Path,
    prefix: &str,
    options: &zip::write::SimpleFileOptions,
) -> io::Result<()> {
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 path"))?;

        let relative_path = path
            .strip_prefix(base_path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to get relative path"))?;

        let archive_path = format!("{}/{}", prefix, relative_path);

        if path.is_dir() {
            zip.add_directory(format!("{}/", archive_path), *options)?;
            add_directory_to_zip(zip, base_path, &path, prefix, options)?;
        } else {
            let mut file = File::open(&path)?;
            zip.start_file(&archive_path, *options)?;
            io::copy(&mut file, zip)?;
        }
    }
    Ok(())
}

/// Create a tar (optionally gzipped) archive from a directory.
pub fn create_tar_archive(source_dir: &Utf8Path, dest_file: &Utf8Path, compress: bool) -> io::Result<()> {
    use tar::Builder;

    let file = File::create(dest_file)?;

    let writer: Box<dyn Write> = if compress {
        Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::default()))
    } else {
        Box::new(file)
    };

    let mut tar = Builder::new(writer);

    let dir_name = source_dir
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source directory"))?;

    tar.append_dir_all(dir_name, source_dir)?;
    tar.finish()?;

    Ok(())
}
