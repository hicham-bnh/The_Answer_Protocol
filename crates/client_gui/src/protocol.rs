use crate::state::{ChatTab, Room};
use serde::Deserialize;

#[derive(Debug, PartialEq)]
pub enum EvtType {
    Enter(String),
    Leave(String),
    Chat(ChatTab, String, String),
    GroupInvite(String),
    GroupJoin(String),
    GroupLeave(String),
    PlayerCount(u32),
}

#[derive(Debug, PartialEq)]
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

#[derive(Debug, Deserialize)]
pub struct CombatReply {
    pub action: String,
    pub attacker_hp: u32,
    pub target_hp: u32,
    pub damage_dealt: u32,
    pub damage_taken: u32,
    pub status: String,
    pub message: Option<String>
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ok() {
        assert_eq!(parse_line("OK connected"), ServerMsg::Ok("connected".to_string()));
        assert_eq!(parse_line("OK"), ServerMsg::Ok("".to_string()));
        assert_eq!(parse_line("OK hello proto=1"), ServerMsg::Ok("hello proto=1".to_string()));
        assert_eq!(
            parse_line(r#"OK { "room": { }, "players":["alice"] }"#),
            ServerMsg::Ok(r#"{ "room": { }, "players":["alice"] }"#.to_string())
        );
    }

    #[test]
    fn parse_err() {
        assert_eq!(parse_line("ERR 404 ITEM_NOT_FOUND"), ServerMsg::Err(404, "ITEM_NOT_FOUND".to_string()));
        assert_eq!(parse_line("ERR 201 NAME_IN_USE"), ServerMsg::Err(201, "NAME_IN_USE".to_string()));
        assert_eq!(parse_line("ERR abc xyz"), ServerMsg::Unknown("Malformed ERR: ERR abc xyz".to_string()));
        assert_eq!(parse_line("ERR"), ServerMsg::Unknown("Malformed ERR: ERR".to_string()));
        assert_eq!(parse_line("ERR 404"), ServerMsg::Unknown("Malformed ERR: ERR 404".to_string()));
    }

    #[test]
    fn parse_presence() {
        assert_eq!(
            parse_line("EVT ROOM PRESENCE ENTER alice"),
            ServerMsg::Evt(EvtType::Enter("alice".to_string()))
        );
        assert_eq!(
            parse_line("EVT ROOM PRESENCE LEAVE bob"),
            ServerMsg::Evt(EvtType::Leave("bob".to_string()))
        );
    }

    #[test]
    fn parse_chat() {
        assert_eq!(
            parse_line("EVT ROOM CHAT alice salut tout le monde"),
            ServerMsg::Evt(EvtType::Chat(ChatTab::Room, "alice".to_string(), "salut tout le monde".to_string()))
        );
        assert_eq!(
            parse_line("EVT GLOBAL CHAT bob hey!"),
            ServerMsg::Evt(EvtType::Chat(ChatTab::Global, "bob".to_string(), "hey!".to_string()))
        );
        assert_eq!(
            parse_line("EVT GROUP CHAT carol on y va"),
            ServerMsg::Evt(EvtType::Chat(ChatTab::Group, "carol".to_string(), "on y va".to_string()))
        );
    }

    #[test]
    fn parse_group() {
        assert_eq!(parse_line("EVT GROUP INVITE alice"), ServerMsg::Evt(EvtType::GroupInvite("alice".to_string())));
        assert_eq!(parse_line("EVT GROUP JOIN bob"), ServerMsg::Evt(EvtType::GroupJoin("bob".to_string())));
        assert_eq!(parse_line("EVT GROUP LEAVE carol"), ServerMsg::Evt(EvtType::GroupLeave("carol".to_string())));
        assert_eq!(
            parse_line("EVT GROUP INVITE alice bob"),
            ServerMsg::Unknown("Malformed EVT: EVT GROUP INVITE alice bob".to_string())
        );
    }

    #[test]
    fn parse_stats() {
        assert_eq!(parse_line("EVT STATS players=5"), ServerMsg::Evt(EvtType::PlayerCount(5)));
        assert_eq!(
            parse_line("EVT STATS players=abc"),
            ServerMsg::Unknown("Malformed EVT: EVT STATS players=abc".to_string())
        );
    }

    #[test]
    fn parse_garbage() {
        assert_eq!(parse_line("GARBAGE stuff"), ServerMsg::Unknown("Malformed message: GARBAGE stuff".to_string()));
        assert_eq!(parse_line(""), ServerMsg::Unknown("Malformed message: ".to_string()));
    }
}