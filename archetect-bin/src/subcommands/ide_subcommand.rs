use std::fs;
use std::path::{Path, PathBuf};

use archetect_core::errors::ArchetectError;
use archetect_core::system::SystemLayout;

const ARCHETECT_LUA: &str = include_str!("../../../archetect-core/lua/annotations/archetect.lua");
const ARCHETECT_MODULES_LUA: &str = include_str!("../../../archetect-core/lua/annotations/archetect_modules.lua");

pub fn handle_ide_subcommand(layout: &dyn SystemLayout) -> Result<(), ArchetectError> {
    let annotations_dir = install_annotations(layout)?;
    maybe_create_luarc(&annotations_dir)?;
    Ok(())
}

fn install_annotations(layout: &dyn SystemLayout) -> Result<PathBuf, ArchetectError> {
    let annotations_dir = PathBuf::from(layout.data_dir().join("lua/annotations").as_str());

    fs::create_dir_all(&annotations_dir)
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to create {}: {}", annotations_dir.display(), e)))?;

    let archetect_path = annotations_dir.join("archetect.lua");
    fs::write(&archetect_path, ARCHETECT_LUA)
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to write {}: {}", archetect_path.display(), e)))?;

    let modules_path = annotations_dir.join("archetect_modules.lua");
    fs::write(&modules_path, ARCHETECT_MODULES_LUA)
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to write {}: {}", modules_path.display(), e)))?;

    eprintln!("archetect: Lua annotations installed to {}", annotations_dir.display());
    Ok(annotations_dir)
}

fn maybe_create_luarc(annotations_dir: &Path) -> Result<(), ArchetectError> {
    let cwd = std::env::current_dir()
        .map_err(|e| ArchetectError::GeneralError(format!("Failed to get current directory: {}", e)))?;

    let has_manifest = cwd.join("archetype.yaml").exists()
        || cwd.join("archetype.yml").exists();
    let has_lua_script = cwd.join("archetype.lua").exists();

    if has_manifest && has_lua_script {
        let luarc_path = cwd.join(".luarc.json");
        let luarc_content = format!(
            "{{\n  \"runtime.version\": \"Lua 5.4\",\n  \"workspace.library\": [\n    \"{}\"\n  ]\n}}\n",
            annotations_dir.display()
        );

        if luarc_path.exists() {
            eprintln!("archetect: .luarc.json already exists, skipping");
        } else {
            fs::write(&luarc_path, luarc_content)
                .map_err(|e| ArchetectError::GeneralError(format!("Failed to write .luarc.json: {}", e)))?;
            eprintln!("archetect: Created .luarc.json for IDE support");
        }
    } else if has_manifest && !has_lua_script {
        eprintln!("archetect: Archetype detected but no archetype.lua found — skipping .luarc.json");
    }

    Ok(())
}
