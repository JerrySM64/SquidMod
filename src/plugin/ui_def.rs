use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum WidgetDef {
    #[serde(rename = "switch")]
    Switch {
        title: String,
        subtitle: String,
    },
    #[serde(rename = "entry")]
    Entry {
        title: String,
        placeholder: String,
        max_chars: u32,
    },
    #[serde(rename = "dropdown")]
    Dropdown {
        title: String,
        subtitle: String,
        items: String,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RowDef {
    pub title: String,
    pub subtitle: Option<String>,
    pub widgets: Vec<WidgetDef>,
}
