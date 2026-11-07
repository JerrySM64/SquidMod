use std::cell::RefCell;
use std::collections::HashMap;

use super::address::AddressMap;
use super::expr::evaluate;
use super::operands::{parse_creg, parse_freg, parse_reg, split_operands};

#[derive(Clone, Copy)]
pub(super) enum CondForm {
    Disp,
    Lr,
    Ctr,
}

pub(super) struct CondBranch {
    bo: u32,
    bi: u32,
    uses_cr: bool,
    form: CondForm,
    lk: u32,
    aa: u32,
}

pub(super) const BRANCH_CONDITIONS: [(&str, u32, u32, bool); 10] = [
    ("dnz", 16, 0, false),
    ("dz", 18, 0, false),
    ("lt", 12, 0, true),
    ("le", 4, 1, true),
    ("eq", 12, 2, true),
    ("ge", 4, 0, true),
    ("gt", 12, 1, true),
    ("ne", 4, 2, true),
    ("so", 12, 3, true),
    ("ns", 4, 3, true),
];

pub(super) fn parse_cond_branch(mnem: &str) -> Option<CondBranch> {
    let (mnem, hint) = match mnem.strip_suffix('+').or_else(|| mnem.strip_suffix('-')) {
        Some(stripped) => (stripped, true),
        None => (mnem, false),
    };
    let rest = mnem.strip_prefix('b')?;
    let &(name, bo, bi, uses_cr) = BRANCH_CONDITIONS.iter().find(|entry| rest.starts_with(entry.0))?;
    let (form, lk, aa) = match &rest[name.len()..] {
        "" => (CondForm::Disp, 0, 0),
        "l" => (CondForm::Disp, 1, 0),
        "a" => (CondForm::Disp, 0, 1),
        "la" => (CondForm::Disp, 1, 1),
        "lr" => (CondForm::Lr, 0, 0),
        "lrl" => (CondForm::Lr, 1, 0),
        "ctr" if uses_cr => (CondForm::Ctr, 0, 0),
        "ctrl" if uses_cr => (CondForm::Ctr, 1, 0),
        _ => return None,
    };
    if hint && !matches!(form, CondForm::Disp) {
        return None;
    }
    Some(CondBranch { bo: bo | hint as u32, bi, uses_cr, form, lk, aa })
}

pub(super) struct Asm<'a> {
    pub(super) mnem: &'a str,
    pub(super) ops: Vec<&'a str>,
    pub(super) addr: u32,
    pub(super) symbols: &'a HashMap<String, u32>,
    pub(super) literal_targets: Option<&'a AddressMap>,
    pub(super) notes: RefCell<Vec<String>>,
}

impl Asm<'_> {
    pub(super) fn count(&self, min: usize, max: usize) -> Result<(), String> {
        let got = self.ops.len();
        if (min..=max).contains(&got) {
            Ok(())
        } else if min == max {
            Err(format!("{} expects {} operands, got {}", self.mnem, min, got))
        } else {
            Err(format!("{} expects {} to {} operands, got {}", self.mnem, min, max, got))
        }
    }

    pub(super) fn op(&self, i: usize) -> Result<&str, String> {
        self.ops.get(i).copied().ok_or_else(|| format!("{} is missing operand {}", self.mnem, i + 1))
    }

    pub(super) fn unknown(&self) -> String {
        format!("Unknown mnemonic: {}", self.mnem)
    }

    pub(super) fn gpr(&self, i: usize) -> Result<u32, String> {
        parse_reg(self.op(i)?)
    }

    pub(super) fn fpr(&self, i: usize) -> Result<u32, String> {
        parse_freg(self.op(i)?)
    }

    pub(super) fn crf(&self, i: usize) -> Result<u32, String> {
        parse_creg(self.op(i)?)
    }

    pub(super) fn value_of(&self, text: &str) -> Result<i64, String> {
        evaluate(text, self.addr, self.symbols)
    }

    pub(super) fn imm16_of(&self, value: i64, text: &str) -> Result<u32, String> {
        if (-0x8000..=0xFFFF).contains(&value) {
            return Ok((value as u32) & 0xFFFF);
        }
        if evaluate(text, self.addr, &HashMap::new()).is_ok() {
            return Err(format!("{}: immediate `{}` does not fit in 16 bits", self.mnem, text));
        }
        let low = (value as u32) & 0xFFFF;
        let mut note = format!(
            "{}: `{}` is 0x{:08X}, which does not fit in 16 bits; used its low half 0x{:04X} like Cemu does (write `{}@l` to make that explicit)",
            self.mnem, text, value as u32, low, text
        );
        let zero_extends = matches!(self.mnem, "ori" | "oris" | "xori" | "xoris" | "andi." | "andis." | "cmplwi" | "lis");
        if low >= 0x8000 && !zero_extends {
            note.push_str("; the low half is 0x8000 or more and gets sign-extended, so the matching lis needs `@ha`, not `@h`");
        }
        self.notes.borrow_mut().push(note);
        Ok(low)
    }

    pub(super) fn imm16(&self, i: usize) -> Result<u32, String> {
        let text = self.op(i)?;
        self.imm16_of(self.value_of(text)?, text)
    }

    pub(super) fn uimm(&self, i: usize, bits: u32) -> Result<u32, String> {
        let text = self.op(i)?;
        let value = self.value_of(text)?;
        if (0..(1i64 << bits)).contains(&value) {
            Ok(value as u32)
        } else {
            Err(format!("{}: `{}` does not fit in {} unsigned bits", self.mnem, text, bits))
        }
    }

    pub(super) fn d_ra(&self, i: usize) -> Result<(u32, u32), String> {
        let text = self.op(i)?;
        let Some(paren) = text.find('(') else {
            return Err(format!("Expected offset(rN) format, got: {}", text));
        };
        let Some(reg_text) = text[paren + 1..].strip_suffix(')') else {
            return Err(format!("Expected offset(rN) format, got: {}", text));
        };
        let offset_text = text[..paren].trim();
        let d = if offset_text.is_empty() { 0 } else { self.imm16_of(self.value_of(offset_text)?, offset_text)? };
        Ok((d, parse_reg(reg_text)?))
    }

    pub(super) fn branch(&self, i: usize, absolute: bool, bits: u32) -> Result<u32, String> {
        let text = self.op(i)?;
        let target = self.value_of(text)?;
        if !(-(1i64 << 31)..(1i64 << 32)).contains(&target) {
            return Err(format!("{}: branch target `{}` is outside the 32-bit address space", self.mnem, text));
        }
        let target = match self.literal_targets {
            Some(window) if evaluate(text, self.addr, &HashMap::new()).is_ok() => window.to_console(target as u32),
            _ => target as u32,
        };
        let raw = if absolute { target } else { target.wrapping_sub(self.addr) };
        let disp = raw as i32 as i64;
        let limit = 1i64 << (bits - 1);
        if disp & 3 != 0 {
            return Err(format!("{}: branch target `{}` is not 4-byte aligned", self.mnem, text));
        }
        if disp < -limit || disp >= limit {
            return Err(format!(
                "{}: branch target `{}` is out of range from 0x{:08X} (limit ±0x{:X})",
                self.mnem, text, self.addr, limit
            ));
        }
        Ok((disp as u32) & ((1u32 << bits) - 1) & !3)
    }

    pub(super) fn fixed(&self, word: u32) -> Result<u32, String> {
        self.count(0, 0)?;
        Ok(word)
    }

    pub(super) fn rlwinm(&self, rc: u32, sh: u32, mb: u32, me: u32) -> Result<u32, String> {
        let ra = self.gpr(0)?;
        let rs = self.gpr(1)?;
        Ok((21 << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (mb << 6) | (me << 1) | rc)
    }

    pub(super) fn cond_branch(&self, cb: CondBranch) -> Result<u32, String> {
        let base = if matches!(cb.form, CondForm::Disp) { 1 } else { 0 };
        let max = if cb.uses_cr { base + 1 } else { base };
        self.count(base, max)?;
        let (cr, next) = if self.ops.len() > base { (self.crf(0)?, 1) } else { (0, 0) };
        let head = (cb.bo << 21) | (((cr << 2) | cb.bi) << 16) | cb.lk;
        match cb.form {
            CondForm::Disp => Ok((16 << 26) | head | self.branch(next, cb.aa == 1, 16)? | (cb.aa << 1)),
            CondForm::Lr => Ok((19 << 26) | head | (16 << 1)),
            CondForm::Ctr => Ok((19 << 26) | head | (528 << 1)),
        }
    }
}

pub fn assemble_at(line: &str, addr: u32, symbols: &HashMap<String, u32>) -> Result<u32, String> {
    assemble_with_notes(line, addr, symbols, None).map(|(word, _)| word)
}

pub fn assemble_with_notes(
    line: &str,
    addr: u32,
    symbols: &HashMap<String, u32>,
    literal_targets: Option<&AddressMap>,
) -> Result<(u32, Vec<String>), String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty line".into());
    }
    let (mnem, ops_str) = match line.split_once(char::is_whitespace) {
        Some((mnem, rest)) => (mnem, rest.trim()),
        None => (line, ""),
    };
    let mnem = mnem.to_ascii_lowercase();
    let asm = Asm {
        mnem: &mnem,
        ops: split_operands(ops_str),
        addr,
        symbols,
        literal_targets,
        notes: RefCell::new(Vec::new()),
    };
    let word = asm.encode()?;
    Ok((word, asm.notes.into_inner()))
}
