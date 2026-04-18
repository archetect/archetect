use std::fs;

use camino::Utf8PathBuf;
use content_inspector::ContentType;
use log::debug;
use similar::{ChangeTag, TextDiff};

use archetect_api::{ClientMessage, ExistingFilePolicy, WriteFileInfo};
use crate::responder::Responder;
use inquire::Confirm;

/// Cap diff output so a giant generated file doesn't flood the terminal.
/// Authors can still inspect the full file on disk after the render.
const MAX_DIFF_LINES: usize = 200;

pub fn handle_write_file(write_info: WriteFileInfo, responses: &dyn Responder) {
    let path = Utf8PathBuf::from(&write_info.destination);

    if path.exists() {
        match write_info.existing_file_policy {
            ExistingFilePolicy::Overwrite => {
                debug!("Overwriting {:?}", path);
                report_diff(&path, &write_info.contents);
            }
            ExistingFilePolicy::Preserve => {
                debug!("Preserving {:?}", path);
                responses.respond(ClientMessage::Ack);
                return;
            }
            ExistingFilePolicy::Prompt => {
                report_diff(&path, &write_info.contents);
                let overwrite = Confirm::new(format!("Overwrite '{}'?", path).as_str())
                    .prompt_skippable()
                    .unwrap_or_default()
                    .unwrap_or_default();
                if !overwrite {
                    debug!("Preserving {:?}", path);
                    responses.respond(ClientMessage::Ack);
                    return;
                }
                debug!("Overwriting {:?}", path);
            }
            ExistingFilePolicy::Error => {
                // Hard-fail — idempotent-render contract violation.
                responses.respond(ClientMessage::Error(format!(
                    "File already exists: {} (if_exists = Existing.Error)",
                    path
                )));
                return;
            }
        }
    } else {
        debug!("Writing {:?}", path);
    }

    match fs::write(&path, write_info.contents) {
        Ok(()) => {
            responses.respond(ClientMessage::Ack);
        }
        Err(error) => {
            responses.respond(ClientMessage::Error(error.to_string()));
        }
    }
}

/// Print a unified diff between the existing file's contents on disk and the
/// new contents about to be written. Best-effort: read failures are silently
/// skipped so the write itself isn't blocked by diff trouble.
fn report_diff(path: &Utf8PathBuf, new_contents: &[u8]) {
    let existing = match fs::read(path) {
        Ok(b) => b,
        Err(_) => return,
    };

    if existing == new_contents {
        eprintln!("--- {} (no change) ---", path);
        return;
    }

    let existing_binary = matches!(
        content_inspector::inspect(&existing),
        ContentType::BINARY
    );
    let new_binary = matches!(
        content_inspector::inspect(new_contents),
        ContentType::BINARY
    );

    if existing_binary || new_binary {
        eprintln!(
            "--- {} (binary, {} → {} bytes) ---",
            path,
            existing.len(),
            new_contents.len()
        );
        return;
    }

    let old_text = String::from_utf8_lossy(&existing);
    let new_text = String::from_utf8_lossy(new_contents);
    let diff = TextDiff::from_lines(&old_text, &new_text);

    eprintln!("--- {} ---", path);
    let mut emitted = 0usize;
    for change in diff.iter_all_changes() {
        if emitted >= MAX_DIFF_LINES {
            eprintln!("... (diff truncated at {} lines)", MAX_DIFF_LINES);
            return;
        }
        let prefix = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        // change.value() includes the trailing newline; trim it so we can
        // re-emit one consistently.
        let line = change.value().trim_end_matches('\n');
        eprintln!("{}{}", prefix, line);
        emitted += 1;
    }
}
