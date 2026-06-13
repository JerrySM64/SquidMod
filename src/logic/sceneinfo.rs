use anyhow::Result;
use crate::ProcessMemory;

#[derive(Debug, Clone, Default)]
pub struct SceneRecord {
    pub current_scene_id: u32,
    pub scene_info_id: u32,
    pub last_scene_id: u32,
    pub next_scene_id: u32,
    pub current_mode: String,
    pub current_mode_id: i32,
}

pub fn fetch_scene_info(pm: &ProcessMemory) -> Result<SceneRecord> {
    let base_addr = pm.read_u32(0x101E6770)?;
    let mode_base_addr = pm.read_u32(0x101DCED0).unwrap_or(0);
    let mut current_mode = "Unknown".to_string();
    let mut current_mode_id = -1;

    if mode_base_addr != 0
         && let Ok(mode_id) = pm.read_u32((mode_base_addr as u64) + 0x18) {
             current_mode_id = mode_id as i32;
             current_mode = get_scene_mode_name(current_mode_id).to_string();
         }

    if base_addr == 0 {
        return Ok(SceneRecord {
            current_scene_id: 0,
            scene_info_id: 0,
            last_scene_id: 0,
            next_scene_id: 0,
            current_mode,
            current_mode_id,
        });
    }

    let current_scene_id = pm.read_u32((base_addr as u64) + 0x15C).unwrap_or(0);
    let scene_info_id = pm.read_u32((base_addr as u64) + 0x160).unwrap_or(0);
    let last_scene_id = pm.read_u32((base_addr as u64) + 0x164).unwrap_or(0);
    let next_scene_id = pm.read_u32((base_addr as u64) + 0x168).unwrap_or(0);

    Ok(SceneRecord {
        current_scene_id,
        scene_info_id,
        last_scene_id,
        next_scene_id,
        current_mode,
        current_mode_id,
    })
}

pub fn get_scene_name(id: u32) -> &'static str {
    match id {
        0 => "Boot",
        1 => "Plaza",
        2 => "Lobby",
        3 => "Match",
        4 => "TeamMatch",
        5 => "PrivateMatch",
        6 => "PartyMatch",
        7 => "VSGame",
        8 => "Mission",
        9 => "World",
        10 => "Duel",
        11 => "ShootingRange",
        12 => "WalkThrough",
        13 => "StaffRoll",
        14 => "Shop",
        15 => "Customize",
        16 => "PlayerMake",
        17 => "DuelSetting",
        18 => "DayChange",
        19 => "Tutorial",
        20 => "EndingPlaza",
        21 => "MiniGame",
        22 => "TitleForShow",
        23 => "LobbyForShow",
        24 => "MatchForShow",
        25 => "TutorialForShow",
        26 => "ThanksForShow",
        27 => "DbgEntry",
        28 => "DbgSetting",
        29 => "DummyMatch",
        30 => "DummyTeamMatch",
        31 => "Viewer",
        32 => "LytCheck",
        33 => "TexViewer",
        34 => "FontViewer",
        35 => "ModelCapture",
        36 => "IconCapture",
        37 => "PlainForPhoto",
        38 => "GameSample",
        39 => "LayoutSample",
        40 => "MiiverseSample",
        41 => "MiiSample",
        42 => "NetSample",
        43 => "FreeTest",
        44 => "ColTest",
        45 => "NkjmTest",
        46 => "MiniGTest",
        47 => "NfpTest",
        48 => "CustomTest",
        _ => "Unknown",
    }
}

pub fn get_scene_mode_name(id: i32) -> &'static str {
    match id {
        -1 => "cNone",
        0 => "cVSGame",
        1 => "cMission",
        2 => "cPlaza",
        3 => "cWorld",
        4 => "cTutForShow",
        5 => "cTutorial",
        6 => "cDuel",
        7 => "cDuelSetting",
        8 => "cShootingRange",
        9 => "cWalkThrough",
        10 => "cStaffRoll",
        11 => "cLobby",
        12 => "cShop",
        13 => "cCustomize",
        14 => "cFreeTest",
        15 => "cDbgSetting",
        16 => "cOther",
        _ => "Unknown",
    }
}
