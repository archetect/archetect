use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path};
use std::rc::Rc;

use semver::Version;

use crate::system::{dot_home_layout, LayoutType, NativeSystemLayout, SystemLayout};
use crate::system::SystemError;
use crate::source::Source;
use crate::{ArchetectError, Archetype, ArchetypeError, RenderError};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Archetect {
    paths: Rc<Box<dyn SystemLayout>>,
    offline: bool,
    headless: bool,
    switches: HashSet<String>,
}

impl Archetect {
    pub fn layout(&self) -> Rc<Box<dyn SystemLayout>> {
        self.paths.clone()
    }

    pub fn offline(&self) -> bool {
        self.offline
    }

    pub fn headless(&self) -> bool {
        self.headless
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::new()
    }

    pub fn build() -> Result<Archetect, ArchetectError> {
        ArchetectBuilder::new().build()
    }

    pub fn enable_switch<S: Into<String>>(&mut self, switch: S) {
        self.switches.insert(switch.into());
    }

    pub fn switches(&self) -> &HashSet<String> {
        &self.switches
    }

    pub fn load_archetype(&self, source: &str, relative_to: Option<Source>) -> Result<Archetype, ArchetypeError> {
        let source = Source::detect(self, source, relative_to)?;
        let archetype = Archetype::from_source(&source)?;
        Ok(archetype)
    }

    pub fn write_contents<P: AsRef<Path>>(&self, destination: P, contents: &str) -> Result<(), RenderError> {
        let destination = destination.as_ref();
        let mut output = File::create(&destination)?;
        output.write(contents.as_bytes())?;
        Ok(())
    }

    pub fn copy_contents<S: AsRef<Path>, D: AsRef<Path>>(&self, source: S, destination: D) -> Result<(), RenderError> {
        let source = source.as_ref();
        let destination = destination.as_ref();
        fs::copy(source, destination)?;
        Ok(())
    }

    pub fn version(&self) -> Version {
        Version::parse(VERSION).unwrap()
    }
}

pub struct ArchetectBuilder {
    layout: Option<Box<dyn SystemLayout>>,
    offline: bool,
    headless: bool,
    switches: HashSet<String>,
}

impl ArchetectBuilder {
    fn new() -> ArchetectBuilder {
        ArchetectBuilder {
            layout: None,
            offline: false,
            headless: false,
            switches: HashSet::new(),
        }
    }

    pub fn build(self) -> Result<Archetect, ArchetectError> {
        let layout = dot_home_layout()?;
        let paths = self.layout.unwrap_or_else(|| Box::new(layout));
        let paths = Rc::new(paths);

        Ok(Archetect {
            paths,
            offline: self.offline,
            headless: self.headless,
            switches: self.switches,
        })
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

    pub fn with_offline(mut self, offline: bool) -> ArchetectBuilder {
        self.offline = offline;
        self
    }

    pub fn with_headless(mut self, headless: bool) -> ArchetectBuilder {
        self.headless = headless;
        self
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
