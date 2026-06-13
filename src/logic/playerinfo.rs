use crate::ProcessMemory;
use crate::gui::NetworkMode;
use crate::id::{
    clothes_name, eye_color_name, headgear_name, rank_label, shoes_name, tank_name,
    weapon_name_main, weapon_name_special, weapon_name_sub,
};
use anyhow::Result;
use chrono::{DateTime, Local};
use reqwest::blocking::Client;
use roxmltree::Document;
use serde::{Deserialize, Serialize};

pub const PLAYER_ROOT_PTR: u64 = 0x101DD330;
pub const PLAYER_LIST_OFFSET: u64 = 0x10;
pub const PLAYER_SLOT_STRIDE: u64 = 0x4;
pub const OFF_NAME: u64 = 0x6;
pub const OFF_GENDER: u64 = 0x34;
pub const OFF_SKIN_TONE: u64 = 0x38;
pub const OFF_EYE_COLOR: u64 = 0x3C;
pub const OFF_SHOES: u64 = 0x54;
pub const OFF_CLOTH: u64 = 0x70;
pub const OFF_HAT: u64 = 0x8C;
pub const OFF_TANK_ID: u64 = 0xA8;
pub const OFF_RANK: u64 = 0xAC;
pub const OFF_RANK_POINTS: u64 = 0xB0;
pub const OFF_FEST_TEAM: u64 = 0xB4;
pub const OFF_FEST_ID: u64 = 0xB8;
pub const OFF_FEST_GRADE: u64 = 0xBC;
pub const OFF_WEAPONID_MAIN: u64 = 0x44;
pub const OFF_WEAPONID_SUB: u64 = 0x48;
pub const OFF_WEAPONID_SPECIAL: u64 = 0x4C;
pub const OFF_WEAPONTURF_TOTAL: u64 = 0x50;
pub const OFF_PID: u64 = 0xD0;

pub const SESSION_ROOT_PTR: u64 = 0x101E8980;
pub const SESSION_INDEX_OFFSET: u64 = 0xBD;
pub const SESSION_ID_BASE_OFFSET: u64 = 0xCC;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRecord {
    pub index: u8,
    pub name: String,
    pub pid_hex: String,
    pub pid_dec: u32,
    pub pnid: String,
    pub gender: i8,
    pub skin_tone: u8,
    pub eye_color: u8,
    pub eye_color_name: String,
    pub headgear: i32,
    pub headgear_name: String,
    pub clothes: i32,
    pub clothes_name: String,
    pub shoes: i32,
    pub shoes_name: String,
    pub tank_id: i32,
    pub tank_name: String,
    pub weapon_id_main: i16,
    pub weapon_main_name: String,
    pub weapon_id_sub: i16,
    pub weapon_sub_name: String,
    pub weapon_id_special: i8,
    pub weapon_special_name: String,
    pub weaponturf_total: i32,
    pub rank: i8,
    pub rank_points: i8,
    pub rank_label: String,
    pub fest_team: i32,
    pub fest_id: i32,
    pub fest_grade: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub players: Vec<PlayerRecord>,
    pub session_id: Option<u32>,
    pub fetched_at: DateTime<Local>,
}

fn get_pnid(pid: i32, network_mode: NetworkMode) -> String {
    let client = match Client::builder().user_agent("Mozilla/5.0").build() {
        Ok(c) => c,
        Err(_) => return "0".to_string(),
    };

    match network_mode {
        NetworkMode::Pretendo => {
            let url = format!("http://account.pretendo.cc/v1/api/miis?pids={}", pid);
            let response = match client
                .get(&url)
                .header("X-Nintendo-Client-ID", "a2efa818a34fa16b8afbc8a74eba3eda")
                .header(
                    "X-Nintendo-Client-Secret",
                    "c91cdb5658bd4954ade78533a339cf9a",
                )
                .send()
            {
                Ok(r) => r,
                Err(_) => return "0".to_string(),
            };

            if !response.status().is_success() {
                return "0".to_string();
            }

            let body = match response.text() {
                Ok(b) => b,
                Err(_) => return "0".to_string(),
            };

            let doc = match Document::parse(&body) {
                Ok(d) => d,
                Err(_) => return "0".to_string(),
            };

            doc.descendants()
                .find(|n| n.tag_name().name() == "user_id")
                .and_then(|n| n.text())
                .unwrap_or("0")
                .to_string()
        }
        NetworkMode::Spacebar => {
            let url = format!(
                "https://account.spfn.net/v1/api/admin/mapped_ids?input_type=pid&output_type=user_id&input={}",
                pid
            );
            let response = match client.get(&url).send() {
                Ok(r) => r,
                Err(_) => return "0".to_string(),
            };

            if !response.status().is_success() {
                return "0".to_string();
            }

            let body = match response.text() {
                Ok(b) => b,
                Err(_) => return "0".to_string(),
            };

            let doc = match Document::parse(&body) {
                Ok(d) => d,
                Err(_) => return "0".to_string(),
            };

            doc.descendants()
                .find(|n| n.tag_name().name() == "out_id")
                .and_then(|n| n.text())
                .filter(|s| !s.is_empty())
                .unwrap_or("0")
                .to_string()
        }
    }
}

fn decode_name(bytes: &[u8]) -> String {
    let mut out = Vec::new();
    for chunk in bytes.chunks_exact(2) {
        let code = u16::from_be_bytes([chunk[0], chunk[1]]);
        if code == 0 {
            break;
        }
        out.push(code);
    }
    String::from_utf16_lossy(&out)
        .trim()
        .replace(['\n', '\r'], "")
}

pub fn gender_label(code: i8) -> &'static str {
    match code {
        0 => "Girl",
        1 => "Boy",
        2 => "Rival",
        _ => "Unknown",
    }
}

pub fn fetch_all_players(pm: &ProcessMemory, network_mode: NetworkMode) -> Result<FetchResult> {
    let mut players = Vec::new();

    let root = pm.read_u32(PLAYER_ROOT_PTR)? as u64;
    let list_ptr = pm.read_u32(root + PLAYER_LIST_OFFSET)? as u64;

    if list_ptr != 0 {
        for i in 0..8 {
            let player_ptr_addr = list_ptr + ((i as u64) * PLAYER_SLOT_STRIDE);
            let player_ptr = pm.read_u32(player_ptr_addr)? as u64;

            if player_ptr == 0 {
                continue;
            }

            let name_bytes = pm.read_bytes(player_ptr + OFF_NAME, 32)?;
            let name = decode_name(&name_bytes);

            let pid_raw = pm.read_u32(player_ptr + OFF_PID)?;
            let pid_hex = format!("{:08X}", pid_raw);
            let pnid = get_pnid(pid_raw as i32, network_mode);

            let gender = pm.read_u32(player_ptr + OFF_GENDER)? as i8;
            let skin_tone = pm.read_u32(player_ptr + OFF_SKIN_TONE)? as u8;
            let eye_color = pm.read_u32(player_ptr + OFF_EYE_COLOR)? as u8;
            let eye_color_name = eye_color_name(eye_color).to_string();
            let headgear = pm.read_u32(player_ptr + OFF_HAT)? as i32;
            let headgear_name = headgear_name(headgear).to_string();
            let clothes = pm.read_u32(player_ptr + OFF_CLOTH)? as i32;
            let clothes_name = clothes_name(clothes).to_string();
            let shoes = pm.read_u32(player_ptr + OFF_SHOES)? as i32;
            let shoes_name = shoes_name(shoes).to_string();
            let tank_id = pm.read_u32(player_ptr + OFF_TANK_ID)? as i32;
            let tank_name = tank_name(tank_id).to_string();

            let rank = pm.read_u32(player_ptr + OFF_RANK)? as i8;
            let rank_points = pm.read_u32(player_ptr + OFF_RANK_POINTS)? as i8;
            let rank_label = rank_label(rank_points).to_string();
            let fest_team = pm.read_u32(player_ptr + OFF_FEST_TEAM)? as i32;
            let fest_id = pm.read_u32(player_ptr + OFF_FEST_ID)? as i32;
            let fest_grade = pm.read_u32(player_ptr + OFF_FEST_GRADE)? as i32;

            let weapon_id_main = pm.read_u32(player_ptr + OFF_WEAPONID_MAIN)? as i16;
            let weapon_main_name = weapon_name_main(weapon_id_main).to_string();
            let weapon_id_sub = pm.read_u32(player_ptr + OFF_WEAPONID_SUB)? as i16;
            let weapon_sub_name = weapon_name_sub(weapon_id_sub).to_string();
            let weapon_id_special = pm.read_u32(player_ptr + OFF_WEAPONID_SPECIAL)? as i8;
            let weapon_special_name = weapon_name_special(weapon_id_special).to_string();
            let weaponturf_total = pm.read_u32(player_ptr + OFF_WEAPONTURF_TOTAL)? as i32;

            players.push(PlayerRecord {
                index: i as u8,
                name,
                pid_hex,
                pid_dec: pid_raw,
                pnid,
                gender,
                skin_tone,
                eye_color,
                eye_color_name,
                headgear,
                headgear_name,
                clothes,
                clothes_name,
                shoes,
                shoes_name,
                tank_id,
                tank_name,
                weapon_id_main,
                weapon_main_name,
                weapon_id_sub,
                weapon_sub_name,
                weapon_id_special,
                weapon_special_name,
                weaponturf_total,
                rank,
                rank_points,
                rank_label,
                fest_team,
                fest_id,
                fest_grade,
            });
        }
    }

    let session_id = (|| -> Result<u32> {
        let root2 = pm.read_u32(SESSION_ROOT_PTR)? as u64;
        if root2 == 0 {
            return Ok(0);
        }
        let idx = pm.read_bytes(root2 + SESSION_INDEX_OFFSET, 1)?[0] as u64;
        pm.read_u32(root2 + idx + SESSION_ID_BASE_OFFSET)
    })()
    .ok();

    Ok(FetchResult {
        players,
        session_id,
        fetched_at: Local::now(),
    })
}
