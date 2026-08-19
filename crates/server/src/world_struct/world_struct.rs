use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Location {
    pub name: String,
    pub description: String,
    pub exits: HashMap<String, String>,
    pub items: Vec<String>,
    pub npcs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Item {
    pub name: String,
    pub description: String,
    pub obtainable: bool,
}

#[derive(Debug, Deserialize)]
pub struct Npc {
    pub name: String,
    pub role: String,
    pub hostile: bool,
    pub description: String,
    pub dialogue: Vec<String>,
    #[serde(default)]
    pub quests: Vec<String>,
    pub stats: Stats,
    #[serde(default)]
    pub respawn_seconds: Option<u32>,
    pub engaged_by: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Stats {
    pub hp: u32,
    #[serde(default)]
    pub damage: Option<u32>,
    #[serde(default)]
    pub max_hp: u32,
}

#[derive(Debug, Deserialize)]
pub struct Quest {
    pub name: String,
    pub giver: String,
    #[serde(rename = "type")]
    pub quest_type: String,
    pub description: String,
    pub objective: Objective,
    pub reward: Reward,
}

#[derive(Debug, Deserialize)]
pub struct Objective {
    #[serde(default)]
    pub item: Option<String>,
    #[serde(default)]
    pub deliver_to: Option<String>,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub count: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct Reward {
    pub item: String,
}

#[derive(Debug, Deserialize)]
pub struct World {
    pub start_location: String,
    pub respawn_location: String,
    pub locations: HashMap<String, Location>,
}

#[derive(Debug, Deserialize)]
pub struct GameWorld {
    pub world: World,
    pub items: HashMap<String, Item>,
    pub npcs: HashMap<String, Npc>,
    pub quests: HashMap<String, Quest>,
}