use super::super::address::CodeWindow;
use super::super::options::{CemuHeader, PortOptions};
use super::super::{cemu_to_console, console_to_cemu};

const VECTOR_A_GECKO: &str = "0EDF6C0C 2C030000\n0EDF6C10 4C810020\n0EDF6C14 38000000\n0EDF6C18 4B60AD7C\n0E401990 489F527C\n0EDF6C1C 2C030000\n0EDF6C20 4C810020\n0EDF6C24 38000001\n0EDF6C28 4B60ADB4\n0E4019D8 489F5244\n0EDF6C30 2C030000\n0EDF6C34 4081000C\n0EDF6C38 4B21CA19\n0EDF6C3C 4B90D810\n0EDF6C40 4B90D818\n0E704448 486F27E8";

const VECTOR_A_CEMU: &str = "\
0x02401990 = b cave1
0x024019D8 = b cave2
0x02704448 = b cave3

.origin = codecave

cave1:
cmpwi r3,0
blelr
li r0,0
b 0x02401994

cave2:
cmpwi r3,0
blelr
li r0,1
b 0x024019DC
nop

cave3:
cmpwi r3,0
ble cave4
bl 0x02013650
b 0x0270444C

cave4:
b 0x02704458";

const VECTOR_B_GECKO: &str = "\
0EDF6C0C 2C030000
0EDF6C10 4C810020
0EDF6C14 38000000
0EDF6C18 4B60AD7C
0EDF6C1C 2C030000
0EDF6C20 4C810020
0EDF6C24 38000001
0EDF6C28 4B60ADB4
0EDF6C2C 60000000
0EDF6C30 2C030000
0EDF6C34 4081000C
0EDF6C38 4B21CA19
0EDF6C3C 4B90D810
0EDF6C40 4B90D818
0E401990 489F527C
0E4019D8 489F5244
0E704448 486F27E8";

const VECTOR_D_GECKO: &str = "0E83D2E0 3D800EDF\n0E83D2E4 618C5000\n0E83D2E8 7D8903A6\n0E83D2EC 4E800420\n0EDF5000 38600002\n0EDF5004 4E800020";

fn sorted_writes(gecko: &str) -> Vec<String> {
    let mut lines: Vec<String> = gecko.lines().map(|line| line.trim().to_uppercase()).collect();
    lines.sort();
    lines
}

#[test]
fn vector_a_console_to_cemu() {
    let ported = console_to_cemu(VECTOR_A_GECKO, &PortOptions::default()).unwrap();
    assert_eq!(ported.text, VECTOR_A_CEMU);
    assert!(ported.warnings.is_empty());
}

#[test]
fn vector_b_cemu_to_console() {
    let ported = cemu_to_console(VECTOR_A_CEMU, 0x02DF_6C0C, &PortOptions::default()).unwrap();
    assert_eq!(ported.text, VECTOR_B_GECKO);
    assert!(ported.warnings.is_empty());
}

#[test]
fn vector_c_relocated_cave_base() {
    let ported = cemu_to_console(VECTOR_A_CEMU, 0x02DF_0000, &PortOptions::default()).unwrap();
    let lines: Vec<&str> = ported.text.lines().collect();
    assert!(lines.contains(&"0E401990 489EE670"));
    assert!(lines.contains(&"0EDF000C 4B611988"));
    assert_eq!(lines.iter().position(|l| *l == "0E401990 489EE670"), Some(14));
}

#[test]
fn vector_d_pointer_relocation_roundtrip() {
    let cemu = console_to_cemu(VECTOR_D_GECKO, &PortOptions::default()).unwrap();
    assert!(cemu.text.contains("0x0283D2E0 = lis r12,cave1@h"));
    assert!(cemu.text.contains("0x0283D2E4 = ori r12,r12,cave1@l"));
    assert!(cemu.text.contains("0x0283D2E8 = mtctr r12"));
    assert!(cemu.text.contains("0x0283D2EC = bctr"));
    let console = cemu_to_console(&cemu.text, 0x02DF_5000, &PortOptions::default()).unwrap();
    assert_eq!(sorted_writes(&console.text), sorted_writes(VECTOR_D_GECKO));
}

#[test]
fn pointer_relocation_with_addi_uses_ha() {
    let gecko = "0E83D2E0 3D800EE0\n0E83D2E4 398C9000\n0EDF9000 4E800020";
    let cemu = console_to_cemu(gecko, &PortOptions::default()).unwrap();
    assert!(cemu.text.contains("0x0283D2E0 = lis r12,cave1@ha"));
    assert!(cemu.text.contains("0x0283D2E4 = addi r12,r12,cave1@l"));
    let console = cemu_to_console(&cemu.text, 0x02DF_9000, &PortOptions::default()).unwrap();
    assert_eq!(sorted_writes(&console.text), sorted_writes(gecko));
}

#[test]
fn code_pointers_are_translated_between_address_spaces() {
    let gecko = "0E89DD98 3F200EDF\n0E89DD9C 38B9C2B0";
    let cemu = console_to_cemu(gecko, &PortOptions::default()).unwrap();
    assert_eq!(cemu.text, "0x0289DD98 = lis r25,0x2df\n0x0289DD9C = subi r5,r25,15696");
    assert!(cemu.warnings.is_empty());
    let console = cemu_to_console(&cemu.text, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, gecko);
    assert!(console.warnings.is_empty());
}

#[test]
fn data_pointers_are_translated_in_both_halves() {
    let gecko = "0E89DD98 3C601063\n0E89DD9C 3863C2A8";
    let cemu = console_to_cemu(gecko, &PortOptions::default()).unwrap();
    assert_eq!(cemu.text, "0x0289DD98 = lis r3,0x1013\n0x0289DD9C = subi r3,r3,27992");
    assert!(cemu.warnings.is_empty());
    let console = cemu_to_console(&cemu.text, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, gecko);
    assert!(console.warnings.is_empty());
}

#[test]
fn cave_labels_resolve_to_console_addresses() {
    let asm = "0x0289DD98 = lis r25,text@ha\n0x0289DD9C = addi r4,r25,text@l\n.origin = codecave\ntext: .string \"hi\"";
    let console = cemu_to_console(asm, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, "0EDF0000 68690000\n0E89DD98 3F200EDF\n0E89DD9C 38990000");
}

#[test]
fn dangling_cave_branch_is_an_error() {
    let err = console_to_cemu("0EDF5000 48000040\n0EDF5004 4E800020", &PortOptions::default()).unwrap_err();
    assert!(err.contains("0x02DF5000"));
    assert!(err.contains("0x02DF5040"));
}

#[test]
fn branch_into_filled_gap_gets_label() {
    let gecko = "0EDF5000 48000008\n0EDF500C 4E800020";
    let cemu = console_to_cemu(gecko, &PortOptions::default()).unwrap();
    assert_eq!(cemu.text, ".origin = codecave\n\ncave1:\nb cave2\nnop\n\ncave2:\nnop\nblr");
}

#[test]
fn cemu_text_in_console_mode_names_the_right_mode() {
    let err = console_to_cemu(".origin=codecave\n\n0x0289DD4C = li r0,1", &PortOptions::default()).unwrap_err();
    assert!(err.starts_with("line 1:"));
    assert!(err.contains("Cemu → Console"));
}

#[test]
fn misaligned_gecko_address_is_an_error() {
    let err = console_to_cemu("10123456 38600001", &PortOptions::default()).unwrap_err();
    assert!(err.contains("line 1"));
}

#[test]
fn data_writes_are_translated_and_not_disassembled() {
    let cemu = console_to_cemu("1062C2A8 38600001\n0E83D2E0 38600001", &PortOptions::default()).unwrap();
    assert_eq!(cemu.text, "0x0283D2E0 = li r3,1\n0x101292A8 = .int 0x38600001");
}

#[test]
fn duplicate_writes_keep_the_last_value() {
    let cemu = console_to_cemu("0E83D2E0 38600001\n0E83D2E0 38600002", &PortOptions::default()).unwrap();
    assert_eq!(cemu.text, "0x0283D2E0 = li r3,2");
    assert_eq!(cemu.warnings.len(), 1);
}

#[test]
fn header_is_rendered() {
    let opts = PortOptions {
        header: Some(CemuHeader { name: "Debug_Move".to_string(), module_matches: 0x659C_782E }),
        ..PortOptions::default()
    };
    let cemu = console_to_cemu("0E83D2E0 38600001", &opts).unwrap();
    assert_eq!(cemu.text, "[Debug_Move]\nmoduleMatches = 0x659C782E\n\n0x0283D2E0 = li r3,1");
}

#[test]
fn full_cemu_file_with_header_and_comments() {
    let asm = "[Debug_Move] ; group\nmoduleMatches = 0x659C782E\n\n0x02401990 = b hook # jump\n.origin = codecave\nhook: li r3,1\n// trailing\nb 0x02401994\n";
    let console = cemu_to_console(asm, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, "0EDF0000 38600001\n0EDF0004 4B611990\n0E401990 489EE670");
}

#[test]
fn data_directives_are_laid_out_like_cemu() {
    let asm = ".origin = codecave\nvalue: .byte 1\n.align 4\nptr: .int value\n.short 0x1234\ntext: .string \"hi\"";
    let console = cemu_to_console(asm, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, "0EDF0000 01000000\n0EDF0004 0EDF0000\n0EDF0008 12346869\n0EDF000C 00000000");
}

#[test]
fn data_directives_in_patch_writes() {
    let asm = ".origin=codecave\n\n0x0289DD4C = li r0,1\n0x101292a8 = .string \"Woomy\"\n0x0289dd98 = lis r25, BuildDate@h\n0x0289de58 = addi r4, r25, BuildDate\n\nBuildDate:\n.string \"[@jerrysm64]\"";
    let console = cemu_to_console(asm, 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(
        console.text,
        "0EDF0000 5B406A65\n0EDF0004 72727973\n0EDF0008 6D36345D\n0EDF000C 00000000\n0E89DD4C 38000001\n1062C2A8 576F6F6D\n1062C2AC 79000000\n0E89DD98 3F200EDF\n0E89DE58 38990000"
    );
    assert_eq!(console.warnings.len(), 2);
    assert!(console.warnings[0].starts_with("line 4:"));
    assert!(console.warnings[1].starts_with("line 6:"));
}

#[test]
fn unaligned_data_patch_is_padded_on_both_sides() {
    let console = cemu_to_console("0x10000002 = .short 0x1234, 0x5678", 0x02DF_0000, &PortOptions::default()).unwrap();
    assert_eq!(console.text, "10503000 00001234\n10503004 56780000");
    assert_eq!(console.warnings.len(), 1);
}

#[test]
fn unknown_label_reports_line_number() {
    let err = cemu_to_console(".origin = codecave\nnop\nb nowhere", 0x02DF_0000, &PortOptions::default()).unwrap_err();
    assert!(err.contains("line 3"));
    assert!(err.contains("nowhere"));
}

#[test]
fn second_group_is_an_error() {
    let err = cemu_to_console("[A]\nmoduleMatches = 0x1\n[B]\n", 0x02DF_0000, &PortOptions::default()).unwrap_err();
    assert!(err.contains("line 3"));
}

#[test]
fn unsupported_references_are_errors() {
    assert!(cemu_to_console(".origin = codecave\nbl import.coreinit.OSReport", 0x02DF_0000, &PortOptions::default()).is_err());
    assert!(cemu_to_console("0x02401990 = li r3,$value", 0x02DF_0000, &PortOptions::default()).is_err());
}

#[test]
fn misaligned_cave_base_is_an_error() {
    assert!(cemu_to_console(".origin = codecave\nnop", 0x02DF_0002, &PortOptions::default()).is_err());
}

#[test]
fn code_window_is_bounded() {
    let window = CodeWindow::default();
    assert_eq!(window.to_exec(0x1012_3456), 0x1012_3456);
    assert_eq!(window.to_exec(0x0E83_D2E0), 0x0283_D2E0);
    assert_eq!(window.to_console(0x1000_0000), 0x1000_0000);
    assert_eq!(window.to_console(0x0283_D2E0), 0x0E83_D2E0);
    assert!(window.is_code(0x0283_D2E0));
    assert!(!window.is_code(0x1000_0000));
}

#[test]
fn lone_data_halves_are_translated_both_ways() {
    let gecko = "0E53E06C 488B791C\n0E53E070 60000000\n0E539E94 488BBB0C\n0EDF5988 806B85E4\n0EDF598C 2C030000\n0EDF5990 4182000C\n0EDF5994 2C000003\n0EDF5998 4B7486DC\n0EDF599C 4B748730\n0EDF59A0 3F20106F\n0EDF59A4 2C060000\n0EDF59A8 41820008\n0EDF59AC 4B7444EC\n0EDF59B0 4B744618";
    let cemu = console_to_cemu(gecko, &PortOptions::default()).unwrap();
    assert!(cemu.text.contains("cave1:\nlwz r3,21988(r11)"));
    assert!(cemu.text.contains("cave3:\nlis r25,0x101f"));
    assert_eq!(cemu.warnings.len(), 2);
    assert!(cemu.warnings.iter().any(|w| w.contains("0x101E")));
    assert!(cemu.warnings.iter().any(|w| w.contains("0x55E4")));
    let console = cemu_to_console(&cemu.text, 0x02DF_5988, &PortOptions::default()).unwrap();
    assert_eq!(sorted_writes(&console.text), sorted_writes(gecko));
    assert_eq!(console.warnings.len(), 2);
}
