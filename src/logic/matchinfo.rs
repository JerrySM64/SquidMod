use crate::ProcessMemory;
use crate::logic::playerinfo::PLAYER_ROOT_PTR;
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const OFF_MATCH_HOUR: u64 = 0x234;
pub const OFF_MATCH_ID: u64 = 0x238;
pub const OFF_GAMEMODE_ID: u64 = 0x23C;
pub const OFF_MAP_NAME: u64 = 0x28;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchRecord {
    pub hour: i32,
    pub hour_label: String,
    pub match_id: i32,
    pub match_label: String,
    pub gamemode_id: i32,
    pub gamemode_label: String,
    pub map_name: String,
}

pub fn hour_label(hour: i32) -> &'static str {
    match hour {
        0 => "Day Time",
        1 => "Night Mode",
        _ => "Unknown",
    }
}

pub fn match_label(id: i32) -> &'static str {
    match id {
        0 => "Turf War",
        1 => "Ranked Battle",
        2 => "Splatfest",
        3 => "Private Battle",
        4 => "Squad Battle",
        _ => "Unknown",
    }
}

pub fn gamemode_label(id: i32) -> &'static str {
    match id {
        -1 => "cNone",
        0 => "Turf War",
        1 => "Rainmaker",
        2 => "Splat Zones",
        3 => "Tower Control",
        _ => "Unknown",
    }
}

pub fn fetch_match_info(pm: &ProcessMemory) -> Result<MatchRecord> {
    let root = pm.read_u32(PLAYER_ROOT_PTR)? as u64;
    if root == 0 {
        return Err(anyhow::anyhow!("Player root is 0"));
    }

    let hour = pm.read_u32(root + OFF_MATCH_HOUR)? as i32;
    let match_id = pm.read_u32(root + OFF_MATCH_ID)? as i32;
    let gamemode_id = pm.read_u32(root + OFF_GAMEMODE_ID)? as i32;
    
    let map_bytes = pm.read_bytes(root + OFF_MAP_NAME, 64)?;
    let null_pos = map_bytes.iter().position(|&b| b == 0).unwrap_or(map_bytes.len());
    let map_name = String::from_utf8_lossy(&map_bytes[..null_pos]).to_string();

    Ok(MatchRecord {
        hour,
        hour_label: hour_label(hour).to_string(),
        match_id,
        match_label: match_label(match_id).to_string(),
        gamemode_id,
        gamemode_label: gamemode_label(gamemode_id).to_string(),
        map_name,
    })
}
