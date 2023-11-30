use std::rc::Rc;

use crate::errors::ArchetectError;
use crate::errors::SystemError;
use crate::system::{dot_home_layout, LayoutType, NativeSystemLayout, RootedSystemLayout, SystemLayout};

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
        ArchetectBuilder {
            layout: None,
        }
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
            LayoutType::DotHome => self.with_layout(RootedSystemLayout::dot_home()?),
            LayoutType::Temp => self.with_layout(RootedSystemLayout::temp()?),
        };
        Ok(builder)
    }
}
