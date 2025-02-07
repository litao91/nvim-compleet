use serde::Deserialize;

use super::{completion::CompletionSettings, sources, ui::UiSettings};
use crate::state::Sources;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    #[serde(default)]
    pub ui: UiSettings,

    #[serde(default)]
    pub completion: CompletionSettings,

    #[serde(default, with = "sources")]
    pub sources: Sources,
}
