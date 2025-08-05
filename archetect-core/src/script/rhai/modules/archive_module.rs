use std::fs::{File, create_dir_all};
use std::io::{self, Write};

use camino::{Utf8Path, Utf8PathBuf};
use log::info;
use rhai::{Engine, EvalAltResult, NativeCallContext};

use crate::archetype::render_context::RenderContext;
use crate::errors::{ArchetypeScriptError, ArchetypeScriptErrorWrapper};
use crate::script::rhai::modules::path_module::Path;
use crate::utils::restrict_path_manipulation;

pub(crate) fn register(engine: &mut Engine, render_context: RenderContext) {
    let render_context_clone = render_context.clone();
    engine.register_fn("zip", move |call: NativeCallContext, source: &str, destination: &str| {
        zip_directory(&call, render_context_clone.clone(), source, destination)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("zip", move |call: NativeCallContext, mut source: Path, destination: &str| {
        zip_path(&call, render_context_clone.clone(), &mut source, destination)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("tar", move |call: NativeCallContext, source: &str, destination: &str| {
        tar_directory(&call, render_context_clone.clone(), source, destination, false)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("tar", move |call: NativeCallContext, mut source: Path, destination: &str| {
        tar_path(&call, render_context_clone.clone(), &mut source, destination, false)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("tar_gz", move |call: NativeCallContext, source: &str, destination: &str| {
        tar_directory(&call, render_context_clone.clone(), source, destination, true)
    });

    let render_context_clone = render_context.clone();
    engine.register_fn("tar_gz", move |call: NativeCallContext, mut source: Path, destination: &str| {
        tar_path(&call, render_context_clone.clone(), &mut source, destination, true)
    });
}

fn zip_directory(
    call: &NativeCallContext,
    render_context: RenderContext,
    source: &str,
    destination: &str,
) -> Result<(), Box<EvalAltResult>> {
    let source_path = render_context.destination().join(restrict_path_manipulation(call, source)?);
    let dest_path = render_context.destination().join(restrict_path_manipulation(call, destination)?);

    if !source_path.exists() {
        let error = ArchetypeScriptError::PathNotFound {
            path: source_path.to_string(),
        };
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if !source_path.is_dir() {
        let error = ArchetypeScriptError::NotADirectory {
            path: source_path.to_string(),
        };
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent).map_err(|e| {
            let error = ArchetypeScriptError::IoError {
                message: format!("Failed to create parent directory: {}", e),
            };
            ArchetypeScriptErrorWrapper(call, error)
        })?;
    }

    info!("Creating zip archive: {} -> {}", source_path, dest_path);
    
    create_zip_archive(&source_path, &dest_path).map_err(|e| {
        let error = ArchetypeScriptError::ArchiveError {
            message: format!("Failed to create zip archive: {}", e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    Ok(())
}

fn zip_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    source: &mut Path,
    destination: &str,
) -> Result<(), Box<EvalAltResult>> {
    let source_str = source.path();
    zip_directory(call, render_context, source_str, destination)
}

fn tar_directory(
    call: &NativeCallContext,
    render_context: RenderContext,
    source: &str,
    destination: &str,
    compress: bool,
) -> Result<(), Box<EvalAltResult>> {
    let source_path = render_context.destination().join(restrict_path_manipulation(call, source)?);
    let dest_path = render_context.destination().join(restrict_path_manipulation(call, destination)?);

    if !source_path.exists() {
        let error = ArchetypeScriptError::PathNotFound {
            path: source_path.to_string(),
        };
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    if !source_path.is_dir() {
        let error = ArchetypeScriptError::NotADirectory {
            path: source_path.to_string(),
        };
        return Err(ArchetypeScriptErrorWrapper(call, error).into());
    }

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        create_dir_all(parent).map_err(|e| {
            let error = ArchetypeScriptError::IoError {
                message: format!("Failed to create parent directory: {}", e),
            };
            ArchetypeScriptErrorWrapper(call, error)
        })?;
    }

    let archive_type = if compress { "tar.gz" } else { "tar" };
    info!("Creating {} archive: {} -> {}", archive_type, source_path, dest_path);
    
    create_tar_archive(&source_path, &dest_path, compress).map_err(|e| {
        let error = ArchetypeScriptError::ArchiveError {
            message: format!("Failed to create {} archive: {}", archive_type, e),
        };
        ArchetypeScriptErrorWrapper(call, error)
    })?;

    Ok(())
}

fn tar_path(
    call: &NativeCallContext,
    render_context: RenderContext,
    source: &mut Path,
    destination: &str,
    compress: bool,
) -> Result<(), Box<EvalAltResult>> {
    let source_str = source.path();
    tar_directory(call, render_context, source_str, destination, compress)
}

// Implementation using zip crate
fn create_zip_archive(source_dir: &Utf8Path, dest_file: &Utf8Path) -> io::Result<()> {
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;
    
    let file = File::create(dest_file)?;
    let mut zip = ZipWriter::new(file);
    
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    add_directory_to_zip(&mut zip, source_dir, source_dir, &options)?;
    
    zip.finish()?;
    Ok(())
}

fn add_directory_to_zip(
    zip: &mut zip::ZipWriter<File>,
    base_path: &Utf8Path,
    dir_path: &Utf8Path,
    options: &zip::write::SimpleFileOptions,
) -> io::Result<()> {
    use std::fs::read_dir;
    
    for entry in read_dir(dir_path)? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 path"))?;
        
        let relative_path = path.strip_prefix(base_path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to get relative path"))?;
        
        if path.is_dir() {
            // Add directory entry
            let dir_name = format!("{}/", relative_path);
            zip.add_directory(&dir_name, *options)?;
            
            // Recursively add directory contents
            add_directory_to_zip(zip, base_path, &path, options)?;
        } else {
            // Add file
            let mut file = File::open(&path)?;
            zip.start_file(relative_path.as_str(), *options)?;
            io::copy(&mut file, zip)?;
        }
    }
    
    Ok(())
}

// Implementation using tar crate
fn create_tar_archive(source_dir: &Utf8Path, dest_file: &Utf8Path, compress: bool) -> io::Result<()> {
    use tar::Builder;
    
    let file = File::create(dest_file)?;
    
    let writer: Box<dyn Write> = if compress {
        Box::new(flate2::write::GzEncoder::new(file, flate2::Compression::default()))
    } else {
        Box::new(file)
    };
    
    let mut tar = Builder::new(writer);
    
    // Get the directory name to use as the root in the archive
    let dir_name = source_dir.file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid source directory"))?;
    
    tar.append_dir_all(dir_name, source_dir)?;
    tar.finish()?;
    
    Ok(())
}