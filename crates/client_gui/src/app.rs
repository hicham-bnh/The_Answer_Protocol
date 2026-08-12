use crate::protocol::{EvtType, ServerMsg};
use crate::state::{prettify, ChatTab, Room};
use eframe::egui;
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

pub struct TapClient {
    server_address: String,
    connected: bool,
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
    pending_cmds: VecDeque<String>,
    rx: Receiver<String>,
    tx_out: Sender<String>,
}

impl TapClient {
    fn new(address: String, rx: Receiver<String>, tx_out: Sender<String>) -> TapClient {
        TapClient {
            server_address: address,
            connected: false,
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
            pending_cmds: VecDeque::new(),
            rx,
            tx_out,
        }
    }

    pub fn fake(rx: Receiver<String>, tx_out: Sender<String>) -> TapClient {
        use std::collections::HashMap;

        TapClient {
            connected: true,
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
            ..TapClient::new("127.0.0.1:8080".to_string(), rx, tx_out)
        }
    }

    fn send_command(&mut self, cmd: String) {
        let log_line = format!(">> {cmd}");
        if self.tx_out.send(cmd.clone()).is_err() {
            self.logs.push("(not connected)".to_string());
            return;
        }
        self.pending_cmds.push_back(cmd);
        self.logs.push(log_line);
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        let mut pending: Option<String> = None;

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
                        pending = Some("GROUP LEAVE".to_string());
                    }
                } else {
                    ui.label("No group");
                    if ui.button("Create").clicked() {
                        pending = Some("GROUP CREATE".to_string());
                    }
                }
            });
            ui.add_space(8.0);

            ui.heading("Actions: ");
            ui.horizontal(|ui| {
                let actions = ["LOOK", "STATUS", "INVENTORY", "WHO", "QUESTS", "QUIT"];
                for action in actions {
                    if ui.button(egui::RichText::new(action).size(18.0)).clicked() {
                        pending = Some(action.to_string());
                    }
                }
            });
            ui.add_space(8.0);
        });

        if let Some(cmd) = pending {
            self.send_command(cmd);
        }
    }

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        let mut pending: Option<String> = None;

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
                                pending = Some(format!("MOVE {direction}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Players here:").size(20.0));
                    for player in &self.players {
                        ui.horizontal(|ui| {
                            ui.label(player);
                            if ui
                                .add_enabled(
                                    self.group.is_some(),
                                    egui::Button::new("Invite to group"),
                                )
                                .clicked()
                            {
                                pending = Some(format!("GROUP INVITE {player}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("Items here:").size(20.0));
                    for item in &self.items {
                        ui.horizontal(|ui| {
                            ui.label(prettify(item));
                            if ui.button("Take").clicked() {
                                pending = Some(format!("TAKE {item}"));
                            }
                        });
                    }

                    ui.separator();

                    ui.label(egui::RichText::new("NPCs:").size(20.0));
                    for npc in &self.npcs {
                        ui.horizontal(|ui| {
                            ui.label(prettify(npc));
                            if ui.button("Talk").clicked() {
                                pending = Some(format!("TALK {npc}"));
                            }
                            if ui.button("Attack").clicked() {
                                pending = Some(format!("ATTACK {npc}"));
                            }
                            if ui.button("Quest").clicked() {
                                pending = Some(format!("QUEST {npc}"));
                            }
                        });
                    }
                } else {
                    ui.label(egui::RichText::new("Not connected.").size(20.0));
                    ui.add_space(8.0);
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                    if ui.button("Connect").clicked() {
                        pending = Some(format!("CONNECT {}", self.username));
                    }
                }

                ui.separator();

                ui.label(egui::RichText::new("Inventory: ").size(20.0));
                for item in &self.inventory {
                    ui.horizontal(|ui| {
                        ui.label(prettify(item));
                        if ui.button("Drop").clicked() {
                            pending = Some(format!("DROP {item}"));
                        }
                    });
                }
            });
        if let Some(cmd) = pending {
            self.send_command(cmd);
        }
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
        let mut pending: Option<String> = None;

        egui::Panel::bottom("chat_input").show(ui, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut self.chat_input).char_limit(200));
            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                let (scope, messages) = match self.active_tab {
                    ChatTab::Global => ("GLOBAL", &mut self.chat_global),
                    ChatTab::Room => ("ROOM", &mut self.chat_room),
                    ChatTab::Group => ("GROUP", &mut self.chat_group),
                };
                pending = Some(format!("CHAT {scope} {}", self.chat_input));
                messages.push(format!("{}: {}", self.username, self.chat_input));
                self.chat_input.clear();
            }
        });

        if let Some(cmd) = pending {
            self.send_command(cmd);
        }
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

    fn apply(&mut self, msg: ServerMsg) {
        match msg {
            ServerMsg::Evt(evt) => match evt {
                EvtType::Enter(username) => self.players.push(username),
                EvtType::Leave(username) => self.players.retain(|x| x != &username),
                EvtType::Chat(tab, username, message) => match tab {
                    ChatTab::Global => self.chat_global.push(format!("{}: {}", username, message)),
                    ChatTab::Room => self.chat_room.push(format!("{}: {}", username, message)),
                    ChatTab::Group => self.chat_group.push(format!("{}: {}", username, message)),
                },
                EvtType::GroupInvite(leader) => {
                    self.logs.push(format!(
                        "Group invite from {leader} (GROUP JOIN {leader} to accept)"
                    ));
                }
                EvtType::GroupJoin(username) => {
                    self.chat_group
                        .push(format!("* {username} joined the group"));
                }
                EvtType::GroupLeave(username) => {
                    self.chat_group.push(format!("* {username} left the group"));
                }
                EvtType::PlayerCount(data) => self.server_players = data,
            },

            ServerMsg::Ok(data) => match self.pending_cmds.pop_front() {
                Some(cmd) => {
                    let mut splitter = cmd.splitn(3, ' ');

                    match (splitter.next(), splitter.next(), splitter.next()) {
                        (Some("CONNECT"), Some("connected"), None) => {
                            self.connected = true;
                            self.send_command("LOOK".to_string());
                            self.send_command("INVENTORY".to_string());
                            self.send_command("STATUS".to_string());
                        }

                        (Some("MOVE"), _, None) => {
                            self.send_command("LOOK".to_string());
                        }

                        (Some("TAKE"), Some(value), None) => {
                            let mut v_splitter = value.splitn(2, '=');

                            match (v_splitter.next(), v_splitter.next()) {
                                (Some("taken"), Some(item)) => {
                                    self.items.retain(|x| x != item);
                                    self.inventory.push(item.to_string());
                                }
                                _ => self.logs.push(format!("OK to '{cmd}': {data}")),
                            }
                        }

                        (Some("DROP"), Some(value), None) => {
                            let mut v_splitter = value.splitn(2, '=');

                            match (v_splitter.next(), v_splitter.next()) {
                                (Some("dropped"), Some(item)) => {
                                    self.inventory.retain(|x| x != item);
                                    self.items.push(item.to_string());
                                }
                                _ => self.logs.push(format!("OK to '{cmd}': {data}")),
                            }
                        }

                        (Some("GROUP"), Some(grp_action), rest) => match grp_action {
                            "JOIN" | "CREATE" => todo!(),
                        },

                        (Some("CHAT"), _, _) => {}

                        (Some("QUIT"), _, _) => {}

                        _ => self.logs.push(format!("OK to '{cmd}': {data}")),
                    }
                }
                None => self.logs.push(format!("Unsolicited OK: {data}")),
            },

            ServerMsg::Err(code, description) => match self.pending_cmds.pop_front() {
                Some(cmd) => self
                    .logs
                    .push(format!("ERR to '{cmd}': {code} {description}")),
                None => self
                    .logs
                    .push(format!("Unsolicited ERR: {code} {description}")),
            },
            ServerMsg::Unknown(data) => self.logs.push(data.to_string()),
        }
    }
}

impl eframe::App for TapClient {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(line) = self.rx.try_recv() {
            let msg = crate::protocol::parse_line(&line);
            self.logs.push(format!("{msg:?}"));
            self.apply(msg);
        }
        self.top_panel(ui);
        self.left_panel(ui);
        self.right_panel(ui);
        self.bottom_panel(ui);
        self.central_panel(ui);
    }
}
