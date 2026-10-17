use serde::Deserialize;

#[derive(Deserialize)]
struct LatestRelease {
    tag_name: String,
}

pub struct UpdateStatus {
    pub latest_tag: String,
    pub current_version: String,
    pub update_available: bool,
}

pub fn check_for_updates() -> Option<UpdateStatus> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();

    let client = reqwest::blocking::Client::builder()
        .user_agent("SquidMod")
        .build()
        .ok()?;

    let release: LatestRelease = client
        .get("https://api.github.com/repos/JerrySM64/SquidMod/releases/latest")
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .ok()?;

    let latest_tag = release.tag_name;
    let normalized = latest_tag
        .strip_prefix('v')
        .unwrap_or(&latest_tag)
        .to_string();
    let update_available = normalized != current_version;

    Some(UpdateStatus {
        latest_tag,
        current_version,
        update_available,
    })
}
