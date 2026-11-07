use std::path::{Path, PathBuf};

const SPLATOON_TITLE_IDS: &str = "0005000010176900,0005000010176A00,0005000010162B00";
const RULES_VERSION: u8 = 7;
const PATCH_MODULE_MATCHES: &str = "0x659C782E";

pub fn generate_rules_txt(name: &str, rules_path: &str, description: &str) -> String {
    format!(
        "[Definition]\ntitleIds = {}\nname = {}\npath = \"{}\"\ndescription = {}\nversion = {}\n",
        SPLATOON_TITLE_IDS, name, rules_path, description, RULES_VERSION
    )
}

pub fn generate_patches_txt(patch_name: &str, patch_text: &str) -> Option<String> {
    let patch_body = patch_text.trim();
    if patch_body.is_empty() {
        return None;
    }
    Some(format!(
        "[{}]\nmoduleMatches = {}\n\n{}\n",
        patch_name, PATCH_MODULE_MATCHES, patch_body
    ))
}

fn has_codecave(patch_text: &str) -> bool {
    patch_text.lines().any(|line| {
        let code = line.split(['#', ';']).next().unwrap_or("");
        let compact: String = code.chars().filter(|c| !c.is_whitespace()).collect();
        compact.eq_ignore_ascii_case(".origin=codecave")
    })
}

pub fn patch_file_name(patch_name: &str, patch_text: &str) -> String {
    if has_codecave(patch_text) {
        format!("patch_{}.asm", patch_name)
    } else {
        "patches.txt".to_string()
    }
}

pub struct GeneratedPack {
    pub name: String,
    pub rules_txt: String,
    pub patches_txt: Option<String>,
    pub patches_file: String,
    pub content_dir: Option<PathBuf>,
}

pub enum OutputEntry {
    File {
        relative_path: PathBuf,
        contents: String,
    },
    CopyDir {
        source: PathBuf,
        relative_path: PathBuf,
    },
}

pub fn generate_pack(
    name: &str,
    rules_path: &str,
    description: &str,
    patch_header: &str,
    patch_text: &str,
    content_dir: Option<&Path>,
) -> GeneratedPack {
    GeneratedPack {
        name: name.trim().to_string(),
        rules_txt: generate_rules_txt(name, rules_path, description),
        patches_txt: generate_patches_txt(patch_header, patch_text),
        patches_file: patch_file_name(patch_header, patch_text),
        content_dir: content_dir.map(|p| p.to_path_buf()),
    }
}

pub fn build_output_entries(pack: &GeneratedPack) -> Vec<OutputEntry> {
    let mut entries = vec![OutputEntry::File {
        relative_path: PathBuf::from(&pack.name).join("rules.txt"),
        contents: pack.rules_txt.clone(),
    }];

    if let Some(patches) = &pack.patches_txt {
        entries.push(OutputEntry::File {
            relative_path: PathBuf::from(&pack.name).join(&pack.patches_file),
            contents: patches.clone(),
        });
    }

    if let Some(source) = &pack.content_dir {
        entries.push(OutputEntry::CopyDir {
            source: source.clone(),
            relative_path: PathBuf::from(&pack.name).join("content"),
        });
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codecave_patches_are_saved_as_asm() {
        assert_eq!(patch_file_name("Debug-Move", ".origin=codecave\nnop"), "patch_Debug-Move.asm");
        assert_eq!(patch_file_name("Debug-Move", "  .ORIGIN = CodeCave ; cave\nnop"), "patch_Debug-Move.asm");
    }

    #[test]
    fn plain_patches_stay_patches_txt() {
        assert_eq!(patch_file_name("Debug-Move", "0x0283D2E0 = li r3,1"), "patches.txt");
        assert_eq!(patch_file_name("Debug-Move", "# .origin = codecave\n0x0283D2E0 = nop"), "patches.txt");
    }
}
