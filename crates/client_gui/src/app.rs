use crate::protocol::{
    CombatReply, EvtType, LookReply, QuestReply, ServerMsg, StatusReply, TalkReply, WhoReply,
};
use crate::state::{prettify, ChatTab, Room};
use eframe::egui;
use std::collections::VecDeque;
use std::sync::mpsc::{Receiver, Sender};

pub struct TapClient {
    server_address: String,
    connected: bool,
    disconnected: bool,
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
    group_invite: Option<String>,
    in_combat: bool,
    combat_target: Option<String>,
    combat_target_hp: u32,
    combat_target_max_hp: u32,
    combat_history: Vec<String>,
    quests: Vec<QuestReply>,
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
    pub fn new(address: String, rx: Receiver<String>, tx_out: Sender<String>) -> TapClient {
        TapClient {
            server_address: address,
            connected: false,
            disconnected: false,
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
            group_invite: None,
            in_combat: false,
            combat_target: None,
            combat_target_hp: 0,
            combat_target_max_hp: 0,
            combat_history: Vec::new(),
            quests: Vec::new(),
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

    fn send_command(&mut self, cmd: String) {
        let log_line = format!(">> {cmd}");
        if self.tx_out.send(cmd.clone()).is_err() {
            self.logs.push("(not connected)".to_string());
            return;
        }
        self.pending_cmds.push_back(cmd);
        self.logs.push(log_line);
    }

    fn connect_panel(&mut self, ui: &mut egui::Ui) {
        let mut pending: Option<String> = None;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(40.0);
            ui.heading(egui::RichText::new("TAP").size(28.0));
            ui.add_space(8.0);
            ui.label(egui::RichText::new(format!("Server: {}", self.server_address)).size(18.0));
            ui.add_space(16.0);
            ui.label("Username:");
            ui.text_edit_singleline(&mut self.username);
            ui.add_space(8.0);

            let name_ok = !self.username.trim().is_empty() && !self.username.contains(' ');
            if ui
                .add_enabled(name_ok, egui::Button::new("Connect"))
                .clicked()
            {
                pending = Some(format!("CONNECT {}", self.username));
            }
            if self.username.contains(' ') {
                ui.label("(no spaces allowed in username)");
            }
        });

        if let Some(cmd) = pending {
            self.send_command(cmd);
        }
    }

    fn top_panel(&mut self, ui: &mut egui::Ui) {
        let mut pending: Option<String> = None;

        egui::Panel::top("main_panel").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new(&self.username).size(21.0));
                let max_hp = self.max_hp.max(1);
                ui.add(
                    egui::ProgressBar::new(self.hp as f32 / max_hp as f32)
                        .desired_width(220.0)
                        .fill(egui::Color32::from_rgb(60, 140, 60))
                        .text(format!("{} / {} HP", self.hp, self.max_hp)),
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
                } else if let Some(leader) = self.group_invite.clone() {
                    ui.label(format!("{leader} invited you to their group"));
                    if ui.button("Join").clicked() {
                        pending = Some(format!("GROUP JOIN {leader}"));
                    }
                    if ui.button("Dismiss").clicked() {
                        self.group_invite = None;
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
                if self.in_combat {
                    ui.add_space(16.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("COMBAT").size(26.0).strong());
                    });
                    ui.add_space(16.0);

                    let target = self.combat_target.as_deref().unwrap_or("Unknown");
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(prettify(target)).size(20.0).strong());
                    });
                    ui.add_space(6.0);
                    let max_hp = self.combat_target_max_hp.max(1);
                    ui.add(
                        egui::ProgressBar::new(self.combat_target_hp as f32 / max_hp as f32)
                            .fill(egui::Color32::from_rgb(170, 55, 55))
                            .text(format!(
                                "{} / {} HP",
                                self.combat_target_hp, self.combat_target_max_hp
                            )),
                    );

                    ui.add_space(14.0);
                    ui.separator();
                    ui.add_space(12.0);

                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui| {
                            for action in ["ATTACK", "DEFEND", "FLEE"] {
                                if ui
                                    .add_sized(
                                        [82.0, 34.0],
                                        egui::Button::new(egui::RichText::new(action).size(16.0)),
                                    )
                                    .clicked()
                                {
                                    pending = Some(action.to_string());
                                }
                                ui.add_space(6.0);
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .max_height(300.0)
                        .show(ui, |ui| {
                            for line in &self.combat_history {
                                ui.label(line);
                                ui.add_space(4.0);
                            }
                        });
                    ui.add_space(8.0);
                } else if let Some(room) = &self.room {
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
                        if *player == self.username {
                            ui.label(player.to_owned() + " (You)");
                            continue;
                        }
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
                    ui.label(egui::RichText::new("Connecting...").size(20.0));
                }

                ui.separator();

                ui.label(egui::RichText::new("Inventory: ").size(20.0));
                for item in &self.inventory {
                    ui.horizontal(|ui| {
                        ui.label(prettify(item));
                        if ui
                            .add_enabled(!self.in_combat, egui::Button::new("Drop"))
                            .clicked()
                        {
                            pending = Some(format!("DROP {item}"));
                        }
                    });
                }

                ui.separator();

                ui.label(egui::RichText::new("Quests: ").size(20.0));
                ui.add_space(4.0);
                if self.quests.is_empty() {
                    ui.label(egui::RichText::new("No active quest").weak());
                }
                for quest in &self.quests {
                    let title = quest
                        .name
                        .clone()
                        .unwrap_or_else(|| prettify(&quest.quest_id));
                    ui.label(
                        egui::RichText::new(format!("{} ({})", title, quest.progress)).strong(),
                    );
                    if let Some(desc) = &quest.description {
                        ui.label(egui::RichText::new(desc).weak());
                    }
                    ui.add_space(6.0);
                }
            });
        if let Some(cmd) = pending {
            self.send_command(cmd);
        }
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("log_list").show(ui, |ui| {
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
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
                let scope = match self.active_tab {
                    ChatTab::Global => "GLOBAL",
                    ChatTab::Room => "ROOM",
                    ChatTab::Group => "GROUP",
                };
                if !self.chat_input.trim().is_empty() {
                    pending = Some(format!("CHAT {scope} {}", self.chat_input));
                }
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
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
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
                    self.logs.push(format!("Group invite from {leader}"));
                    self.group_invite = Some(leader);
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
                    let mut cmd_splitter = cmd.splitn(3, ' ');
                    let first_word = cmd_splitter.next().unwrap_or("");

                    match first_word {
                        "CONNECT" => {
                            if data == "connected" {
                                self.connected = true;
                                self.send_command("LOOK".to_string());
                                self.send_command("INVENTORY".to_string());
                                self.send_command("STATUS".to_string());
                            } else {
                                self.logs
                                    .push(format!("Unexpected reply to CONNECT: {data}"));
                            }
                        }

                        "MOVE" => {
                            let mut data_splitter = data.splitn(3, '=');

                            match (
                                data_splitter.next(),
                                data_splitter.next(),
                                data_splitter.next(),
                            ) {
                                (Some("room"), Some(_room_name), None) => {
                                    self.send_command("LOOK".to_string())
                                }
                                _ => self.logs.push(format!("Unexpected reply to MOVE: {data}")),
                            }
                        }

                        "TAKE" => {
                            let mut data_splitter = data.splitn(3, '=');

                            match (
                                data_splitter.next(),
                                data_splitter.next(),
                                data_splitter.next(),
                            ) {
                                (Some("taken"), Some(item_name), None) => {
                                    self.items.retain(|x| x != item_name);
                                    self.inventory.push(item_name.to_string());
                                }
                                _ => self.logs.push(format!("Unexpected reply to TAKE: {data}")),
                            }
                        }

                        "DROP" => {
                            let mut data_splitter = data.splitn(3, '=');

                            match (
                                data_splitter.next(),
                                data_splitter.next(),
                                data_splitter.next(),
                            ) {
                                (Some("dropped"), Some(item_name), None) => {
                                    self.inventory.retain(|x| x != item_name);
                                    self.items.push(item_name.to_string());
                                }
                                _ => self.logs.push(format!("Unexpected reply to DROP: {data}")),
                            }
                        }

                        "GROUP" => {
                            let second_word = cmd_splitter.next().unwrap_or("");

                            match second_word {
                                "JOIN" | "CREATE" => {
                                    let mut data_splitter = data.splitn(3, '=');

                                    match (
                                        data_splitter.next(),
                                        data_splitter.next(),
                                        data_splitter.next(),
                                    ) {
                                        (Some("group"), Some(group_name), None) => {
                                            self.group = Some(group_name.to_string());
                                            self.group_invite = None
                                        }
                                        _ => self
                                            .logs
                                            .push(format!("Unexpected reply to GROUP: {data}")),
                                    }
                                }
                                "LEAVE" => {
                                    if !(data.is_empty()) {
                                        self.logs.push(format!(
                                            "Unexpected reply to GROUP LEAVE: {data}"
                                        ));
                                    } else {
                                        self.group = None
                                    }
                                }
                                _ => self.logs.push(format!("Unexpected reply to GROUP: {data}")),
                            }
                        }

                        "CHAT" => {
                            if !(data.is_empty()) {
                                self.logs.push(format!("Unexpected reply to CHAT: {data}"))
                            }
                        }

                        "STATUS" => match serde_json::from_str::<StatusReply>(&data) {
                            Ok(s) => {
                                self.hp = s.hp;
                                self.max_hp = s.max_hp;
                            }
                            Err(_) => self
                                .logs
                                .push(format!("Unexpected reply to STATUS: {data}")),
                        },

                        "LOOK" => match serde_json::from_str::<LookReply>(&data) {
                            Ok(s) => {
                                self.room = Some(s.room);
                                self.players = s.players;
                                self.items = s.items;
                                self.npcs = s.npcs;
                            }
                            Err(_) => self.logs.push(format!("Unexpected reply to LOOK: {data}")),
                        },

                        "WHO" => match serde_json::from_str::<WhoReply>(&data) {
                            Ok(s) => {
                                self.players = s.room;
                                self.server_players = s.server;
                            }
                            Err(_) => self.logs.push(format!("Unexpected reply to WHO: {data}")),
                        },

                        "TALK" => match serde_json::from_str::<TalkReply>(&data) {
                            Ok(s) => self.chat_room.push(format!(
                                "[NPC DIALOGUE] {}: {}",
                                prettify(&s.npc),
                                s.dialogue
                            )),
                            Err(_) => self.logs.push(format!("Unexpected reply to TALK: {data}")),
                        },

                        "INVENTORY" => match serde_json::from_str::<Vec<String>>(&data) {
                            Ok(s) => self.inventory = s,
                            Err(_) => self
                                .logs
                                .push(format!("Unexpected reply to INVENTORY: {data}")),
                        },

                        w @ ("ATTACK" | "DEFEND" | "FLEE") => {
                            match serde_json::from_str::<CombatReply>(&data) {
                                Ok(s) => {
                                    match cmd_splitter.next() {
                                        Some(ennemy_name) => {
                                            if self.in_combat {
                                                self.logs.push(format!("Unexpected reply to {w}: {data} (already in a fight)"))
                                            } else {
                                                self.in_combat = true;
                                                self.combat_target = Some(ennemy_name.to_string());
                                                self.hp = s.attacker_hp;
                                                self.combat_target_hp = s.target_hp;
                                                self.combat_target_max_hp = s.target_hp;
                                                if let Some(combat_log) = s.message {
                                                    self.combat_history.push(combat_log);
                                                }
                                            }
                                        }
                                        None => {
                                            if !self.in_combat {
                                                self.logs.push(format!("Unexpected reply to {w}: {data} (not in a fight)"))
                                            } else {
                                                match s.status.as_str() {
                                                    "in_combat" => {
                                                        self.hp = s.attacker_hp;
                                                        self.combat_target_hp = s.target_hp;
                                                        self.combat_history.push(format!("Inflicted {} to {}, counter attacked for {}", s.damage_dealt, self.combat_target.as_deref().unwrap_or("Unknown"), s.damage_taken))
                                                    }
                                                    "won" | "fled" | "dead" => {
                                                        self.in_combat = false;
                                                        self.combat_target = None;
                                                        self.combat_target_hp = 0;
                                                        self.combat_target_max_hp = 0;
                                                        self.combat_history.push(
                                                            s.message.unwrap_or_else(|| {
                                                                "Combat ended".to_string()
                                                            }),
                                                        );
                                                        self.send_command("LOOK".to_string());
                                                        self.send_command("STATUS".to_string());
                                                    }
                                                    _ => self.logs.push(format!(
                                                        "Unexpected reply to {w}: {data}"
                                                    )),
                                                }
                                            }
                                        }
                                    }
                                }

                                Err(_) => {
                                    self.logs.push(format!("Unexpected reply to {w}: {data}"))
                                }
                            }
                        }

                        "QUESTS" => match serde_json::from_str::<Vec<QuestReply>>(&data) {
                            Ok(s) => self.quests = s,
                            Err(_) => self
                                .logs
                                .push(format!("Unexpected reply to QUESTS: {data}")),
                        },

                        "QUEST" => match cmd_splitter.next() {
                            Some(_) => match serde_json::from_str::<QuestReply>(&data) {
                                Ok(s) => {
                                    if s.status == "completed" {
                                        self.send_command("INVENTORY".to_string());
                                    }
                                    if let Some(index) =
                                        self.quests.iter().position(|q| q.quest_id == s.quest_id)
                                    {
                                        self.quests[index] = s;
                                    } else {
                                        self.quests.push(s);
                                    }
                                }
                                Err(_) => {
                                    self.logs.push(format!("Unexpected reply to QUEST: {data}"))
                                }
                            },
                            None => self.logs.push(format!("Unexpected reply to QUEST: {data}")),
                        },

                        "QUIT" => {
                            if data != "bye" {
                                self.logs.push(format!("Unexpected reply to QUIT: {data}"))
                            } else {
                                self.logs.push("Quitting".to_string())
                            }
                        }

                        _ => self.logs.push(format!("OK to '{cmd}': {data}")),
                    }
                    if first_word != "QUESTS" && !self.in_combat {
                        self.send_command("QUESTS".to_string());
                    }
                }

                None => {
                    if data.starts_with("hello proto=") {
                        self.logs.push("Server's greeting".to_string())
                    } else {
                        self.logs.push(format!("Unsolicited OK: {data}"))
                    }
                }
            },

            ServerMsg::Err(code, description) => {
                if code == 900 {
                    self.disconnected = true;
                    self.logs.push(format!("Disconnected: {description}"));
                } else {
                    match self.pending_cmds.pop_front() {
                        Some(cmd) => {
                            if code == 900 {
                                self.disconnected = true;
                                self.logs.push(format!("Disconnected: {description}"));
                            } else {
                                self.logs
                                    .push(format!("ERR to '{cmd}': {code} {description}"));
                            }
                        }
                        None => self
                            .logs
                            .push(format!("Unsolicited ERR: {code} {description}")),
                    }
                }
            }
            ServerMsg::Unknown(data) => self.logs.push(format!("(unparsed) {data}")),
        }
    }
}

impl eframe::App for TapClient {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(line) = self.rx.try_recv() {
            self.logs.push(format!("<< {line}"));
            self.apply(crate::protocol::parse_line(&line));
        }
        if self.disconnected {
            self.right_panel(ui);
            egui::CentralPanel::default().show(ui, |ui| {
                ui.add_space(40.0);
                ui.heading("You've been disconnected");
            });
        } else if self.connected {
            self.top_panel(ui);
            self.left_panel(ui);
            self.right_panel(ui);
            self.bottom_panel(ui);
            self.central_panel(ui);
        } else {
            self.right_panel(ui);
            self.connect_panel(ui);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::parse_line;
    use std::sync::mpsc::{channel, Receiver};

    fn client() -> (TapClient, Receiver<String>) {
        let (_tx, rx) = channel();
        let (tx_out, out) = channel();
        (TapClient::new("test".to_string(), rx, tx_out), out)
    }

    fn feed(c: &mut TapClient, sent: &str, reply: &str) {
        c.pending_cmds.push_back(sent.to_string());
        c.apply(parse_line(reply));
    }

    fn last_log(c: &TapClient) -> &str {
        c.logs.last().map(|s| s.as_str()).unwrap_or("")
    }

    #[test]
    fn connect_ok_sets_connected_and_sends_initial_salvo() {
        let (mut c, out) = client();
        feed(&mut c, "CONNECT test", "OK connected");
        assert!(c.connected);
        assert_eq!(out.try_recv().unwrap(), "LOOK");
        assert_eq!(out.try_recv().unwrap(), "INVENTORY");
        assert_eq!(out.try_recv().unwrap(), "STATUS");
        assert!(out.try_recv().is_err());
        assert_eq!(c.pending_cmds.len(), 3);
    }

    #[test]
    fn connect_unexpected_reply_is_logged() {
        let (mut c, _out) = client();
        feed(&mut c, "CONNECT test", "OK weird");
        assert!(!c.connected);
        assert!(last_log(&c).contains("CONNECT"));
    }

    #[test]
    fn status_ok_updates_hp() {
        let (mut c, _out) = client();
        feed(
            &mut c,
            "STATUS",
            r#"OK {"hp": 55, "max_hp": 120, "status": "healthy"}"#,
        );
        assert_eq!(c.hp, 55);
        assert_eq!(c.max_hp, 120);
    }

    #[test]
    fn status_malformed_json_is_logged_and_state_untouched() {
        let (mut c, _out) = client();
        feed(&mut c, "STATUS", r#"OK {"hp": "abc"}"#);
        assert_eq!(c.hp, 100);
        assert!(last_log(&c).contains("STATUS"));
    }

    #[test]
    fn look_ok_fills_room_and_lists() {
        let (mut c, _out) = client();
        feed(
            &mut c,
            "LOOK",
            r#"OK {"room": {"id": "loc.tavern", "name": "The Rusty Tankard", "description": "Smoky.", "exits": {"south": "loc.square", "up": "loc.attic"}}, "players": ["alice", "bob"], "items": ["item.ale"], "npcs": ["npc.barkeep"]}"#,
        );
        let room = c.room.as_ref().expect("room should be set");
        assert_eq!(room.name, "The Rusty Tankard");
        assert_eq!(room.exits.len(), 2);
        assert_eq!(room.exits.get("up").unwrap(), "loc.attic");
        assert_eq!(c.players, vec!["alice", "bob"]);
        assert_eq!(c.items, vec!["item.ale"]);
        assert_eq!(c.npcs, vec!["npc.barkeep"]);
    }

    #[test]
    fn look_replaces_previous_state_not_appends() {
        let (mut c, _out) = client();
        c.players = vec!["ghost".to_string()];
        c.items = vec!["item.old".to_string()];
        feed(
            &mut c,
            "LOOK",
            r#"OK {"room": {"id": "loc.a", "name": "A", "description": "d", "exits": {}}, "players": [], "items": [], "npcs": []}"#,
        );
        assert!(c.players.is_empty());
        assert!(c.items.is_empty());
    }

    #[test]
    fn look_missing_key_is_logged_room_stays_none() {
        let (mut c, _out) = client();
        feed(
            &mut c,
            "LOOK",
            r#"OK {"players": [], "items": [], "npcs": []}"#,
        );
        assert!(c.room.is_none());
        assert!(last_log(&c).contains("LOOK"));
    }

    #[test]
    fn inventory_ok_replaces_inventory() {
        let (mut c, _out) = client();
        c.inventory = vec!["item.old".to_string()];
        feed(&mut c, "INVENTORY", r#"OK ["item.herbs", "item.bread"]"#);
        assert_eq!(c.inventory, vec!["item.herbs", "item.bread"]);
    }

    #[test]
    fn inventory_empty_list_is_valid() {
        let (mut c, _out) = client();
        c.inventory = vec!["item.old".to_string()];
        feed(&mut c, "INVENTORY", "OK []");
        assert!(c.inventory.is_empty());
    }

    #[test]
    fn who_ok_updates_server_players_and_room_list() {
        let (mut c, _out) = client();
        feed(
            &mut c,
            "WHO",
            r#"OK {"room": ["alice", "bob"], "server": 7}"#,
        );
        assert_eq!(c.server_players, 7);
        assert_eq!(c.players, vec!["alice", "bob"]);
    }

    #[test]
    fn talk_ok_displays_dialogue_in_room_chat() {
        let (mut c, _out) = client();
        feed(
            &mut c,
            "TALK npc.guard",
            r#"OK {"npc": "guard", "dialogue": "Stay safe, traveler."}"#,
        );
        let line = c.chat_room.last().expect("dialogue should be in room chat");
        assert!(line.contains("Stay safe, traveler."));
        assert!(line.contains("guard"));
    }

    #[test]
    fn move_ok_triggers_look() {
        let (mut c, out) = client();
        feed(&mut c, "MOVE north", "OK room=loc.tavern");
        assert_eq!(out.try_recv().unwrap(), "LOOK");
        assert_eq!(c.pending_cmds.len(), 1);
    }

    #[test]
    fn move_unexpected_reply_is_logged() {
        let (mut c, out) = client();
        feed(&mut c, "MOVE north", "OK nonsense");
        assert!(out.try_recv().is_err());
        assert!(last_log(&c).contains("MOVE"));
    }

    #[test]
    fn take_ok_moves_item_from_room_to_inventory() {
        let (mut c, _out) = client();
        c.items = vec!["item.herbs".to_string(), "item.rock".to_string()];
        feed(&mut c, "TAKE item.herbs", "OK taken=item.herbs");
        assert_eq!(c.items, vec!["item.rock"]);
        assert_eq!(c.inventory, vec!["item.herbs"]);
    }

    #[test]
    fn take_by_display_name_uses_canonical_id_from_reply() {
        let (mut c, _out) = client();
        c.items = vec!["item.herbs".to_string()];
        feed(&mut c, "TAKE Herbs", "OK taken=item.herbs");
        assert!(c.items.is_empty());
        assert_eq!(c.inventory, vec!["item.herbs"]);
    }

    #[test]
    fn drop_ok_moves_item_from_inventory_to_room() {
        let (mut c, _out) = client();
        c.inventory = vec!["item.bread".to_string()];
        feed(&mut c, "DROP item.bread", "OK dropped=item.bread");
        assert!(c.inventory.is_empty());
        assert_eq!(c.items, vec!["item.bread"]);
    }

    #[test]
    fn group_create_ok_sets_group() {
        let (mut c, _out) = client();
        feed(&mut c, "GROUP CREATE", "OK group=grp_123");
        assert_eq!(c.group.as_deref(), Some("grp_123"));
    }

    #[test]
    fn group_join_ok_sets_group() {
        let (mut c, _out) = client();
        feed(&mut c, "GROUP JOIN alice", "OK group=grp_777");
        assert_eq!(c.group.as_deref(), Some("grp_777"));
    }

    #[test]
    fn group_leave_ok_clears_group() {
        let (mut c, _out) = client();
        c.group = Some("grp_123".to_string());
        feed(&mut c, "GROUP LEAVE", "OK");
        assert!(c.group.is_none());
    }

    #[test]
    fn err_dequeues_like_ok_keeping_order() {
        let (mut c, _out) = client();
        c.pending_cmds.push_back("STATUS".to_string());
        c.pending_cmds.push_back("INVENTORY".to_string());
        c.apply(parse_line("ERR 500 SERVER_ERROR"));
        assert!(last_log(&c).contains("STATUS"));
        c.apply(parse_line(r#"OK ["item.x"]"#));
        assert_eq!(c.inventory, vec!["item.x"]);
        assert!(c.pending_cmds.is_empty());
    }

    #[test]
    fn unsolicited_ok_is_logged_not_crashing() {
        let (mut c, _out) = client();
        c.apply(parse_line("OK hello proto=1"));
        assert!(last_log(&c).to_lowercase().contains("unsolicited"));
        assert!(c.pending_cmds.is_empty());
    }

    #[test]
    fn unsolicited_err_is_logged() {
        let (mut c, _out) = client();
        c.apply(parse_line("ERR 500 SERVER_ERROR"));
        assert!(last_log(&c).to_lowercase().contains("unsolicited"));
    }

    #[test]
    fn evt_enter_and_leave_update_players() {
        let (mut c, _out) = client();
        c.apply(parse_line("EVT ROOM PRESENCE ENTER carol"));
        assert_eq!(c.players, vec!["carol"]);
        c.apply(parse_line("EVT ROOM PRESENCE LEAVE carol"));
        assert!(c.players.is_empty());
        assert!(c.pending_cmds.is_empty());
    }

    #[test]
    fn evt_chat_goes_to_the_right_tab() {
        let (mut c, _out) = client();
        c.apply(parse_line("EVT ROOM CHAT alice salut tout le monde"));
        c.apply(parse_line("EVT GLOBAL CHAT bob hey"));
        c.apply(parse_line("EVT GROUP CHAT carol go"));
        assert!(c.chat_room.last().unwrap().contains("salut tout le monde"));
        assert!(c.chat_global.last().unwrap().contains("hey"));
        assert!(c.chat_group.last().unwrap().contains("go"));
    }

    #[test]
    fn evt_stats_updates_server_players() {
        let (mut c, _out) = client();
        c.apply(parse_line("EVT STATS players=12"));
        assert_eq!(c.server_players, 12);
    }

    #[test]
    fn evt_group_join_leave_are_visible_in_group_chat() {
        let (mut c, _out) = client();
        c.apply(parse_line("EVT GROUP JOIN dave"));
        assert!(c.chat_group.last().unwrap().contains("dave"));
        c.apply(parse_line("EVT GROUP LEAVE dave"));
        assert!(c.chat_group.last().unwrap().contains("dave"));
    }

    #[test]
    fn unknown_line_is_logged_and_harmless() {
        let (mut c, _out) = client();
        c.apply(parse_line("GARBAGE stuff"));
        assert!(last_log(&c).contains("GARBAGE stuff"));
        assert!(c.pending_cmds.is_empty());
        assert!(!c.connected);
    }
}
