use archetect::system::layout::LayoutType;
use archetect::ArchetectError;

#[test]
fn render_archetypes() -> Result<(), ArchetectError> {
//    for entry in fs::read_dir("./archetypes")? {
//        let entry = entry?;
//        let path = entry.path();
//        if path.is_dir() {
//            println!("{}", entry.path().display());
//        }
//    }

    let archetect = archetect::Archetect::builder().with_layout_type(LayoutType::Temp)?.build()?;
    println!("{}", archetect.layout().cache_dir().display());

    Ok(())
}