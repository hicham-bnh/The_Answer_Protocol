use eframe::egui;
use crate::state::{prettify, ChatTab, Room};
use std::sync::mpsc::Receiver;

pub struct TapClient {
    server_address: String,
    username: String,
    room: Option<Room>,
    players: Vec<String>,
    server_players: u32,
    items: Vec<String>,
    npcs: Vec<String>,
    inventory: Vec<String>,
    hp: u32,
    max_hp: u32,
    group: Option<String>,
    chat_global: Vec<String>,
    chat_room: Vec<String>,
    chat_group: Vec<String>,
    logs: Vec<String>,
    active_tab: ChatTab,
    chat_input: String,
    rx: Receiver<String>
}

impl TapClient {
    fn new(address: String, rx: Receiver<String>) -> TapClient {
        TapClient {
            server_address: address,
            username: String::new(),
            room: None,
            players: Vec::new(),
            server_players: 0,
            items: Vec::new(),
            npcs: Vec::new(),
            inventory: Vec::new(),
            hp: 100,
            max_hp: 100,
            group: None,
            chat_global: Vec::new(),
            chat_room: Vec::new(),
            chat_group: Vec::new(),
            logs: Vec::new(),
            active_tab: ChatTab::default(),
            chat_input: String::new(),
            rx,
        }
    }

    pub fn fake(rx: Receiver<String>) -> TapClient {
        use std::collections::HashMap;

        TapClient {
            room: Some(Room {
                id: "loc.square".to_string(),
                name: "Village Square".to_string(),
                description: "The heart of the village. A stone fountain gurgles in the center."
                    .to_string(),
                exits: HashMap::from([
                    ("north".to_string(), "loc.tavern".to_string()),
                    ("east".to_string(), "loc.market".to_string()),
                    ("south".to_string(), "loc.road".to_string()),
                ]),
            }),
            players: vec!["alice".to_string(), "bob".to_string()],
            username: "Matthieu".to_string(),
            server_players: 2,
            items: vec!["item.herbs".to_string()],
            npcs: vec!["npc.guard".to_string()],
            inventory: vec!["item.bread".to_string(), "item.rusty_sword".to_string()],
            hp: 80,
            max_hp: 100,
            group: None,
            chat_global: vec![
                "alice: Hello everyone".to_string(),
                "bob: hey!".to_string(),
                "alice: anyone near the forge?".to_string(),
                "carol: selling herbs, meet at the market".to_string(),
            ],
            chat_room: vec![
                "bob: nice fountain".to_string(),
                "alice: the guard says the roads are dangerous".to_string(),
            ],
            chat_group: vec![
                "bob: let's do the goblin quest".to_string(),
                "you: I need to buy a sword first".to_string(),
            ],
            logs: vec![
                "Connected to 127.0.0.1:8080".to_string(),
                "OK connected".to_string(),
            ],
            ..TapClient::new("127.0.0.1:8080".to_string(), rx)
        }
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("main_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(format!(
                        "{}'s HP: {}/{}",
                        self.username, self.hp, self.max_hp
                    ))
                    .size(21.0),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.heading(format!("{} player(s) online", self.server_players));
                });
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if let Some(group) = &self.group {
                    ui.label(format!("Group: {group}"));
                    if ui.button("Leave").clicked() {
                        self.logs.push("GROUP LEAVE".to_string());
                    }
                } else {
                    ui.label("No group");
                    if ui.button("Create").clicked() {
                        self.logs.push("GROUP CREATE".to_string());
                    }
                }
            });
            ui.add_space(8.0);

            ui.heading("Actions: ");
            ui.horizontal(|ui| {
                let actions = ["LOOK", "STATUS", "INVENTORY", "WHO", "QUESTS", "QUIT"];
                for action in actions {
                    if ui.button(egui::RichText::new(action).size(18.0)).clicked() {
                        self.logs.push(action.to_string());
                    }
                }
            });
            ui.add_space(8.0);
        });
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("room_panel")
            .default_size(300.0)
            .show(ui, |ui| {
                if let Some(room) = &self.room {
                    ui.label(egui::RichText::new(&room.name).size(20.0));
                    ui.label(format!("{} player(s) in this room", self.players.len()));
                    ui.add_space(8.0);
                    ui.label(&room.description);

                    ui.separator();

                    ui.label(egui::RichText::new("Paths:").size(20.0));
                    for (direction, dest) in &room.exits {
                        ui.horizontal(|ui| {
                            ui.label(format!("{direction} -> {}", prettify(dest)));
                            if ui.button("Go").clicked() {
                                self.logs.push(format!("MOVE {direction}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Players here:").size(20.0));
                    for player in &self.players {
                        ui.horizontal(|ui| {
                            ui.label(player);
                            if ui.add_enabled(self.group.is_some(), egui::Button::new("Invite to group")).clicked() {
                                self.logs.push(format!("GROUP INVITE {player}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Items here:").size(20.0));
                    for item in &self.items {
                        ui.horizontal(|ui| {
                            ui.label(prettify(item));
                            if ui.button("Take").clicked() {
                                self.logs.push(format!("TAKE {item}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("NPCs:").size(20.0));
                    for npc in &self.npcs {
                        ui.horizontal(|ui| {
                            ui.label(prettify(npc));
                            if ui.button("Talk").clicked() {
                                self.logs.push(format!("TALK {npc}"));
                            }
                            if ui.button("Attack").clicked() {
                                self.logs.push(format!("ATTACK {npc}"));
                            }
                            if ui.button("Quest").clicked() {
                                self.logs.push(format!("QUEST {npc}"));
                            }
                        });
                    }
                } else {
                    ui.label(egui::RichText::new("Not connected.").size(20.0));
                    ui.add_space(8.0);
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                    if ui.button("Connect").clicked() {
                        self.logs.push(format!("CONNECT {}", self.username));
                    }
                }

                ui.separator();

                ui.label(egui::RichText::new("Inventory: ").size(20.0));
                for item in &self.inventory {
                    ui.horizontal(|ui| {
                        ui.label(prettify(item));
                        if ui.button("Drop").clicked() {
                            self.logs.push(format!("DROP {item}"));
                        }
                    });
                }
            });
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("log_list").show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for log in &self.logs {
                    ui.label(format!("- {}", log));
                }
            });
        });
    }

    fn bottom_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("chat_input").show(ui, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut self.chat_input).char_limit(200));
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let (scope, messages) = match self.active_tab {
                    ChatTab::Global => ("GLOBAL", &mut self.chat_global),
                    ChatTab::Room => ("ROOM", &mut self.chat_room),
                    ChatTab::Group => ("GROUP", &mut self.chat_group),
                };
                self.logs.push(format!("CHAT {scope} {}", self.chat_input));
                messages.push(format!("{}: {}", self.username, self.chat_input));
                self.chat_input.clear();
            }
        });
    }

    fn central_panel(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ChatTab::Global, "Global");
                ui.selectable_value(&mut self.active_tab, ChatTab::Room, "Room");
                ui.selectable_value(&mut self.active_tab, ChatTab::Group, "Group");
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                let messages = match self.active_tab {
                    ChatTab::Global => &self.chat_global,
                    ChatTab::Room => &self.chat_room,
                    ChatTab::Group => &self.chat_group,
                };
                for msg in messages {
                    ui.label(msg);
                }
            });
        });
    }
}

impl eframe::App for TapClient {

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.top_panel(ui);
        self.left_panel(ui);
        self.right_panel(ui);
        self.bottom_panel(ui);
        self.central_panel(ui);
    }
}
