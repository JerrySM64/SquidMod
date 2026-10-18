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

pub struct GeneratedPack {
    pub name: String,
    pub rules_txt: String,
    pub patches_txt: Option<String>,
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
            relative_path: PathBuf::from(&pack.name).join("patches.txt"),
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
    fn rules_txt_uses_template_values() {
        let rules = generate_rules_txt("My Pack", "Splatoon/Graphics/MyPack", "Does things");
        assert!(rules.starts_with("[Definition]\n"));
        assert!(rules.contains("titleIds = 0005000010176900,0005000010176A00,0005000010162B00"));
        assert!(rules.contains("name = My Pack"));
        assert!(rules.contains("path = \"Splatoon/Graphics/MyPack\""));
        assert!(rules.contains("description = Does things"));
        assert!(rules.contains("version = 7"));
        assert!(!rules.contains("<TO BE CONFIGURED>"));
    }

    #[test]
    fn patches_txt_none_when_patch_text_blank() {
        assert!(generate_patches_txt("My-Graphic-Pack", "").is_none());
        assert!(generate_patches_txt("My-Graphic-Pack", "   \n\t ").is_none());
    }

    #[test]
    fn patches_txt_uses_header_and_module_matches() {
        let patches = generate_patches_txt("My-Graphic-Pack", "0x00000000 = 0x00000001")
            .expect("patches should be generated");
        assert!(patches.starts_with("[My-Graphic-Pack]\n"));
        assert!(patches.contains("moduleMatches = 0x659C782E\n\n0x00000000 = 0x00000001\n"));
        assert!(patches.ends_with("0x00000000 = 0x00000001\n"));
    }

    #[test]
    fn full_pack_produces_complete_structure() {
        let pack = GeneratedPack {
            name: "My Pack".to_string(),
            rules_txt: "rules".to_string(),
            patches_txt: Some("patches".to_string()),
            content_dir: Some(PathBuf::from("/tmp/modpack")),
        };
        let entries = build_output_entries(&pack);
        assert_eq!(entries.len(), 3);

        match &entries[0] {
            OutputEntry::File {
                relative_path,
                contents,
            } => {
                assert_eq!(relative_path, &PathBuf::from("My Pack").join("rules.txt"));
                assert_eq!(contents, "rules");
            }
            OutputEntry::CopyDir { .. } => panic!("expected rules.txt file entry"),
        }

        match &entries[1] {
            OutputEntry::File {
                relative_path,
                contents,
            } => {
                assert_eq!(relative_path, &PathBuf::from("My Pack").join("patches.txt"));
                assert_eq!(contents, "patches");
            }
            OutputEntry::CopyDir { .. } => panic!("expected patches.txt file entry"),
        }

        match &entries[2] {
            OutputEntry::CopyDir {
                source,
                relative_path,
            } => {
                assert_eq!(source, &PathBuf::from("/tmp/modpack"));
                assert_eq!(relative_path, &PathBuf::from("My Pack").join("content"));
            }
            OutputEntry::File { .. } => panic!("expected content copy entry"),
        }
    }

    #[test]
    fn empty_patch_text_skips_patches_txt_entry() {
        let pack = GeneratedPack {
            name: "My Pack".to_string(),
            rules_txt: "rules".to_string(),
            patches_txt: None,
            content_dir: Some(PathBuf::from("/tmp/modpack")),
        };
        let entries = build_output_entries(&pack);
        assert_eq!(entries.len(), 2);
        match &entries[1] {
            OutputEntry::CopyDir {
                source,
                relative_path,
            } => {
                assert_eq!(source, &PathBuf::from("/tmp/modpack"));
                assert_eq!(relative_path, &PathBuf::from("My Pack").join("content"));
            }
            OutputEntry::File { .. } => panic!("patches.txt should be skipped"),
        }
    }

    #[test]
    fn patch_only_pack_skips_content_folder() {
        let pack = GeneratedPack {
            name: "My Pack".to_string(),
            rules_txt: "rules".to_string(),
            patches_txt: Some("patches".to_string()),
            content_dir: None,
        };
        let entries = build_output_entries(&pack);
        assert_eq!(entries.len(), 2);
        for entry in &entries {
            assert!(matches!(entry, OutputEntry::File { .. }));
        }
    }

    #[test]
    fn generate_pack_trims_name_and_wires_generation() {
        let pack = generate_pack(
            "  My Pack  ",
            "Splatoon/Graphics/MyPack",
            "Desc",
            "My-Pack",
            "some patch",
            Some(Path::new("/tmp/modpack")),
        );
        assert_eq!(pack.name, "My Pack");
        assert!(pack.rules_txt.contains("name = My Pack"));
        assert!(pack
            .patches_txt
            .as_deref()
            .unwrap_or_default()
            .starts_with("[My-Pack]"));
        assert_eq!(pack.content_dir, Some(PathBuf::from("/tmp/modpack")));
    }

    #[test]
    fn generate_pack_skips_patches_when_text_empty() {
        let pack = generate_pack("My Pack", "p", "d", "My-Pack", "   ", None);
        assert!(pack.patches_txt.is_none());
        assert!(pack.content_dir.is_none());
        let entries = build_output_entries(&pack);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            OutputEntry::File { relative_path, .. } => {
                assert_eq!(relative_path, &PathBuf::from("My Pack").join("rules.txt"));
            }
            OutputEntry::CopyDir { .. } => panic!("expected rules.txt file entry"),
        }
    }
}
