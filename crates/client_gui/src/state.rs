use std::collections::HashMap;

pub struct Room {
    pub id: String,
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>,
}

#[derive(Default, PartialEq, Clone, Debug)]
pub enum ChatTab {
    Global,
    #[default]
    Room,
    Group,
}

pub fn prettify(id: &str) -> String {
    for prefix in ["loc.", "item.", "npc."] {
        if let Some(rest) = id.strip_prefix(prefix) {
            return rest.replace("_", " ");
        }
    }
    id.to_string()
}
