use super::assembler::{parse_cond_branch, Asm};
use super::isa::load_store_opcode;

impl Asm<'_> {
    pub(super) fn encode(&self) -> Result<u32, String> {
        let m = self.mnem;
        let rc = if m.ends_with('.') { 1u32 } else { 0 };
        let base = m.trim_end_matches('.');

        if let Some((opcode, float)) = load_store_opcode(m) {
            self.count(2, 2)?;
            let rt = if float { self.fpr(0)? } else { self.gpr(0)? };
            let (d, ra) = self.d_ra(1)?;
            return Ok((opcode << 26) | (rt << 21) | (ra << 16) | d);
        }

        match m {
            "nop" => self.fixed(0x6000_0000),
            "blr" => self.fixed(0x4E80_0020),
            "blrl" => self.fixed(0x4E80_0021),
            "bctr" => self.fixed(0x4E80_0420),
            "bctrl" => self.fixed(0x4E80_0421),
            "bdnzlr" => self.fixed(0x4E00_0020),
            "bdnzlrl" => self.fixed(0x4E00_0021),
            "bdzlr" => self.fixed(0x4E40_0020),
            "bdzlrl" => self.fixed(0x4E40_0021),
            "sc" => self.fixed(0x4400_0002),
            "sync" => self.fixed(0x7C00_04AC),
            "isync" => self.fixed(0x4C00_012C),
            "eieio" => self.fixed(0x7C00_06AC),

            ".int" | ".uint" | ".long" | ".ptr" | ".u32" => {
                self.count(1, 1)?;
                let text = self.op(0)?;
                let value = self.value_of(text)?;
                if (-(1i64 << 31)..(1i64 << 32)).contains(&value) {
                    Ok(value as u32)
                } else {
                    Err(format!("{}: `{}` does not fit in 32 bits", m, text))
                }
            }
            ".float" => {
                self.count(1, 1)?;
                let text = self.op(0)?;
                text.parse::<f32>().map(f32::to_bits).map_err(|_| format!(".float: invalid number `{}`", text))
            }
            ".word" | ".short" | ".u16" | ".byte" | ".u8" | ".char" | ".string" | ".double" | ".align" => {
                Err(format!("{} does not encode exactly one 32-bit word (Cemu's .word is 16-bit; use .int)", m))
            }

            "li" | "lis" => {
                self.count(2, 2)?;
                let opcode = if m == "li" { 14 } else { 15 };
                Ok((opcode << 26) | (self.gpr(0)? << 21) | self.imm16(1)?)
            }
            "addi" | "addis" | "addic" | "addic." | "subfic" | "mulli" => {
                self.count(3, 3)?;
                let opcode = match m {
                    "addi" => 14,
                    "addis" => 15,
                    "addic" => 12,
                    "addic." => 13,
                    "subfic" => 8,
                    "mulli" => 7,
                    _ => return Err(self.unknown()),
                };
                Ok((opcode << 26) | (self.gpr(0)? << 21) | (self.gpr(1)? << 16) | self.imm16(2)?)
            }
            "subi" => {
                self.count(3, 3)?;
                let text = self.op(2)?;
                let imm = self.imm16_of(-self.value_of(text)?, text)?;
                Ok((14 << 26) | (self.gpr(0)? << 21) | (self.gpr(1)? << 16) | imm)
            }
            "cmpwi" | "cmplwi" => {
                self.count(2, 3)?;
                let opcode = if m == "cmpwi" { 11 } else { 10 };
                let (bf, ra, imm) = if self.ops.len() == 3 {
                    (self.crf(0)?, self.gpr(1)?, self.imm16(2)?)
                } else {
                    (0, self.gpr(0)?, self.imm16(1)?)
                };
                Ok((opcode << 26) | (bf << 23) | (ra << 16) | imm)
            }
            "ori" | "oris" | "xori" | "xoris" | "andi." | "andis." => {
                self.count(3, 3)?;
                let opcode = match m {
                    "ori" => 24,
                    "oris" => 25,
                    "xori" => 26,
                    "xoris" => 27,
                    "andi." => 28,
                    "andis." => 29,
                    _ => return Err(self.unknown()),
                };
                Ok((opcode << 26) | (self.gpr(1)? << 21) | (self.gpr(0)? << 16) | self.imm16(2)?)
            }
            "mr" | "mr." => {
                self.count(2, 2)?;
                let ra = self.gpr(0)?;
                let rs = self.gpr(1)?;
                Ok((31 << 26) | (rs << 21) | (ra << 16) | (rs << 11) | (444 << 1) | rc)
            }

            "rlwinm" | "rlwinm." | "rlwimi" | "rlwimi." => {
                self.count(5, 5)?;
                let opcode = if base == "rlwinm" { 21 } else { 20 };
                let ra = self.gpr(0)?;
                let rs = self.gpr(1)?;
                let sh = self.uimm(2, 5)?;
                let mb = self.uimm(3, 5)?;
                let me = self.uimm(4, 5)?;
                Ok((opcode << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (mb << 6) | (me << 1) | rc)
            }
            "rlwnm" | "rlwnm." => {
                self.count(5, 5)?;
                let ra = self.gpr(0)?;
                let rs = self.gpr(1)?;
                let rb = self.gpr(2)?;
                let mb = self.uimm(3, 5)?;
                let me = self.uimm(4, 5)?;
                Ok((23 << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (mb << 6) | (me << 1) | rc)
            }
            "slwi" | "slwi." | "srwi" | "srwi." | "clrlwi" | "clrlwi." | "clrrwi" | "clrrwi." | "rotlwi" | "rotlwi." => {
                self.count(3, 3)?;
                let n = self.uimm(2, 5)?;
                let (sh, mb, me) = match base {
                    "slwi" => (n, 0, 31 - n),
                    "srwi" if n == 0 => (0, 0, 31),
                    "srwi" => (32 - n, n, 31),
                    "clrlwi" => (0, n, 31),
                    "clrrwi" => (0, 0, 31 - n),
                    "rotlwi" => (n, 0, 31),
                    _ => return Err(self.unknown()),
                };
                self.rlwinm(rc, sh, mb, me)
            }

            "add" | "add." | "addc" | "addc." | "adde" | "adde." |
            "subf" | "subf." | "subfc" | "subfc." | "subfe" | "subfe." |
            "mullw" | "mullw." | "mulhw" | "mulhw." | "mulhwu" | "mulhwu." |
            "divw" | "divw." | "divwu" | "divwu." => {
                self.count(3, 3)?;
                let xo = match base {
                    "add" => 266, "addc" => 10, "adde" => 138,
                    "subf" => 40, "subfc" => 8, "subfe" => 136,
                    "mullw" => 235, "mulhw" => 75, "mulhwu" => 11,
                    "divw" => 491, "divwu" => 459,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.gpr(0)? << 21) | (self.gpr(1)? << 16) | (self.gpr(2)? << 11) | (xo << 1) | rc)
            }
            "and" | "and." | "or" | "or." | "xor" | "xor." |
            "nor" | "nor." | "nand" | "nand." | "eqv" | "eqv." |
            "andc" | "andc." | "orc" | "orc." |
            "slw" | "slw." | "srw" | "srw." | "sraw" | "sraw." => {
                self.count(3, 3)?;
                let xo = match base {
                    "and" => 28, "or" => 444, "xor" => 316,
                    "nor" => 124, "nand" => 476, "eqv" => 284,
                    "andc" => 60, "orc" => 412,
                    "slw" => 24, "srw" => 536, "sraw" => 792,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.gpr(1)? << 21) | (self.gpr(0)? << 16) | (self.gpr(2)? << 11) | (xo << 1) | rc)
            }
            "neg" | "neg." => {
                self.count(2, 2)?;
                Ok((31 << 26) | (self.gpr(0)? << 21) | (self.gpr(1)? << 16) | (104 << 1) | rc)
            }
            "extsb" | "extsb." | "extsh" | "extsh." | "cntlzw" | "cntlzw." => {
                self.count(2, 2)?;
                let xo = match base {
                    "extsb" => 954,
                    "extsh" => 922,
                    "cntlzw" => 26,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.gpr(1)? << 21) | (self.gpr(0)? << 16) | (xo << 1) | rc)
            }
            "srawi" | "srawi." => {
                self.count(3, 3)?;
                Ok((31 << 26) | (self.gpr(1)? << 21) | (self.gpr(0)? << 16) | (self.uimm(2, 5)? << 11) | (824 << 1) | rc)
            }
            "cmpw" | "cmplw" => {
                self.count(2, 3)?;
                let xo = if m == "cmpw" { 0 } else { 32 };
                let (bf, ra, rb) = if self.ops.len() == 3 {
                    (self.crf(0)?, self.gpr(1)?, self.gpr(2)?)
                } else {
                    (0, self.gpr(0)?, self.gpr(1)?)
                };
                Ok((31 << 26) | (bf << 23) | (ra << 16) | (rb << 11) | (xo << 1))
            }

            "lwzx" | "lwzux" | "lbzx" | "lbzux" | "lhzx" | "lhzux" | "lhax" | "lhaux" |
            "stwx" | "stwux" | "stbx" | "stbux" | "sthx" | "sthux" => {
                self.count(3, 3)?;
                let xo = match m {
                    "lwzx" => 23, "lwzux" => 55, "lbzx" => 87, "lbzux" => 119,
                    "lhzx" => 279, "lhzux" => 311, "lhax" => 343, "lhaux" => 375,
                    "stwx" => 151, "stwux" => 183, "stbx" => 215, "stbux" => 247,
                    "sthx" => 407, "sthux" => 439,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.gpr(0)? << 21) | (self.gpr(1)? << 16) | (self.gpr(2)? << 11) | (xo << 1))
            }
            "lfsx" | "lfsux" | "lfdx" | "lfdux" | "stfsx" | "stfsux" | "stfdx" | "stfdux" => {
                self.count(3, 3)?;
                let xo = match m {
                    "lfsx" => 535, "lfsux" => 567, "lfdx" => 599, "lfdux" => 631,
                    "stfsx" => 663, "stfsux" => 695, "stfdx" => 727, "stfdux" => 759,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.fpr(0)? << 21) | (self.gpr(1)? << 16) | (self.gpr(2)? << 11) | (xo << 1))
            }

            "mflr" | "mfctr" | "mfxer" | "mtlr" | "mtctr" | "mtxer" => {
                self.count(1, 1)?;
                let spr = match &m[2..] {
                    "lr" => 8,
                    "ctr" => 9,
                    "xer" => 1,
                    _ => return Err(self.unknown()),
                };
                let xo = if m.starts_with("mf") { 339 } else { 467 };
                Ok((31 << 26) | (self.gpr(0)? << 21) | (spr << 16) | (xo << 1))
            }
            "mfspr" => {
                self.count(2, 2)?;
                let rd = self.gpr(0)?;
                let spr = self.uimm(1, 10)?;
                Ok((31 << 26) | (rd << 21) | ((spr & 0x1F) << 16) | ((spr >> 5) << 11) | (339 << 1))
            }
            "mtspr" => {
                self.count(2, 2)?;
                let spr = self.uimm(0, 10)?;
                let rs = self.gpr(1)?;
                Ok((31 << 26) | (rs << 21) | ((spr & 0x1F) << 16) | ((spr >> 5) << 11) | (467 << 1))
            }
            "mfcr" => {
                self.count(1, 1)?;
                Ok((31 << 26) | (self.gpr(0)? << 21) | (19 << 1))
            }
            "mtcrf" => {
                self.count(2, 2)?;
                let fxm = self.uimm(0, 8)?;
                Ok((31 << 26) | (self.gpr(1)? << 21) | (fxm << 12) | (144 << 1))
            }
            "mfmsr" | "mtmsr" => {
                self.count(1, 1)?;
                let xo = if m == "mfmsr" { 83 } else { 146 };
                Ok((31 << 26) | (self.gpr(0)? << 21) | (xo << 1))
            }
            "dcbf" | "dcbt" | "dcbtst" => {
                self.count(2, 2)?;
                let xo = match m {
                    "dcbf" => 86,
                    "dcbt" => 278,
                    "dcbtst" => 246,
                    _ => return Err(self.unknown()),
                };
                Ok((31 << 26) | (self.gpr(0)? << 16) | (self.gpr(1)? << 11) | (xo << 1))
            }

            "b" | "bl" | "ba" | "bla" => {
                self.count(1, 1)?;
                let aa = if m.ends_with('a') { 1u32 } else { 0 };
                let lk = if m == "bl" || m == "bla" { 1u32 } else { 0 };
                Ok((18 << 26) | self.branch(0, aa == 1, 26)? | (aa << 1) | lk)
            }
            "bc" | "bcl" | "bca" | "bcla" => {
                self.count(3, 3)?;
                let aa = if m.ends_with('a') { 1u32 } else { 0 };
                let lk = if m == "bcl" || m == "bcla" { 1u32 } else { 0 };
                let bo = self.uimm(0, 5)?;
                let bi = self.uimm(1, 5)?;
                Ok((16 << 26) | (bo << 21) | (bi << 16) | self.branch(2, aa == 1, 16)? | (aa << 1) | lk)
            }
            "bclr" | "bclrl" | "bcctr" | "bcctrl" => {
                self.count(2, 2)?;
                let lk = if m.ends_with('l') { 1u32 } else { 0 };
                let xo = if m.starts_with("bclr") { 16 } else { 528 };
                let bo = self.uimm(0, 5)?;
                let bi = self.uimm(1, 5)?;
                Ok((19 << 26) | (bo << 21) | (bi << 16) | (xo << 1) | lk)
            }

            "crand" | "cror" | "crxor" | "crnand" | "crnor" | "creqv" | "crandc" | "crorc" => {
                self.count(3, 3)?;
                let xo = match m {
                    "crand" => 257, "cror" => 449, "crxor" => 193, "crnand" => 225,
                    "crnor" => 33, "creqv" => 289, "crandc" => 129, "crorc" => 417,
                    _ => return Err(self.unknown()),
                };
                Ok((19 << 26) | (self.uimm(0, 5)? << 21) | (self.uimm(1, 5)? << 16) | (self.uimm(2, 5)? << 11) | (xo << 1))
            }
            "mcrf" => {
                self.count(2, 2)?;
                Ok((19 << 26) | (self.crf(0)? << 23) | (self.crf(1)? << 18))
            }

            "fadd" | "fadd." | "fadds" | "fadds." |
            "fsub" | "fsub." | "fsubs" | "fsubs." |
            "fdiv" | "fdiv." | "fdivs" | "fdivs." => {
                self.count(3, 3)?;
                let single = base.ends_with('s');
                let primary = if single { 59 } else { 63 };
                let xo = match base.trim_end_matches('s') {
                    "fadd" => 21,
                    "fsub" => 20,
                    "fdiv" => 18,
                    _ => return Err(self.unknown()),
                };
                Ok((primary << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(2)? << 11) | (xo << 1) | rc)
            }
            "fmul" | "fmul." | "fmuls" | "fmuls." => {
                self.count(3, 3)?;
                let primary = if base.ends_with('s') { 59 } else { 63 };
                Ok((primary << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(2)? << 6) | (25 << 1) | rc)
            }
            "fmadd" | "fmadd." | "fmadds" | "fmadds." |
            "fmsub" | "fmsub." | "fmsubs" | "fmsubs." => {
                self.count(4, 4)?;
                let primary = if base.ends_with('s') { 59 } else { 63 };
                let xo = match base.trim_end_matches('s') {
                    "fmadd" => 29,
                    "fmsub" => 28,
                    _ => return Err(self.unknown()),
                };
                Ok((primary << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(3)? << 11) | (self.fpr(2)? << 6) | (xo << 1) | rc)
            }
            "fneg" | "fneg." | "fabs" | "fabs." | "fmr" | "fmr." | "fnabs" | "fnabs." |
            "frsp" | "frsp." | "fctiwz" | "fctiwz." => {
                self.count(2, 2)?;
                let xo = match base {
                    "fneg" => 40, "fabs" => 264, "fmr" => 72, "fnabs" => 136,
                    "frsp" => 12, "fctiwz" => 15,
                    _ => return Err(self.unknown()),
                };
                Ok((63 << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 11) | (xo << 1) | rc)
            }
            "fcmpu" | "fcmpo" => {
                self.count(3, 3)?;
                let xo = if m == "fcmpu" { 0 } else { 32 };
                Ok((63 << 26) | (self.crf(0)? << 23) | (self.fpr(1)? << 16) | (self.fpr(2)? << 11) | (xo << 1))
            }

            "ps_add" | "ps_add." | "ps_sub" | "ps_sub." | "ps_div" | "ps_div." => {
                self.count(3, 3)?;
                let xo = match base {
                    "ps_add" => 21,
                    "ps_sub" => 20,
                    "ps_div" => 18,
                    _ => return Err(self.unknown()),
                };
                Ok((4 << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(2)? << 11) | (xo << 1) | rc)
            }
            "ps_mul" | "ps_mul." => {
                self.count(3, 3)?;
                Ok((4 << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(2)? << 6) | (25 << 1) | rc)
            }
            "ps_madd" | "ps_madd." | "ps_msub" | "ps_msub." => {
                self.count(4, 4)?;
                let xo = if base == "ps_madd" { 29 } else { 28 };
                Ok((4 << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(3)? << 11) | (self.fpr(2)? << 6) | (xo << 1) | rc)
            }
            "ps_merge00" | "ps_merge00." | "ps_merge01" | "ps_merge01." |
            "ps_merge10" | "ps_merge10." | "ps_merge11" | "ps_merge11." => {
                self.count(3, 3)?;
                let xo = match base {
                    "ps_merge00" => 528,
                    "ps_merge01" => 560,
                    "ps_merge10" => 592,
                    "ps_merge11" => 624,
                    _ => return Err(self.unknown()),
                };
                Ok((4 << 26) | (self.fpr(0)? << 21) | (self.fpr(1)? << 16) | (self.fpr(2)? << 11) | (xo << 1) | rc)
            }

            _ => match parse_cond_branch(m) {
                Some(cb) => self.cond_branch(cb),
                None => Err(self.unknown()),
            },
        }
    }
}
