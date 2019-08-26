use crate::util::{SystemPaths, DirectorySystemPaths};
use std::rc::Rc;

pub struct Archetect {
    paths: Rc<Box<dyn SystemPaths>>
}

impl Archetect {

    pub fn paths(&self) -> Rc<Box<dyn SystemPaths>> {
        self.paths.clone()
    }

    pub fn builder() -> ArchetectBuilder {
        ArchetectBuilder::new()
    }
}

pub struct ArchetectBuilder {
    paths: Option<Box<dyn SystemPaths>>,
}

impl ArchetectBuilder {
    fn new() -> ArchetectBuilder {
        ArchetectBuilder{ paths: None }
    }

    pub fn build(self) -> Result<Archetect, String> {
        let result = shellexpand::full("~/.architect/").unwrap();
        let paths = DirectorySystemPaths::new(result.to_string())?;
        let paths = self.paths.unwrap_or_else(|| Box::new(paths));
        let paths = Rc::new(paths);
        Ok(Archetect{ paths })
    }

    pub fn with_paths<P: SystemPaths + 'static>(mut self, paths: P) -> ArchetectBuilder {
        self.paths = Some(Box::new(paths));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::NativeSystemPaths;

    #[test]
    fn test_explicit_native_paths() {
        let archetect = Archetect::builder().with_paths(NativeSystemPaths::new().unwrap()).build().unwrap();

        println!("{}", archetect.paths().user_config().display());
    }

    #[test]
    fn test_explicit_directory_paths() {
        let paths = DirectorySystemPaths::new("~/.archetect/").unwrap();
        let archetect = Archetect::builder().with_paths(paths).build().unwrap();

        println!("{}", archetect.paths().user_config().display());
    }

    #[test]
    fn test_implicit() {
        let archetect = Archetect::builder().build().unwrap();

        println!("{}", archetect.paths().user_config().display());

        std::fs::create_dir_all(archetect.paths().configs_dir()).expect("Error creating directory");
        std::fs::create_dir_all(archetect.paths().git_cache_dir()).expect("Error creating directory");
    }
}