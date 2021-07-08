//use archetect_core::archetype::ArchetypeError::ArchetypeInvalid;
//use archetect_core::system::layout::LayoutType;
//use archetect_core::template_engine::Context;
// use archetect_core::ArchetectError;
use std::fs;

#[test]
// #[ignore]
fn render_archetypes() -> Result<(), Box<dyn std::error::Error>> {

    for entry in fs::read_dir("tests/archetypes")? {
        let archetype_suite = entry?;
        if archetype_suite.path().is_dir() {
            println!("{:?}", archetype_suite.path());
        }
    }

    //    let archetect = archetect_core::archetect::builder()
    //        .with_layout_type(LayoutType::Temp)?
    //        .build()?;
    //
    //    for entry in fs::read_dir("../test_archetypes").unwrap() {
    //        let entry = entry.unwrap();
    //        let path = entry.path();
    //        if path.is_dir() {
    //
    //            let path = path.to_str().unwrap();
    //            println!("{}", path);

    //            let archetype = archetect.load_archetype(path, None)?;
    //
    //            let mut context = Context::new();
    //            context.insert("name", "Example");
    //            context.insert("author", "Joe Blow");
    //            archetype.render("~/tmp", context);
    //        }
    //
    //    }

    Ok(())
}
