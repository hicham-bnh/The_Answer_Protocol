use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::TcpStream;
use std::io::{Write, BufReader, BufRead};
use crate::Player;
use std::thread;
use std::time::Duration;

use crate::world_struct::world_struct::GameWorld;

pub fn connect_user(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>, 
    spawn_room: String
    ) -> (bool, String){
    let mut args = line.splitn(2, ' ');
    let mut name = String::new();
    let mut guard = players.lock().unwrap();
    match args.next(){
        Some("CONNECT") => {
            if let Some(arg_name) = args.next() {
                name.push_str(arg_name);
                if guard.contains_key(&name) {
                    stream.write_all(b"ERR 201 NAME_IN_USE\n").expect("write failed");
                    return (false, String::new());
                }
                else {
                    let stream_for_map = stream.try_clone().expect("clone failed");
                    let player = Player {
                        stream: Mutex::new(stream_for_map),
                        room: spawn_room.to_string(),
                        name: name.to_string(),
                        inventory: vec![],
                        hp: 100,
                        pv: 15,
                        combat: "not in combat".to_string()
                    };
                    guard.insert(name.clone(), player);
                    stream.write_all(b"OK connected\n").expect("write failed");
                    return (true, name);
                }
            }
            else {
                stream.write_all(b"please connect with your username\n").expect("Failder to write reponse");
                return (false, String::new());
            }
        }
        Some("QUIT") => {
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
            println!("USER QUIT");
            return (false, String::new());
        }
        Some(cmd_inconnue) => {
            stream.write_all(b"please connect\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
            return (false, String::new());
        }
        _none => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
            return (false, String::new());
        }
    }
}



pub fn parse_command(
    line: &str,
    stream: &mut TcpStream,
    players: &Arc<Mutex<HashMap<String, Player>>>,
    name: &str,
    world: &Arc<Mutex<GameWorld>>
    ) {
    let mut args = line.splitn(2, ' ');
    match args.next(){
        Some("CONNECT") => {
            stream.write_all(b"you are connect yet\n").expect("Failder to write reponse");
        }
        Some("LOOK") => {
            let mut guard = players.lock().unwrap();
            let mut room_str = String::new();
            let mut players: Vec<String> = Vec::new();
            if let Some(player) = guard.get(name) {
                room_str = player.room.clone();
            }
            for (_client_name, client_steam) in guard.iter_mut(){
                 if client_steam.room.to_string() == room_str {
                     players.push(_client_name.to_string());
                 }
             }
             if let Some(player) = guard.get(name) {
                let w = world.lock().unwrap();
                let items = &w.world.locations.get(&room_str).unwrap().items;
                let npcs = &w.world.locations.get(&room_str).unwrap().npcs;
                let msg = format!(
                    "OK {{ \"room\": {}, \"players\": {:?}, \"items\": {:?}, , \"npcs\": {:?} }}\n",
                    room_str, players, items, npcs
                );
                let mut guard_stream = player.stream.lock().unwrap();
                guard_stream.write_all(msg.as_bytes()).expect("Failder to write reponse");
            }
            println!("USER USE LOOK");
        }
        Some("MOVE") => {
            let Some(rest) = args.next() else {
                    stream.write_all(b"ERR usage: MOVE <direction>\n").ok();
                    println!("USER USE MOVE (missing arg)");
                    return;
                };
            let mut chat_args = rest.splitn(2, ' ');
            let move_to = chat_args.next().unwrap_or("");
            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let room_str = player.room.clone();
                let w = world.lock().unwrap();
                let exists = &w.world.locations.get(&room_str).unwrap().exits;
                if let Some(room_to) = exists.get(move_to) {
                    let room_to = room_to.clone();
                    let msg = format!("OK room={}\n", room_to);
                    stream.write_all(msg.as_bytes()).expect("Failder to write reponse");
                    player.room = room_to.clone();
                    for (_client_name, client_steam) in guard.iter_mut(){
                         if client_steam.room.to_string() == room_str {
                             let msg_evnt_leave = format!("EVT ROOM PRESENCE LEAVE {}\n", name);
                             client_steam.stream.lock().unwrap().write_all(msg_evnt_leave.as_bytes()).expect("ERROR");
                        }
                        else if client_steam.room.to_string() == room_to {
                            let msg_evnt_enter = format!("EVT ROOM PRESENCE ENTER {}\n", name);
                            client_steam.stream.lock().unwrap().write_all(msg_evnt_enter.as_bytes()).expect("ERROR");
                        }
                     }
                }
                else {
                    stream.write_all(b"ERR 301 NO_EXIT\n").ok();
                    return;
                }
            }
            println!("USER USE MOVE");
        }
        Some("QUIT") => {
            println!("{} QUIT", name);
            let mut guard = players.lock().unwrap();
            guard.remove(name);
            stream.write_all(b"OK bye\n").expect("Failder to write reponse");
        }
        Some("CHAT") => {
            let mut chat_args = args.next().expect("REASON").splitn(2, ' ');
            if chat_args.next() == Some("GLOBAL"){
                //let target = sub_args.next().unwrap_or("");
                let chat_msg = chat_args.next().unwrap_or("");
                let mut guard = players.lock().unwrap();
                stream.write_all(b"OK\n").expect("Failder to write reponse");
                for (_client_name, client_steam) in guard.iter_mut()
                {
                    let msg = format!("EVT GLOBAL CHAT {} {}\n", name, chat_msg);
                    client_steam.stream.lock().unwrap().write_all(msg.as_bytes()).expect("write failed");
                }
            }
            println!("{} USE CHAT", name);
        }
        Some("WHO") => {
            let mut nmbr_players: u32 = 0;
            let mut guard = players.lock().unwrap();
            for _client_name in guard.iter_mut(){
                nmbr_players += 1;
            }
            let msg = format!("OK {{{}}}\n", nmbr_players);
            stream.write_all(msg.as_bytes()).expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("GROUP") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("TAKE") => {
            let Some(rest) = args.next() else {
                    stream.write_all(b"ERR usage: TAKE <item>\n").ok();
                    println!("USER USE TAKE (missing arg)");
                    return;
                };
            let mut chat_args = rest.splitn(2, ' ');
            let items_to_take = chat_args.next().unwrap_or("");
            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let room_player = player.room.clone();
                let mut w = world.lock().unwrap();
                let is_obtainable = w.items.get(items_to_take).map(|i| i.obtainable).unwrap_or(false);
                if is_obtainable {
                    if let Some(location) = w.world.locations.get_mut(&room_player) {
                        if let Some(index) = location.items.iter().position(|x| x == items_to_take) {
                            location.items.remove(index);
                            player.inventory.push(items_to_take.to_string());
                            let msg = format!("OK taken={}\n", items_to_take);
                            stream.write_all(msg.as_bytes()).expect("Failed to write response");
                        }
                    }
                }
            }
            println!("USER USE TAKE");
        }
        Some("DROP") => {
            let Some(rest) = args.next() else {
                    stream.write_all(b"ERR usage: TAKE <item>\n").ok();
                    println!("USER USE TAKE (missing arg)");
                    return;
                };
            let mut chat_args = rest.splitn(2, ' ');
            let items_to_drop = chat_args.next().unwrap_or("");
            let mut guard = players.lock().unwrap();
                        if let Some(player) = guard.get_mut(name) {
                            let room_player = player.room.clone();
                            let mut w = world.lock().unwrap();
                            if let Some(index) = player.inventory.iter().position(|x| x == items_to_drop) {
                                if let Some(location) = w.world.locations.get_mut(&room_player) {
                                    player.inventory.remove(index);
                                    location.items.push(items_to_drop.to_string());
                                    let msg = format!("OK dropped={}\n", items_to_drop);
                                    stream.write_all(msg.as_bytes()).expect("Failed to write response");
                                }
                            }
            }
            println!("USER USE LOOK");
        }
        Some("INVENTORY") => {
            let guard = players.lock().unwrap();
            if let Some(player) = guard.get(name) {
                let msg = format!("OK {:?}\n", player.inventory);
                stream.write_all(msg.as_bytes()).expect("ERROR");
            }
            else {
                let msg_error = format!("ERROR NO PLAYER\n");
                stream.write_all(msg_error.as_bytes()).expect("ERROR");
            }
            println!("USER USE LOOK");
        }
        Some("TALK") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("ATTACK") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("STATUS") => {
            let mut guard = players.lock().unwrap();
            if let Some(player) = guard.get_mut(name) {
                let msg = format!("OK status {{hp={}, combat={}}}\n", player.hp, player.combat);
                stream.write_all(msg.as_bytes()).expect("ERROR PRINT TATUS");
            }
            println!("USER USE STATUS");
        }
        Some("QUEST") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("QUESTS") => {
            stream.write_all(b"OK\n").expect("Failder to write reponse");
            println!("USER USE LOOK");
        }
        Some("COMBAT") => {
                    let room = {
                        let mut guard = players.lock().unwrap();
                        guard.get_mut(name).map(|p| p.room.clone())
                    };
                    if let Some(room) = room {
                        let npc_name = {
                            let w = world.lock().unwrap();
                            w.world.locations.get(&room)
                                .and_then(|loc| loc.npcs.first())
                                .cloned()
                        };
                        if let Some(npc_id) = npc_name {
                            let npc_display_name = {
                                let w = world.lock().unwrap();
                                w.npcs.get(&npc_id).map(|n| n.name.clone())
                            };
                            let npc_info = {
                                let w = world.lock().unwrap();
                                w.npcs.get(&npc_id).map(|n| (n.stats.hp, n.stats.damage, n.hostile))
                            };
                            let (_npc_hp, _npc_damage, npc_hostile) = npc_info.unwrap();
                            if npc_hostile == false {
                                let msh = format!("ERR 405 NPC_NOT_HOSTILE\n");
                                stream.write(msh.as_bytes()).expect("ERROR");
                                return ;
                            }
                            if let Some(display_name) = npc_display_name {
                                let can_engage = {
                                    let mut w = world.lock().unwrap();
                                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                                        if n.engaged_by.is_some() {
                                            false
                                        } else {
                                            n.engaged_by = Some(name.to_string());
                                            true
                                        }
                                    } else {
                                        false
                                    }
                                };
                                if !can_engage {
                                    stream.write_all(b"ERR NPC already in combat\n").expect("ERROR");
                                    return;
                                } 
                                let msg = format!("combat with {}\n", display_name);
                                stream.write_all(msg.as_bytes()).expect("ERROR");
                                let _in_combat = {
                                    let mut guard = players.lock().unwrap();
                                    guard.get_mut(name).map(|p| p.combat = "in combat".to_string())
                                };
                                let stream_clone = stream.try_clone().expect("clone failed");
                                let read_buf = BufReader::new(stream_clone);
                                for line in read_buf.lines() {
                                    let line = line.expect("erreur reading ligne");
                                    let mut args = line.splitn(2, ' ');
                                    match args.next() {
                                        Some("ATTACK") => {
                                            let pv = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.pv.clone())
                                            };
                                            let hp = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.hp.clone())
                                            };
                                            let npc_info = {
                                                let w = world.lock().unwrap();
                                                w.npcs.get(&npc_id).map(|n| (n.stats.hp, n.stats.damage, n.hostile))
                                            };
                                            let (_npc_hp, _npc_damage, npc_hostile) = npc_info.unwrap();
                                            if _npc_hp < 1 {
                                                let msg = format!("ERR 404 NPC_NOT_FOUND\n");
                                                stream.write_all(msg.as_bytes()).expect("ERROR");
                                                let mut w = world.lock().unwrap();
                                                if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                    n.engaged_by = None;
                                                }
                                                return;
                                            }
                                            let npc_info = {
                                                let mut w = world.lock().unwrap();
                                                if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                    n.stats.hp = n.stats.hp.saturating_sub(pv.unwrap_or(0));
                                                    Some((n.stats.hp, n.stats.damage, n.hostile))
                                                } else {
                                                    None
                                                }
                                            };
                                            let (npc_hp, npc_damage, _npc_hostile) = npc_info.unwrap();
                                            if npc_hp < 1 {
                                                let _won = {
                                                    let mut guard = players.lock().unwrap();
                                                    guard.get_mut(name).map(|p| p.combat = "won".to_string())
                                                };
                                                let combat_status = {
                                                    let mut guard = players.lock().unwrap();
                                                    guard.get_mut(name).map(|p| p.combat.clone()).unwrap_or("unknown".to_string())
                                                };
                                                let msg = format!(
                                                    "OK {{attacker_hp: {}, target_hp: {}, damage_dealt: {}, damage_taken: {}, status: {}}}\n",
                                                    hp.unwrap_or(0), npc_hp, pv.unwrap_or(0), npc_damage.unwrap_or(0), combat_status
                                                );
                                                stream.write_all(msg.as_bytes()).expect("ERROR");
                                                let msg_won = format!("OK you won the fight\n");
                                                stream.write_all(msg_won.as_bytes()).expect("ERROR");
                                                {
                                                    let mut w = world.lock().unwrap();
                                                    if let Some(location) = w.world.locations.get_mut(&room) {
                                                        location.npcs.retain(|id| id != &npc_id);
                                                    }
                                                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                        n.engaged_by = None;
                                                    }
                                                }
                                                let world_clone = Arc::clone(world);
                                                let npc_id_clone = npc_id.clone();
                                                let room_clone = room.clone();
                                                thread::spawn(move || {
                                                    thread::sleep(Duration::from_secs(30));
                                                    let mut w = world_clone.lock().unwrap();
                                                    if let Some(n) = w.npcs.get_mut(&npc_id_clone) {
                                                        n.stats.hp = n.stats.max_hp;
                                                        n.engaged_by = None;
                                                    }
                                                    if let Some(location) = w.world.locations.get_mut(&room_clone) {
                                                        if !location.npcs.contains(&npc_id_clone) {
                                                            location.npcs.push(npc_id_clone.clone());
                                                        }
                                                    }
                                                });
                                                break;
                                            } else if npc_hostile {
                                                let combat_status = {
                                                    let mut guard = players.lock().unwrap();
                                                    guard.get_mut(name).map(|p| p.combat.clone()).unwrap_or("unknown".to_string())
                                                };
                                                let msg = format!(
                                                    "OK {{attacker_hp: {}, target_hp: {}, damage_dealt: {}, damage_taken: {}, status: {}}}\n",
                                                    hp.unwrap_or(0), npc_hp, pv.unwrap_or(0), npc_damage.unwrap_or(0), combat_status
                                                );
                                                stream.write_all(msg.as_bytes()).expect("ERROR");
                                                let hp = {
                                                    let mut guard = players.lock().unwrap();
                                                    if let Some(p) = guard.get_mut(name) {
                                                        p.hp = p.hp.saturating_sub(npc_damage.unwrap_or(0));
                                                        Some(p.hp)
                                                    } else {
                                                        None
                                                    }
                                                };
                                                let msg_repost = format!(
                                                    "repost {{attacker_hp: {}, target_hp: {}, damage_dealt: {}, damage_taken: {}, status: {}}}\n",
                                                    hp.unwrap_or(0), npc_hp, pv.unwrap_or(0), npc_damage.unwrap_or(0), combat_status
                                                );
                                                stream.write_all(msg_repost.as_bytes()).expect("ERROR");
                                                if hp.unwrap_or(0) < 1 {
                                                    let respawn_room = {
                                                        let w = world.lock().unwrap();
                                                        w.world.respawn_location.clone()
                                                    };
                                                    {
                                                        let mut w = world.lock().unwrap();
                                                        if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                            n.engaged_by = None;
                                                        }
                                                    }
                                                    {
                                                        let mut guard = players.lock().unwrap();
                                                        if let Some(p) = guard.get_mut(name) {
                                                            p.hp = 50;
                                                            p.combat = "not in combat".to_string();
                                                            p.room = respawn_room.clone();
                                                        }
                                                    }
                                                    let msg_death = format!("OK you died, respawned in {} with 50 hp\n", respawn_room);
                                                    stream.write_all(msg_death.as_bytes()).expect("ERROR");
                                                    break;
                                                }
                                            } else {
                                                let combat_status = {
                                                    let mut guard = players.lock().unwrap();
                                                    guard.get_mut(name).map(|p| p.combat.clone()).unwrap_or("unknown".to_string())
                                                };
                                                let msg = format!(
                                                    "OK {{attacker_hp: {}, target_hp: {}, damage_dealt: {}, damage_taken: 0, status: {}}}\n",
                                                    hp.unwrap_or(0), npc_hp, pv.unwrap_or(0), combat_status
                                                );
                                                stream.write_all(msg.as_bytes()).expect("ERROR");
                                            }
                                        }
                                        Some("DEFEND") => {
                                            let msg = format!("YOU CHOICE DEFEND\n");
                                            stream.write_all(msg.as_bytes()).expect("ERROR");
                                            let pv = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.pv.clone())
                                            };
                                            let npc_info = {
                                                let w = world.lock().unwrap();
                                                w.npcs.get(&npc_id).map(|n| (n.stats.hp, n.stats.damage, n.hostile))
                                            };
                                            let (npc_hp, npc_damage, _npc_hostile) = npc_info.unwrap();
                                            let hp = {
                                                let mut guard = players.lock().unwrap();
                                                if let Some(p) = guard.get_mut(name) {
                                                    p.hp = p.hp.saturating_sub(npc_damage.unwrap_or(0) / 2);
                                                    Some(p.hp)
                                                } else {
                                                    None
                                                }
                                            };
                                            let combat_status = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.combat.clone()).unwrap_or("unknown".to_string())
                                            };
                                            let msg_repost = format!(
                                                "repost {{attacker_hp: {}, target_hp: {}, damage_dealt: {}, damage_taken: {}, status: {}}}\n",
                                                hp.unwrap_or(0), npc_hp, pv.unwrap_or(0), npc_damage.unwrap_or(0) / 2, combat_status
                                            );
                                            stream.write_all(msg_repost.as_bytes()).expect("ERROR");
                                            if hp.unwrap_or(0) < 1 {
                                                let respawn_room = {
                                                    let w = world.lock().unwrap();
                                                    w.world.respawn_location.clone()
                                                };
                                                {
                                                    let mut w = world.lock().unwrap();
                                                    if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                        n.engaged_by = None;
                                                    }
                                                }
                                                {
                                                    let mut guard = players.lock().unwrap();
                                                    if let Some(p) = guard.get_mut(name) {
                                                        p.hp = 50;
                                                        p.combat = "not in combat".to_string();
                                                        p.room = respawn_room.clone();
                                                    }
                                                }
                                                let msg_death = format!("OK you died, respawned in {} with 50 hp\n", respawn_room);
                                                stream.write_all(msg_death.as_bytes()).expect("ERROR");
                                                break;
                                            }
                                        }
                                        Some("FLEE") => {
                                            {
                                                let mut w = world.lock().unwrap();
                                                if let Some(n) = w.npcs.get_mut(&npc_id) {
                                                    n.engaged_by = None;
                                                }
                                            }
                                            let _out_combat = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.combat = "flee".to_string())
                                            };
                                            let msg = format!("you flee the combat\n");
                                            stream.write(msg.as_bytes()).expect("ERROR");
                                            break;
                                        }
                                        Some("STATUS") => {
                                            let in_combat = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.hp)
                                            };
                                            let combat_status = {
                                                let mut guard = players.lock().unwrap();
                                                guard.get_mut(name).map(|p| p.combat.clone()).unwrap_or("unknown".to_string())
                                            };
                                            let msg = format!("OK {{hp={:?},max_hp: 100 ,status : {}}}\n", in_combat.unwrap_or(0), combat_status);
                                            stream.write_all(msg.as_bytes()).expect("ERROR PRINT TATUS");
                                        }
                                        Some(cmd_inconnue) => {
                                            stream.write_all(b"ERROR COMMANDE YOU CAN ATTACK DEFEND FLEEE STATUS\n").expect("Failder to write reponse");
                                            println!("COMMANDE ERROR: {}", cmd_inconnue);
                                        }
                                        None => {
                                            stream.write_all(b"ERROR COMMANDE YOU CAN ATTACK DEFEND FLEEE STATUS\n").expect("Failder to write reponse");
                                            println!("COMMANDE ERROR");
                                        }
                                    }
                                }
                            }
                        }
                        else {
                            let msg = format!("ERR 404 NPC_NOT_FOUND\n");
                            stream.write_all(msg.as_bytes()).expect("ERROR");
                            return ;
                        }
                        }
                    println!("USER USE LOOK");
                }
        Some(cmd_inconnue) => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR: {}", cmd_inconnue);
        }
        _none => {
            stream.write_all(b"ERROR COMMANDE\n").expect("Failder to write reponse");
            println!("COMMANDE ERROR");
        }
    }
}
