use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Location {
    name: String,
    description: String,
    exits: HashMap<String, String>,
    items: Vec<String>,
    npcs: Vec<String>
}

#[derive(Debug, Deserialize)]
struct Item {
    name: String,
    description: String,
    obtainable: bool
}

#[derive(Debug, Deserialize)]
struct Npc {
    name: String,
    role: String,
    hostile: bool,
    description: String,
    dialogue: Vec<String>,
    #[serde(default)]
    quests: Vec<String>,
    stats: Stats,   
    #[serde(default)]
    respawn_seconds: Option<u32>
}

#[derive(Debug, Deserialize)]
struct Stats {
    hp: u32,
    #[serde(default)]
    damage: Option<u32>
}

#[derive(Debug, Deserialize)]
struct Quest {
    name: String,
    giver: String,
    #[serde(rename = "type")]
    quest_type: String,
    description: String,
    objective: Objective,
    reward: Reward
}

#[derive(Debug, Deserialize)]
struct Objective {
    #[serde(default)]
    item: Option<String>,
    #[serde(default)]
    deliver_to: Option<String>,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Reward {
    item: String
}

#[derive(Debug, Deserialize)]
struct World {
    start_location: String,
    respawn_location: String,
    locations: HashMap<String, Location>,
}

#[derive(Debug, Deserialize)]
struct GameWorld {
    world: World,
    items: HashMap<String, Item>,
    npcs: HashMap<String, Npc>,
    quests: HashMap<String, Quest>
}