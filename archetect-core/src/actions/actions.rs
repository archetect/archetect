use crate::actions::action_info::{RenderArchetypeInfo, RenderCatalogInfo, RenderGroupInfo};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ArchetectAction {
    #[serde(rename = "render_group", alias = "group")]
    RenderGroup{
        description: String,
        #[serde(flatten)]
        info: RenderGroupInfo,
    },
    #[serde(rename = "render_catalog", alias = "catalog")]
    RenderCatalog {
        description: String,
        #[serde(flatten)]
        info: RenderCatalogInfo,
    },
    #[serde(rename = "render_archetype", alias="archetype")]
    RenderArchetype{
        description: String,
        #[serde(flatten)]
        info: RenderArchetypeInfo,
    },
}