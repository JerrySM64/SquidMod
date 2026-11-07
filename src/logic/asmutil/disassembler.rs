use std::collections::HashMap;

use super::assembler::assemble_at;
use super::dis_float::{dis_opcode59, dis_opcode63, dis_paired};
use super::dis_integer::{
    dis_addi, dis_addic, dis_addic_dot, dis_addis, dis_andi_dot, dis_andis_dot, dis_cmpi, dis_cmpli, dis_load_store_imm, dis_mulli,
    dis_opcode31, dis_ori, dis_oris, dis_rlwimi, dis_rlwinm, dis_rlwnm, dis_subfic, dis_xori, dis_xoris,
};
use super::disassemble_instruction;
use super::isa::{branch_target, cond_name, load_store_mnemonic, raw_word};

pub(super) const CEMU_UNSUPPORTED: [&str; 12] = [
    "bclr", "bclrl", "bcctr", "bcctrl", "mfmsr", "mtmsr", "eieio", "dcbf", "dcbt", "dcbtst", "fnabs", "fnabs.",
];

pub(super) struct Dis<'a> {
    addr: u32,
    labels: &'a HashMap<u32, String>,
    relative_labels: bool,
}

impl Dis<'_> {
    fn target(&self, target: u32) -> String {
        match self.labels.get(&target) {
            Some(label) => label.clone(),
            None => format!("0x{:08X}", target),
        }
    }

    fn absolute_suffix(&self, absolute: bool, target: u32) -> &'static str {
        if absolute && !(self.relative_labels && self.labels.contains_key(&target)) {
            "a"
        } else {
            ""
        }
    }
}

pub fn disassemble_with(word: u32, addr: u32, labels: &HashMap<u32, String>) -> String {
    let canonical = disassemble_raw(word, &Dis { addr, labels, relative_labels: false });
    if canonical.starts_with(".int") {
        return canonical;
    }
    let symbols: HashMap<String, u32> = labels.iter().map(|(&a, name)| (name.clone(), a)).collect();
    match assemble_at(&canonical, addr, &symbols) {
        Ok(encoded) if encoded == word => canonical,
        _ => raw_word(word),
    }
}

pub fn disassemble_for_cemu(word: u32, addr: u32, labels: &HashMap<u32, String>) -> String {
    let canonical = disassemble_with(word, addr, labels);
    let mnemonic = canonical.split_whitespace().next().unwrap_or("");
    if canonical.starts_with(".int") || CEMU_UNSUPPORTED.contains(&mnemonic) {
        return raw_word(word);
    }
    disassemble_raw(word, &Dis { addr, labels, relative_labels: true })
}

pub fn disassemble_with_imm(word: u32, imm: &str) -> Option<String> {
    if disassemble_instruction(word, 0).starts_with(".int") {
        return None;
    }
    let opcode = word >> 26;
    let rt = (word >> 21) & 0x1F;
    let ra = (word >> 16) & 0x1F;
    match opcode {
        14 if ra != 0 => Some(format!("addi r{},r{},{}", rt, ra, imm)),
        15 if ra == 0 => Some(format!("lis r{},{}", rt, imm)),
        24 => Some(format!("ori r{},r{},{}", ra, rt, imm)),
        _ => load_store_mnemonic(opcode).map(|(name, float)| {
            format!("{} {}{},{}(r{})", name, if float { 'f' } else { 'r' }, rt, imm, ra)
        }),
    }
}

pub(super) fn disassemble_raw(word: u32, dis: &Dis) -> String {
    let opcode = (word >> 26) & 0x3F;
    match opcode {
        4 => dis_paired(word),
        14 => dis_addi(word),
        15 => dis_addis(word),
        12 => dis_addic(word),
        13 => dis_addic_dot(word),
        7 => dis_mulli(word),
        8 => dis_subfic(word),
        10 => dis_cmpli(word),
        11 => dis_cmpi(word),
        24 => dis_ori(word),
        25 => dis_oris(word),
        26 => dis_xori(word),
        27 => dis_xoris(word),
        28 => dis_andi_dot(word),
        29 => dis_andis_dot(word),
        21 => dis_rlwinm(word),
        20 => dis_rlwimi(word),
        23 => dis_rlwnm(word),
        32..=55 => match load_store_mnemonic(opcode) {
            Some((name, float)) => dis_load_store_imm(word, name, float),
            None => raw_word(word),
        },
        18 => dis_branch(word, dis),
        16 => dis_bc(word, dis),
        31 => dis_opcode31(word),
        19 => dis_opcode19(word),
        59 => dis_opcode59(word),
        63 => dis_opcode63(word),
        17 if word == 0x4400_0002 => "sc".to_string(),
        _ => raw_word(word),
    }
}

pub(super) fn dis_branch(w: u32, dis: &Dis) -> String {
    let Some((target, absolute)) = branch_target(w, dis.addr) else { return raw_word(w) };
    let lk = if w & 1 == 1 { "l" } else { "" };
    format!("b{}{} {}", lk, dis.absolute_suffix(absolute, target), dis.target(target))
}

pub(super) fn dis_bc(w: u32, dis: &Dis) -> String {
    let Some((target, absolute)) = branch_target(w, dis.addr) else { return raw_word(w) };
    let bo = (w >> 21) & 0x1F; let bi = (w >> 16) & 0x1F;
    let lk = if w & 1 == 1 { "l" } else { "" };
    let aa = dis.absolute_suffix(absolute, target);
    let target_text = dis.target(target);
    let hint = if bo & 1 == 1 { "+" } else { "" };
    if bi == 0 && ((bo & !1) == 16 || (bo & !1) == 18) {
        let base = if (bo & !1) == 16 { "bdnz" } else { "bdz" };
        return format!("{}{}{}{} {}", base, lk, aa, hint, target_text);
    }
    if let Some((cond, _)) = cond_name(bo, bi & 3) {
        let cr = if bi >> 2 != 0 { format!("cr{},", bi >> 2) } else { String::new() };
        return format!("b{}{}{}{} {}{}", cond, lk, aa, hint, cr, target_text);
    }
    format!("bc{}{} {},{},{}", lk, aa, bo, bi, target_text)
}

pub(super) fn dis_cr_op(w: u32, name: &str) -> String {
    let bt = (w >> 21) & 0x1F; let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F;
    format!("{} {},{},{}", name, bt, ba, bb)
}

pub(super) fn dis_opcode19(w: u32) -> String {
    let xo = (w >> 1) & 0x3FF;
    let bo = (w >> 21) & 0x1F; let bi = (w >> 16) & 0x1F;
    let lk = if w & 1 != 0 { "l" } else { "" };
    match xo {
        16 => {
            if bo == 20 && bi == 0 { return format!("blr{}", lk); }
            if bo == 16 && bi == 0 { return format!("bdnzlr{}", lk); }
            if bo == 18 && bi == 0 { return format!("bdzlr{}", lk); }
            if let Some((cond, false)) = cond_name(bo, bi & 3) {
                return if bi >> 2 != 0 {
                    format!("b{}lr{} cr{}", cond, lk, bi >> 2)
                } else {
                    format!("b{}lr{}", cond, lk)
                };
            }
            format!("bclr{} {},{}", lk, bo, bi)
        }
        528 => {
            if bo == 20 && bi == 0 { return format!("bctr{}", lk); }
            format!("bcctr{} {},{}", lk, bo, bi)
        }
        257 => dis_cr_op(w, "crand"),
        449 => dis_cr_op(w, "cror"),
        193 => dis_cr_op(w, "crxor"),
        225 => dis_cr_op(w, "crnand"),
        33 => dis_cr_op(w, "crnor"),
        289 => dis_cr_op(w, "creqv"),
        129 => dis_cr_op(w, "crandc"),
        417 => dis_cr_op(w, "crorc"),
        0 => { let bf = (w >> 23) & 7; let bfa = (w >> 18) & 7; format!("mcrf cr{},cr{}", bf, bfa) }
        150 => "isync".into(),
        _ => raw_word(w),
    }
}

