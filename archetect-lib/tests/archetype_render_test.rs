use archetect::system::layout::LayoutType;
use archetect::ArchetectError;
use std::fs;
use archetect::archetype::ArchetypeError::ArchetypeInvalid;

#[test]
fn render_archetypes() -> Result<(), ArchetectError> {
    for entry in fs::read_dir("../archetypes").map_err(|e|ArchetectError::GenericError("Whoops!".to_owned())).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            println!("{}", entry.path().display());
        }
    }

    let archetect = archetect::Archetect::builder().with_layout_type(LayoutType::Temp)?.build()?;
    println!("{}", archetect.layout().cache_dir().display());

    Ok(())
}