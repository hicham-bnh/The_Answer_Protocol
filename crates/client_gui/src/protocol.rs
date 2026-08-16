use crate::state::{ChatTab, Room};
use serde::Deserialize;

#[derive(Debug)]
pub enum EvtType {
    Enter(String),
    Leave(String),
    Chat(ChatTab, String, String),
    GroupInvite(String),
    GroupJoin(String),
    GroupLeave(String),
    PlayerCount(u32),
}

#[derive(Debug)]
pub enum ServerMsg {
    Ok(String),
    Err(u16, String),
    Evt(EvtType),
    Unknown(String),
}

pub fn parse_line(line: &str) -> ServerMsg {
    let mut parts = line.splitn(2, ' ');

    match parts.next() {
        Some("OK") => ServerMsg::Ok(parts.next().unwrap_or("").to_string()),

        Some("ERR") => match parts.next() {
            Some(text) => {
                let mut err_parts = text.splitn(2, ' ');
                let (code, msg) = (err_parts.next(), err_parts.next());
                match (code, msg) {
                    (Some(code), Some(msg)) => match code.parse::<u16>() {
                        Ok(n) => ServerMsg::Err(n, msg.to_string()),
                        Err(_) => ServerMsg::Unknown(format!("Malformed ERR: {line}")),
                    },
                    _ => ServerMsg::Unknown(format!("Malformed ERR: {line}")),
                }
            }
            None => ServerMsg::Unknown(format!("Malformed ERR: {line}")),
        },

        Some("EVT") => {
            let mut evt = parts.next().unwrap_or("").splitn(4, ' ');
            match (evt.next(), evt.next(), evt.next(), evt.next()) {
                (Some("ROOM"), Some("PRESENCE"), Some("ENTER"), Some(username)) => {
                    ServerMsg::Evt(EvtType::Enter(username.to_string()))
                }
                (Some("ROOM"), Some("PRESENCE"), Some("LEAVE"), Some(username)) => {
                    ServerMsg::Evt(EvtType::Leave(username.to_string()))
                }
                (Some(chat_tab), Some("CHAT"), Some(username), Some(msg)) => match chat_tab {
                    "ROOM" => ServerMsg::Evt(EvtType::Chat(
                        ChatTab::Room,
                        username.to_string(),
                        msg.to_string(),
                    )),
                    "GLOBAL" => ServerMsg::Evt(EvtType::Chat(
                        ChatTab::Global,
                        username.to_string(),
                        msg.to_string(),
                    )),
                    "GROUP" => ServerMsg::Evt(EvtType::Chat(
                        ChatTab::Group,
                        username.to_string(),
                        msg.to_string(),
                    )),
                    _ => ServerMsg::Unknown(format!("Malformed EVT: {line}")),
                },
                (Some("GROUP"), Some("INVITE"), Some(username), None) => {
                    ServerMsg::Evt(EvtType::GroupInvite(username.to_string()))
                }
                (Some("GROUP"), Some("JOIN"), Some(username), None) => {
                    ServerMsg::Evt(EvtType::GroupJoin(username.to_string()))
                }
                (Some("GROUP"), Some("LEAVE"), Some(username), None) => {
                    ServerMsg::Evt(EvtType::GroupLeave(username.to_string()))
                }
                (Some("STATS"), Some(data), None, None) => {
                    let mut stats = data.splitn(2, '=');

                    match (
                        stats.next(),
                        stats.next().and_then(|c| c.parse::<u32>().ok()),
                    ) {
                        (Some("players"), Some(player_count)) => {
                            ServerMsg::Evt(EvtType::PlayerCount(player_count))
                        }
                        _ => ServerMsg::Unknown(format!("Malformed EVT: {line}")),
                    }
                }
                _ => ServerMsg::Unknown(format!("Malformed EVT: {line}")),
            }
        }

        Some(_) => ServerMsg::Unknown(format!("Malformed message: {line}")),

        None => ServerMsg::Unknown("No message".to_string()),
    }
}

#[derive(Debug, Deserialize)]
pub struct StatusReply {
    pub hp: u32,
    pub max_hp: u32,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct LookReply {
    pub room: Room,
    pub players: Vec<String>,
    pub items: Vec<String>,
    pub npcs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct TalkReply {
    pub npc: String,
    pub dialogue: String,
}

#[derive(Debug, Deserialize)]
pub struct WhoReply {
    pub room: Vec<String>,
    pub server: u32,
}
