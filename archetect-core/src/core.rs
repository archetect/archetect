use std::rc::Rc;

use crate::system::SystemError;
use crate::system::{dot_home_layout, LayoutType, NativeSystemLayout, SystemLayout};
use crate::ArchetectError;

pub struct Archetect {
    paths: Rc<Box<dyn SystemLayout>>,
}

impl Archetect {
    pub fn layout(&self) -> Rc<Box<dyn SystemLayout>> {
        self.paths.clone()
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::new()
    }

    pub fn build() -> Result<Archetect, ArchetectError> {
        ArchetectBuilder::new().build()
    }
}

pub struct ArchetectBuilder {
    layout: Option<Box<dyn SystemLayout>>,
}

impl ArchetectBuilder {
    fn new() -> ArchetectBuilder {
        ArchetectBuilder { layout: None }
    }

    pub fn build(self) -> Result<Archetect, ArchetectError> {
        let layout = dot_home_layout()?;
        let paths = self.layout.unwrap_or_else(|| Box::new(layout));
        let paths = Rc::new(paths);

        Ok(Archetect { paths })
    }

    pub fn with_layout<P: SystemLayout + 'static>(mut self, layout: P) -> ArchetectBuilder {
        self.layout = Some(Box::new(layout));
        self
    }

    pub fn with_layout_type(self, layout: LayoutType) -> Result<ArchetectBuilder, SystemError> {
        let builder = match layout {
            LayoutType::Native => self.with_layout(NativeSystemLayout::new()?),
            LayoutType::DotHome => self.with_layout(dot_home_layout()?),
            LayoutType::Temp => self.with_layout(crate::system::temp_layout()?),
        };
        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use crate::system::{NativeSystemLayout, RootedSystemLayout};

    use super::*;

    #[test]
    fn test_explicit_native_paths() {
        let archetect = Archetect::builder()
            .with_layout(NativeSystemLayout::new().unwrap())
            .build()
            .unwrap();

        println!("{}", archetect.layout().catalog_cache_dir());
    }

    #[test]
    fn test_explicit_directory_paths() {
        let paths = RootedSystemLayout::new("~/.archetect/").unwrap();
        let archetect = Archetect::builder().with_layout(paths).build().unwrap();

        println!("{}", archetect.layout().catalog_cache_dir());
    }

    #[test]
    fn test_implicit() {
        let archetect = Archetect::build().unwrap();

        println!("{}", archetect.layout().catalog_cache_dir());

        std::fs::create_dir_all(archetect.layout().configs_dir()).expect("Error creating directory");
        std::fs::create_dir_all(archetect.layout().git_cache_dir()).expect("Error creating directory");
    }
}
