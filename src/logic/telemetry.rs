use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha1::{Digest, Sha1};

use crate::gui::{ConnectionMode, EngineState, NetworkMode};
use crate::platform::ProcessMemory;

const TELEMETRY_URL: &str = "http://splatpost.spbr.net/post";
const MATCH_ACTIVE_ADDR: u64 = 0x101E4FF8;
const PAYLOAD_PTR_ADDR: u64 = 0x101DCDB0;
const PAYLOAD_OFFSET: u64 = 0x150;
const PAYLOAD_SIZE: usize = 0x1800;
const FACEIMG_PATTERN: [u8; 16] = [
    0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x80, 0x00,
];
const FACEIMG_SIZE: usize = 65580;
const BOSS_UNIQUE_ID: &str = "0162b";

pub struct SharedConfig {
    pub connection_mode: ConnectionMode,
    pub network_mode: NetworkMode,
}

struct TelemetryState {
    last_start_network_time: u32,
}

pub fn spawn_telemetry_loop(
    engine_state: Arc<Mutex<EngineState>>,
    shared_config: Arc<Mutex<SharedConfig>>,
) {
    let telemetry_state = Arc::new(Mutex::new(TelemetryState {
        last_start_network_time: 0,
    }));

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(5));

            let config = match shared_config.lock() {
                Ok(c) => c,
                Err(_) => continue,
            };

            if config.connection_mode != ConnectionMode::Cemu
                || config.network_mode != NetworkMode::Spacebar
            {
                drop(config);
                continue;
            }
            drop(config);

            let pm = {
                let es = match engine_state.lock() {
                    Ok(es) => es,
                    Err(_) => continue,
                };
                match &es.pmem {
                    Some(ProcessMemory::Native(_)) => es.pmem.clone(),
                    Some(ProcessMemory::WiiU(_)) => None,
                    None => None,
                }
            };

            let pm = match pm {
                Some(pm) => pm,
                None => continue,
            };

            if let Err(e) = tick(&pm, &telemetry_state) {
                eprintln!("[telemetry] tick error: {e}");
            }
        }
    });
}

fn tick(pm: &ProcessMemory, telemetry_state: &Arc<Mutex<TelemetryState>>) -> anyhow::Result<()> {
    let match_status = pm.read_u32(MATCH_ACTIVE_ADDR)?;
    if match_status != 0 {
        return Ok(());
    }

    let payload_base = pm.read_u32(PAYLOAD_PTR_ADDR)? as u64;
    let resolved_addr = payload_base + PAYLOAD_OFFSET;
    let payload_bytes = pm.read_bytes(resolved_addr, PAYLOAD_SIZE)?;

    let pattern_offset = payload_bytes
        .windows(FACEIMG_PATTERN.len())
        .position(|w| w == FACEIMG_PATTERN);
    let pattern_offset = match pattern_offset {
        Some(offset) => offset,
        None => return Ok(()),
    };

    let mut fields = parse_telemetry_payload(&payload_bytes);
    if fields.is_empty() {
        return Ok(());
    }

    let start_network_time: u32 = match fields.get("StartNetworkTime") {
        Some(val) => val.parse().unwrap_or(0),
        None => return Ok(()),
    };

    {
        let ts_guard = telemetry_state.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        if ts_guard.last_start_network_time == start_network_time {
            return Ok(());
        }
    }

    {
        let mut ts_guard = telemetry_state.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        ts_guard.last_start_network_time = start_network_time;
    }

    let total_available = PAYLOAD_SIZE.saturating_sub(pattern_offset);
    let faceimg_size = FACEIMG_SIZE.min(total_available);
    let faceimg_data = pm.read_bytes(resolved_addr + pattern_offset as u64, faceimg_size)?;

    fields.insert("ServerEnv".to_string(), "L1".to_string());

    let boundary = generate_boundary();
    let body = build_multipart_body(&fields, &faceimg_data, &boundary);
    let content_type = format!("multipart/form-data; boundary={boundary}");

    let sha1_hex = compute_sha1_hex(&body);

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0")
        .build()?;

    let response = client
        .post(TELEMETRY_URL)
        .header("X-BOSS-UniqueId", BOSS_UNIQUE_ID)
        .header("X-BOSS-Digest", &sha1_hex)
        .header("Content-Type", &content_type)
        .body(body)
        .send();

    match response {
        Ok(resp) => {
            let status = resp.status();
            if !status.is_success() {
                eprintln!("[telemetry] server returned status {status}");
            }
        }
        Err(e) => {
            eprintln!("[telemetry] request failed: {e}");
        }
    }

    Ok(())
}

fn parse_telemetry_payload(raw: &[u8]) -> HashMap<String, String> {
    let mut result = HashMap::new();

    let actual_start = match raw.iter().position(|&b| b == 0x2D) {
        Some(pos) => pos,
        None => return result,
    };

    let content = String::from_utf8_lossy(&raw[actual_start..]);

    let mut lines = content.lines();
    let boundary_line = match lines.next() {
        Some(l) => l.trim().to_string(),
        None => return result,
    };

    if boundary_line.is_empty() {
        return result;
    }

    let boundary = boundary_line.trim_start_matches('-').to_string();
    if boundary.is_empty() {
        return result;
    }

    let delimiter = format!("--{}", boundary);
    let parts: Vec<&str> = content.split(&delimiter).collect();

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() || trimmed == "--" {
            continue;
        }

        if !trimmed.contains("Content-Disposition") {
            continue;
        }

        if trimmed.contains("name=\"FaceImg\"") {
            continue;
        }

        let name = match extract_field_name_from_part(trimmed) {
            Some(n) => n,
            None => continue,
        };

        let value = match extract_field_value_from_part(trimmed) {
            Some(v) => v,
            None => continue,
        };

        result.insert(name, value);
    }

    result
}

fn extract_field_name_from_part(part: &str) -> Option<String> {
    let name_marker = "name=\"";
    let name_start = part.find(name_marker)?;
    let after_marker = &part[name_start + name_marker.len()..];
    let end_quote = after_marker.find('"')?;
    Some(after_marker[..end_quote].to_string())
}

fn extract_field_value_from_part(part: &str) -> Option<String> {
    let value_start = match part.find("\r\n\r\n") {
        Some(pos) => pos + 4,
        None => match part.find("\n\n") {
            Some(pos) => pos + 2,
            None => return None,
        },
    };

    let value = &part[value_start..];
    let value = value.trim_matches(|c: char| c == '\r' || c == '\n' || c == '-' || c == ' ');

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn build_multipart_body(
    fields: &HashMap<String, String>,
    faceimg: &[u8],
    boundary: &str,
) -> Vec<u8> {
    let mut body = Vec::new();

    for (key, value) in fields {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{}\"\r\n", key).as_bytes(),
        );
        body.extend_from_slice(b"Content-Type: text/plain; charset=utf-8\r\n\r\n");
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }

    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"FaceImg\"; filename=\"face.bin\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(faceimg);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{}--\r\n", boundary).as_bytes());

    body
}

fn generate_boundary() -> String {
    use std::time::SystemTime;
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("----SquidModBoundary{:020x}", timestamp)
}

fn compute_sha1_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}