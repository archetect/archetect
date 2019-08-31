use archetect::system::layout::LayoutType;
use archetect::ArchetectError;
use std::fs;
use archetect::archetype::ArchetypeError::ArchetypeInvalid;
use archetect::template_engine::Context;


#[test]
#[ignore]
fn render_archetypes() -> Result<(), ArchetectError> {
    let archetect = archetect::Archetect::builder()
        .with_layout_type(LayoutType::Temp)?
        .build()?;

    for entry in fs::read_dir("../archetypes").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {

            let path = path.to_str().unwrap();
            println!("{}", path);

//            let archetype = archetect.load_archetype(path, None)?;
//
//            let mut context = Context::new();
//            context.insert("name", "Example");
//            context.insert("author", "Joe Blow");
//            archetype.render("~/tmp", context);
        }

    }

    Ok(())
}