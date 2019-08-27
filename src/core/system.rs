use std::rc::Rc;

use crate::{Archetype, ArchetypeError, ArchetectError};
use crate::template_engine::Context;
use crate::system::layout::{dot_home_layout, SystemLayout, LayoutType, NativeSystemLayout};
use crate::system::SystemError;
use crate::util::Source;
use crate::config::AnswerConfig;

pub struct Archetect {
    paths: Rc<Box<dyn SystemLayout>>,
    offline: bool,
}

impl Archetect {

    pub fn layout(&self) -> Rc<Box<dyn SystemLayout>> {
        self.paths.clone()
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::new()
    }

    pub fn build() -> Result<Archetect, ArchetectError> {
        ArchetectBuilder::new().build()
    }

    pub fn load_archetype(&self, source: &str, relative_to: Option<Source>) -> Result<Archetype, ArchetectError> {
        let source = Source::detect(source, self.offline, relative_to)?;
        Archetype::from_source(source, self.offline).map_err(|e| e.into())
    }

    pub fn render_string(&self, _template: &str, _context: Context) -> Result<String, String> {
        unimplemented!()
    }
}

pub struct ArchetectBuilder {
    layout: Option<Box<dyn SystemLayout>>,
    offline: bool,
}

impl ArchetectBuilder {
    fn new() -> ArchetectBuilder {
        ArchetectBuilder{ layout: None, offline: false }
    }

    pub fn build(self) -> Result<Archetect, ArchetectError> {
        let layout = dot_home_layout()?;
        let paths = self.layout.unwrap_or_else(|| Box::new(layout));
        let paths = Rc::new(paths);
        Ok(Archetect{ paths, offline: self.offline })
    }

    pub fn with_layout<P: SystemLayout + 'static>(mut self, layout: P) -> ArchetectBuilder {
        self.layout = Some(Box::new(layout));
        self
    }

    pub fn with_layout_type(self, layout: LayoutType) -> Result<ArchetectBuilder, SystemError> {
        let builder = match layout {
            LayoutType::Native => self.with_layout(NativeSystemLayout::new()?),
            LayoutType::DotHome => self.with_layout(dot_home_layout()?),
            LayoutType::Temp => self.with_layout(crate::system::layout::temp_layout()?),
        };
        Ok(builder)
    }

    pub fn with_offline(mut self, offline: bool) -> ArchetectBuilder {
        self.offline = offline;
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::system::layout::{NativeSystemLayout, RootedSystemLayout};

    use super::*;

    #[test]
    fn test_explicit_native_paths() {
        let archetect = Archetect::builder().with_layout(NativeSystemLayout::new().unwrap()).build().unwrap();

        println!("{}", archetect.layout().user_config().display());
    }

    #[test]
    fn test_explicit_directory_paths() {
        let paths = RootedSystemLayout::new("~/.archetect/").unwrap();
        let archetect = Archetect::builder().with_layout(paths).build().unwrap();

        println!("{}", archetect.layout().user_config().display());
    }

    #[test]
    fn test_implicit() {
        let archetect = Archetect::build().unwrap();
        
        println!("{}", archetect.layout().user_config().display());

        std::fs::create_dir_all(archetect.layout().configs_dir()).expect("Error creating directory");
        std::fs::create_dir_all(archetect.layout().git_cache_dir()).expect("Error creating directory");
    }
}