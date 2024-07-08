use std::fmt::{Display, Formatter};
use std::rc::Rc;

use linked_hash_map::LinkedHashMap;

use archetect_api::{ClientMessage, ScriptMessage, SelectPromptInfo};

use crate::actions::ArchetectAction;
use crate::Archetect;
use crate::archetype::render_context::RenderContext;
use crate::catalog::CatalogManifest;
use crate::errors::{ArchetectError, CatalogError};
use crate::source::Source;

#[derive(Clone)]
pub struct Catalog {
    archetect: Archetect,
    pub(crate) inner: Rc<Inner>,
}

pub(crate) struct Inner {
    source: Option<Source>,
    manifest: CatalogManifest,
}

impl Catalog {
    pub fn load(archetect: Archetect, source: Source) -> Result<Catalog, CatalogError> {
        let manifest = CatalogManifest::load(source.path()?)?;
        let inner = Rc::new(Inner {
            source: Some(source),
            manifest,
        });
        let catalog = Catalog { archetect, inner };
        Ok(catalog)
    }

    pub fn new(archetect: Archetect, manifest: CatalogManifest) -> Self {
        Catalog {
            archetect,
            inner: Rc::new(Inner { source: None, manifest }),
        }
    }

    pub fn source(&self) -> &Option<Source> {
        &self.inner.source
    }

    pub fn check_requirements(&self) -> Result<(), CatalogError> {
        self.inner.manifest.requires().check_requirements(&self.archetect)?;
        Ok(())
    }

    pub fn entries(&self) -> &[ArchetectAction] {
        self.inner.manifest.entries()
    }

    pub fn render(&self, render_context: RenderContext) -> Result<(), ArchetectError> {
        let mut catalog = self.clone();

        loop {
            let entries = catalog.inner.manifest.entries().to_owned();
            if entries.is_empty() {
                return Err(CatalogError::EmptyCatalog.into());
            }

            let choice = self.select_from_entries(entries)?;

            match choice {
                ArchetectAction::RenderCatalog { description: _, info } => {
                    catalog = self.archetect.new_catalog(info.source())?;
                }
                ArchetectAction::RenderArchetype { description: _, info } => {
                    let archetype = self.archetect.new_archetype(info.source())?;

                    archetype.check_requirements()?;
                    let _result = archetype.render(render_context.extend_with(&info))?;
                    return Ok(());
                }
                ArchetectAction::RenderGroup {
                    description: _,
                    info: _,
                } => unreachable!(),
                ArchetectAction::Connect { info, .. } => {
                    crate::client::start(render_context.extend_with(&info), info.endpoint)?;
                    return Ok(());
                }
            }
        }
    }

    pub fn select_from_entries(
        &self,
        mut entry_items: Vec<ArchetectAction>,
    ) -> Result<ArchetectAction, ArchetectError> {
        if entry_items.is_empty() {
            return Err(CatalogError::EmptyGroup.into());
        }

        loop {
            let options_map = create_options_map(entry_items);

            let options = options_map.iter().map(|(k, _v)| k.to_owned()).collect::<Vec<_>>();
            let default = options.get(0).map(|v| v.to_owned());

            let key: Option<String> = None;

            // TODO: handle page size
            let prompt_info = SelectPromptInfo::new("Catalog Selection:", key, options).with_default(default);

            self.archetect.request(ScriptMessage::PromptForSelect(prompt_info))?;

            match self.archetect.receive()? {
                ClientMessage::String(answer) => {
                    // TODO: Handle item missing error
                    let action = options_map.get(&answer).expect("Required Catalog Item").clone();
                    match action {
                        ArchetectAction::RenderGroup { description: _, info } => {
                            entry_items = info.actions_owned();
                        }
                        ArchetectAction::RenderCatalog { .. } => return Ok(action),
                        ArchetectAction::RenderArchetype { .. } => return Ok(action),
                        ArchetectAction::Connect { .. } => return Ok(action),
                    }
                }
                ClientMessage::None => {
                    return Err(CatalogError::SelectionCancelled.into());
                }
                ClientMessage::Abort => {
                    return Err(CatalogError::SelectionCancelled.into());
                }
                _response => {
                    // TODO: Better error handling
                    return Err(CatalogError::SelectionCancelled.into());
                }
            }
        }
    }
}

fn create_options_map(entry_items: Vec<ArchetectAction>) -> LinkedHashMap<String, ArchetectAction> {
    let mut map = LinkedHashMap::new();

    let item_count = entry_items.len();

    for (id, entry) in entry_items.into_iter().enumerate() {
        match item_count {
            1..=99 => {
                let display = format!("{:>02}: {} {}", id + 1, item_icon(&entry), entry.description());
                map.insert(display, entry);
            }
            100..=999 => {
                let display = format!("{:>003}: {} {}", id + 1, item_icon(&entry), entry.description());
                map.insert(display, entry);
            }
            _ => {
                let display = format!("{:>0004}: {} {}", id + 1, item_icon(&entry), entry.description());
                map.insert(display, entry);
            }
        }
    }
    map
}

fn item_icon(entry: &ArchetectAction) -> &'static str {
    match entry {
        ArchetectAction::RenderArchetype { .. } => "📦",
        _ => "📂",
    }
}

pub(crate) struct CatalogItem {
    text: String,
    pub(crate) entry: ArchetectAction,
}

impl CatalogItem {
    pub fn new(text: String, entry: ArchetectAction) -> CatalogItem {
        CatalogItem { text, entry }
    }
    pub fn entry(self) -> ArchetectAction {
        self.entry
    }
}

impl Display for CatalogItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
