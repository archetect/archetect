
use std::fs;
use std::path::PathBuf;

#[test]
fn render_archetypes() -> Result<(), std::io::Error> {
    for entry in fs::read_dir("./archetypes")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            println!("{}", entry.path().display());
        }
    }

    Ok(())
}