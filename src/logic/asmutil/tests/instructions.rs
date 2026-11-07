use std::collections::HashMap;

use super::super::assembler::{assemble_at, assemble_with_notes};
use super::super::disassembler::{disassemble_for_cemu, disassemble_with};
use super::super::{
    assemble_instruction, cemu_addr_to_console, cemu_asm_to_gecko, console_addr_to_cemu, disassemble_instruction,
    gecko_convert_addresses, gecko_to_asm,
};

fn none() -> HashMap<String, u32> {
    HashMap::new()
}

#[test]
fn test_li_r3_1() {
    assert_eq!(assemble_instruction("li r3,1").unwrap(), 0x38600001);
}

#[test]
fn test_li_r0_0() {
    assert_eq!(assemble_instruction("li r0,0").unwrap(), 0x38000000);
}

#[test]
fn test_disassemble_li() {
    assert_eq!(disassemble_instruction(0x38600001, 0), "li r3,1");
}

#[test]
fn test_nop() {
    assert_eq!(assemble_instruction("nop").unwrap_or(0), 0x60000000);
    assert_eq!(disassemble_instruction(0x60000000, 0), "nop");
}

#[test]
fn test_blr() {
    assert_eq!(assemble_instruction("blr").unwrap_or(0), 0x4E800020);
    assert_eq!(disassemble_instruction(0x4E800020, 0), "blr");
}

#[test]
fn test_stw() {
    let val = assemble_instruction("stw r3,0(r1)").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "stw r3,0(r1)");
}

#[test]
fn test_lwz() {
    let val = assemble_instruction("lwz r3,4(r1)").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "lwz r3,4(r1)");
}

#[test]
fn test_addi() {
    let val = assemble_instruction("addi r3,r4,100").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "addi r3,r4,100");
}

#[test]
fn test_lis() {
    let val = assemble_instruction("lis r3,0x1234").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "lis r3,0x1234");
}

#[test]
fn test_roundtrip_add() {
    let val = assemble_instruction("add r3,r4,r5").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "add r3,r4,r5");
}

#[test]
fn test_roundtrip_mr() {
    let val = assemble_instruction("mr r3,r4").unwrap_or(0);
    assert_eq!(disassemble_instruction(val, 0), "mr r3,r4");
}

#[test]
fn test_dis_branch_absolute_target() {
    assert_eq!(disassemble_instruction(0x48000008, 0x100), "b 0x00000108");
}

#[test]
fn test_dis_bc_absolute_target() {
    assert_eq!(disassemble_instruction(0x40820008, 0x100), "bne 0x00000108");
}

#[test]
fn test_gecko_roundtrip() {
    let asm_text = "li r3,1";
    let gecko = cemu_asm_to_gecko(asm_text, 0x0EDF5000).unwrap_or("".to_string());
    assert_eq!(gecko, "0EDF5000 38600001");
    let back = gecko_to_asm(&gecko).unwrap_or("".to_string());
    assert!(back.contains("li r3,1"));
}

#[test]
fn test_gecko_multi_cave_reconstruction() {
    let gecko = "# test patch\n0E83D2E0 485B7D20\n0E83D2E4 38600001\n0EDF5000 38600002\n0EDF5004 4E800020\n0EDF6000 4BFFF000\n0EDF6004 4E800020";
    let asm = gecko_to_asm(gecko).unwrap_or("".to_string());
    assert!(asm.contains("0x0283D2E0 = b cave1"));
    assert!(asm.contains("0x0283D2E4 = li r3,1"));
    assert!(asm.contains(".origin = codecave"));
    assert!(asm.contains("cave1:"));
    assert!(asm.contains("cave2:"));
    assert!(asm.contains("b cave1"));
}

#[test]
fn test_gecko_cave_branch_to_unlabeled_target() {
    let gecko = "0EDF5000 48000040\n0EDF5004 4E800020";
    let err = gecko_to_asm(gecko).unwrap_err();
    assert!(err.contains("0x02DF5040"));
}

#[test]
fn test_gecko_standard_writes_only() {
    let gecko = "0E83D100 38600001\n0E83D200 4E800020";
    let asm = gecko_to_asm(gecko).unwrap_or("".to_string());
    assert!(asm.contains("0x0283D100 = li r3,1"));
    assert!(asm.contains("0x0283D200 = blr"));
    assert!(!asm.contains("codecave"));
}

#[test]
fn test_gecko_contiguous_cave_blocks_get_labels() {
    let gecko = "0EDF6C0C 2C030000\n0EDF6C10 4C810020\n0EDF6C14 38000000\n0EDF6C18 4B60AD7C\n0E401990 489F527C\n0EDF6C1C 2C030000\n0EDF6C20 4C810020\n0EDF6C24 38000001\n0EDF6C28 4B60ADB4\n0E4019D8 489F5244\n0EDF6C30 2C030000\n0EDF6C34 4081000C\n0EDF6C38 4B21CA19\n0EDF6C3C 4B90D810\n0EDF6C40 4B90D818\n0E704448 486F27E8";
    let asm = gecko_to_asm(gecko).unwrap_or("".to_string());
    assert!(asm.contains("0x02401990 = b cave1"));
    assert!(asm.contains("0x024019D8 = b cave2"));
    assert!(asm.contains("0x02704448 = b cave3"));
    assert!(asm.contains("cave1:"));
    assert!(asm.contains("cave2:"));
    assert!(asm.contains("cave3:"));
    assert!(asm.contains("cave4:"));
    assert!(asm.contains("ble cave4"));
}

#[test]
fn test_conditional_branches_have_no_link_bit() {
    assert_eq!(assemble_at("blt 0x10", 0, &none()).unwrap(), 0x41800010);
    assert_eq!(assemble_at("ble 0x10", 0, &none()).unwrap(), 0x40810010);
    assert_eq!(assemble_at("bltl 0x10", 0, &none()).unwrap(), 0x41800011);
    assert_eq!(assemble_at("blea 0x10", 0, &none()).unwrap(), 0x40810012);
}

#[test]
fn test_relative_branches_use_instruction_address() {
    assert_eq!(assemble_at("b 0x108", 0x100, &none()).unwrap(), 0x48000008);
    assert_eq!(assemble_at("bne 0x108", 0x100, &none()).unwrap(), 0x40820008);
    assert_eq!(assemble_at("b 0x02401994", 0x02DF6C18, &none()).unwrap(), 0x4B60AD7C);
    assert_eq!(assemble_at("bl 0x02013650", 0x02DF6C38, &none()).unwrap(), 0x4B21CA19);
}

#[test]
fn test_branch_range_and_alignment_errors() {
    assert!(assemble_at("bne 0x10000", 0, &none()).is_err());
    assert!(assemble_at("b 0x02000000", 0, &none()).is_err());
    assert!(assemble_at("b 0x102", 0x100, &none()).is_err());
}

#[test]
fn test_missing_operands_are_errors() {
    assert!(assemble_at("addi r3,r4", 0, &none()).unwrap_err().contains("expects 3 operands, got 2"));
    assert!(assemble_at("rlwinm r3,r4,1", 0, &none()).is_err());
    assert!(assemble_at("lwz r3", 0, &none()).is_err());
    assert!(assemble_at("b", 0, &none()).is_err());
    assert!(assemble_at("ble", 0, &none()).is_err());
}

#[test]
fn test_labels_and_relocation_modifiers() {
    let symbols: HashMap<String, u32> = [("cave1".to_string(), 0x02DF_9000u32)].into_iter().collect();
    assert_eq!(assemble_at("b cave1", 0x02DF_8FF0, &symbols).unwrap(), 0x48000010);
    assert_eq!(assemble_at("lis r12,cave1@ha", 0, &symbols).unwrap(), 0x3D8002E0);
    assert_eq!(assemble_at("lis r12,cave1@h", 0, &symbols).unwrap(), 0x3D8002DF);
    assert_eq!(assemble_at("addi r12,r12,cave1@l", 0, &symbols).unwrap(), 0x398C9000);
    assert_eq!(assemble_at("lwz r3,cave1+4@l(r12)", 0, &symbols).unwrap(), 0x806C9004);
    assert_eq!(assemble_at(".int cave1-0x10", 0, &symbols).unwrap(), 0x02DF8FF0);
    assert!(assemble_at("b nowhere", 0, &symbols).unwrap_err().contains("nowhere"));
}

#[test]
fn test_symbolic_immediates_are_masked_like_cemu() {
    let symbols: HashMap<String, u32> = [("target".to_string(), 0x02DF_9000u32)].into_iter().collect();
    let (word, notes) = assemble_with_notes("addi r4,r25,target", 0, &symbols, None).unwrap();
    assert_eq!(word, 0x38999000);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("@ha"));
    let (word, notes) = assemble_with_notes("addi r4,r25,target@l", 0, &symbols, None).unwrap();
    assert_eq!(word, 0x38999000);
    assert!(notes.is_empty());
    assert!(assemble_at("addi r4,r25,0x12345", 0, &symbols).is_err());
}

#[test]
fn test_ori_zero_is_not_mr() {
    assert_eq!(disassemble_instruction(0x60830000, 0), "ori r3,r4,0x0");
    assert_eq!(assemble_instruction("ori r3,r4,0x0").unwrap(), 0x60830000);
}

#[test]
fn test_conditional_blr_with_cr_field() {
    assert_eq!(disassemble_instruction(0x4D850020, 0), "bgtlr cr1");
    assert_eq!(assemble_instruction("bgtlr cr1").unwrap(), 0x4D850020);
}

#[test]
fn test_conditional_branch_with_cr_field_roundtrip() {
    assert_eq!(assemble_at("blt cr7,0x10", 0, &none()).unwrap(), 0x419C0010);
    assert_eq!(disassemble_instruction(0x419C0010, 0), "blt cr7,0x00000010");
}

#[test]
fn test_bdnz_and_bc_roundtrip() {
    assert_eq!(assemble_at("bdnz 0x100", 0x100, &none()).unwrap(), 0x42000000);
    assert_eq!(disassemble_instruction(0x42000000, 0x100), "bdnz 0x00000100");
    assert_eq!(disassemble_instruction(0x42800008, 0x100), "bc 20,0,0x00000108");
    assert_eq!(assemble_at("bc 20,0,0x108", 0x100, &none()).unwrap(), 0x42800008);
}

#[test]
fn test_branch_hint_suffix() {
    assert_eq!(disassemble_instruction(0x41A20008, 0x100), "beq+ 0x00000108");
    assert_eq!(assemble_at("beq+ 0x108", 0x100, &none()).unwrap(), 0x41A20008);
    assert_eq!(assemble_at("beq- 0x108", 0x100, &none()).unwrap(), 0x41A20008);
}

#[test]
fn test_srwi_zero() {
    assert_eq!(assemble_instruction("srwi r3,r4,0").unwrap(), 0x5483003E);
    assert_eq!(assemble_instruction("srwi r3,r4,4").unwrap(), 0x5483E13E);
    assert_eq!(disassemble_instruction(0x5483E13E, 0), "srwi r3,r4,4");
}

#[test]
fn test_lis_prints_unsigned() {
    assert_eq!(disassemble_instruction(0x3C608000, 0), "lis r3,0x8000");
    assert_eq!(assemble_instruction("lis r3,-0x8000").unwrap(), 0x3C608000);
    assert_eq!(assemble_instruction("lis r3,0x8000").unwrap(), 0x3C608000);
}

#[test]
fn test_unknown_words_fall_back_to_int() {
    assert_eq!(disassemble_instruction(0x00000000, 0), ".int 0x00000000");
    assert_eq!(disassemble_instruction(0x2C230000, 0), ".int 0x2C230000");
    assert_eq!(assemble_instruction(".int 0x2C230000").unwrap(), 0x2C230000);
    assert_eq!(assemble_instruction(".long 0x12345678").unwrap(), 0x12345678);
}

#[test]
fn test_label_aware_disassembly() {
    let labels: HashMap<u32, String> = [(0x02DF_6C40u32, "cave4".to_string())].into_iter().collect();
    assert_eq!(disassemble_with(0x4081000C, 0x02DF_6C34, &labels), "ble cave4");
    assert_eq!(disassemble_with(0x48000002 | 0x0000_0040, 0, &[(0x40u32, "cave1".to_string())].into_iter().collect()), "ba cave1");
    assert_eq!(disassemble_for_cemu(0x48000042, 0, &[(0x40u32, "cave1".to_string())].into_iter().collect()), "b cave1");
}

#[test]
fn test_cemu_unsupported_mnemonics_become_int() {
    assert_eq!(disassemble_instruction(0x7C0006AC, 0), "eieio");
    assert_eq!(disassemble_for_cemu(0x7C0006AC, 0, &HashMap::new()), ".int 0x7C0006AC");
}

#[test]
fn test_console_address_translation_is_bounded() {
    assert_eq!(console_addr_to_cemu(0x1012_3456), 0x1012_3456);
    assert_eq!(console_addr_to_cemu(0x0E83_D2E0), 0x0283_D2E0);
    assert_eq!(cemu_addr_to_console(0x1000_0000), 0x1050_3000);
    assert_eq!(cemu_addr_to_console(0x1012_92A8), 0x1062_C2A8);
    assert_eq!(console_addr_to_cemu(0x1062_C2A8), 0x1012_92A8);
    assert_eq!(cemu_addr_to_console(0x0283_D2E0), 0x0E83_D2E0);
}

#[test]
fn test_gecko_convert_addresses() {
    let c2 = "C283D2E0 00000001\n38600001 4E800020";
    assert_eq!(gecko_convert_addresses(c2, true), c2);
    assert_eq!(gecko_convert_addresses("10123456 00000001", true), "10123456 00000001");
    assert_eq!(gecko_convert_addresses("1062C2A8 00000001", true), "101292A8 00000001");
    assert_eq!(gecko_convert_addresses("0E83D2E0 38600001", true), "0283D2E0 38600001");
    assert_eq!(gecko_convert_addresses("0283D2E0 38600001", false), "0E83D2E0 38600001");
    assert_eq!(gecko_convert_addresses("# comment\n\n0E83D2E0 38600001 extra", true), "# comment\n\n0E83D2E0 38600001 extra");
}
