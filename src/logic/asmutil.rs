fn sign_extend(val: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((val << shift) as i32) >> shift
}

fn spr_name(spr: u32) -> Option<&'static str> {
    match spr {
        8 => Some("lr"),
        9 => Some("ctr"),
        1 => Some("xer"),
        _ => None,
    }
}



fn cr_cond(bo: u32, bi_field: u32) -> Option<&'static str> {
    match (bo, bi_field) {
        (12, 0) => Some("lt"),
        (4, 0) => Some("ge"),
        (12, 1) => Some("gt"),
        (4, 1) => Some("le"),
        (12, 2) => Some("eq"),
        (4, 2) => Some("ne"),
        (12, 3) => Some("so"),
        (4, 3) => Some("ns"),
        _ => None,
    }
}

pub fn disassemble_instruction(word: u32) -> String {
    let opcode = (word >> 26) & 0x3F;
    match opcode {
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
        32 => dis_load_store_imm(word, "lwz"),
        33 => dis_load_store_imm(word, "lwzu"),
        34 => dis_load_store_imm(word, "lbz"),
        35 => dis_load_store_imm(word, "lbzu"),
        40 => dis_load_store_imm(word, "lhz"),
        41 => dis_load_store_imm(word, "lhzu"),
        42 => dis_load_store_imm(word, "lha"),
        43 => dis_load_store_imm(word, "lhau"),
        36 => dis_load_store_imm(word, "stw"),
        37 => dis_load_store_imm(word, "stwu"),
        38 => dis_load_store_imm(word, "stb"),
        39 => dis_load_store_imm(word, "stbu"),
        44 => dis_load_store_imm(word, "sth"),
        45 => dis_load_store_imm(word, "sthu"),
        48 => dis_load_store_imm(word, "lfs"),
        49 => dis_load_store_imm(word, "lfsu"),
        50 => dis_load_store_imm(word, "lfd"),
        51 => dis_load_store_imm(word, "lfdu"),
        52 => dis_load_store_imm(word, "stfs"),
        53 => dis_load_store_imm(word, "stfsu"),
        54 => dis_load_store_imm(word, "stfd"),
        55 => dis_load_store_imm(word, "stfdu"),
        18 => dis_branch(word),
        16 => dis_bc(word),
        31 => dis_opcode31(word),
        19 => dis_opcode19(word),
        59 => dis_opcode59(word),
        63 => dis_opcode63(word),
        17 => { if (word >> 1) & 1 == 1 { "sc".to_string() } else { format!(".long 0x{:08X}", word) } },
        _ => format!(".long 0x{:08X}", word),
    }
}

fn dis_addi(w: u32) -> String {
    let rd = (w >> 21) & 0x1F;
    let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    if ra == 0 { format!("li r{},{}", rd, imm) }
    else if imm < 0 { format!("subi r{},r{},{}", rd, ra, -imm) }
    else { format!("addi r{},r{},{}", rd, ra, imm) }
}

fn dis_addis(w: u32) -> String {
    let rd = (w >> 21) & 0x1F;
    let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    if ra == 0 { format!("lis r{},{}", rd, imm) }
    else { format!("addis r{},r{},{}", rd, ra, imm) }
}

fn dis_addic(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("addic r{},r{},{}", rd, ra, imm)
}

fn dis_addic_dot(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("addic. r{},r{},{}", rd, ra, imm)
}

fn dis_mulli(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("mulli r{},r{},{}", rd, ra, imm)
}

fn dis_subfic(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("subfic r{},r{},{}", rd, ra, imm)
}

fn dis_cmpi(w: u32) -> String {
    let bf = (w >> 23) & 0x7; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    if bf == 0 { format!("cmpwi r{},{}", ra, imm) }
    else { format!("cmpwi cr{},r{},{}", bf, ra, imm) }
}

fn dis_cmpli(w: u32) -> String {
    let bf = (w >> 23) & 0x7; let ra = (w >> 16) & 0x1F;
    let imm = w & 0xFFFF;
    if bf == 0 { format!("cmplwi r{},{}", ra, imm) }
    else { format!("cmplwi cr{},r{},{}", bf, ra, imm) }
}

fn dis_ori(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    if rs == 0 && ra == 0 && imm == 0 { "nop".to_string() }
    else if imm == 0 { format!("mr r{},r{}", ra, rs) }
    else { format!("ori r{},r{},{}", ra, rs, imm) }
}

fn dis_oris(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("oris r{},r{},{}", ra, rs, imm)
}

fn dis_xori(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("xori r{},r{},{}", ra, rs, imm)
}

fn dis_xoris(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("xoris r{},r{},{}", ra, rs, imm)
}

fn dis_andi_dot(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("andi. r{},r{},{}", ra, rs, imm)
}

fn dis_andis_dot(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("andis. r{},r{},{}", ra, rs, imm)
}

fn dis_rlwinm(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let sh = (w >> 11) & 0x1F; let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1;
    let dot = if rc != 0 { "." } else { "" };
    if mb == 0 && me == 31 - sh { return format!("slwi{} r{},r{},{}", dot, ra, rs, sh); }
    if me == 31 && mb == 32 - sh && sh != 0 { return format!("srwi{} r{},r{},{}", dot, ra, rs, 32 - sh); }
    if sh == 0 && me == 31 { return format!("clrlwi{} r{},r{},{}", dot, ra, rs, mb); }
    if sh == 0 && mb == 0 { return format!("clrrwi{} r{},r{},{}", dot, ra, rs, 31 - me); }
    if mb == 0 && me == 31 { return format!("rotlwi{} r{},r{},{}", dot, ra, rs, sh); }
    format!("rlwinm{} r{},r{},{},{},{}", dot, ra, rs, sh, mb, me)
}

fn dis_rlwimi(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let sh = (w >> 11) & 0x1F; let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    format!("rlwimi{} r{},r{},{},{},{}", dot, ra, rs, sh, mb, me)
}

fn dis_rlwnm(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let rb = (w >> 11) & 0x1F;
    let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    format!("rlwnm{} r{},r{},r{},{},{}", dot, ra, rs, rb, mb, me)
}

fn dis_load_store_imm(w: u32, mnem: &str) -> String {
    let rt = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let d = sign_extend(w & 0xFFFF, 16);
    if mnem.starts_with('l') && mnem.contains('f') || mnem.starts_with('s') && mnem.contains('f') {
        format!("{} f{},{}(r{})", mnem, rt, d, ra)
    } else {
        format!("{} r{},{}(r{})", mnem, rt, d, ra)
    }
}

fn dis_branch(w: u32) -> String {
    let li = sign_extend(w & 0x03FFFFFC, 26);
    let aa = (w >> 1) & 1; let lk = w & 1;
    let mnem = match (aa, lk) {
        (0, 0) => "b", (0, 1) => "bl", (1, 0) => "ba", (1, 1) => "bla", _ => "b",
    };
    format!("{} 0x{:X}", mnem, li)
}

fn dis_bc(w: u32) -> String {
    let bo = (w >> 21) & 0x1F; let bi = (w >> 16) & 0x1F;
    let bd = sign_extend(w & 0xFFFC, 16);
    let aa = (w >> 1) & 1; let lk = w & 1;
    let cr_field = bi >> 2; let bi_field = bi & 3;
    if bo == 16 && bi == 0 {
        let s = if lk != 0 { "bdnzl" } else { "bdnz" };
        return if aa != 0 { format!("{}a 0x{:X}", s, bd) } else { format!("{} 0x{:X}", s, bd) };
    }
    if bo == 18 && bi == 0 {
        let s = if lk != 0 { "bdzl" } else { "bdz" };
        return if aa != 0 { format!("{}a 0x{:X}", s, bd) } else { format!("{} 0x{:X}", s, bd) };
    }
    if let Some(cond) = cr_cond(bo, bi_field) {
        let cr_s = if cr_field != 0 { format!("cr{},", cr_field) } else { String::new() };
        let lk_s = if lk != 0 { "l" } else { "" };
        let aa_s = if aa != 0 { "a" } else { "" };
        return format!("b{}{}{} {}0x{:X}", cond, lk_s, aa_s, cr_s, bd);
    }
    let lk_s = if lk != 0 { "l" } else { "" }; let aa_s = if aa != 0 { "a" } else { "" };
    format!("bc{}{} {},{},0x{:X}", lk_s, aa_s, bo, bi, bd)
}

fn dis_opcode19(w: u32) -> String {
    let xo = (w >> 1) & 0x3FF;
    let bo = (w >> 21) & 0x1F; let bi = (w >> 16) & 0x1F; let lk = w & 1;
    match xo {
        16 => {
            if bo == 20 && bi == 0 { return if lk != 0 { "blrl".into() } else { "blr".into() }; }
            let bi_field = bi & 3; let cr_field = bi >> 2;
            if let Some(cond) = cr_cond(bo, bi_field) {
                let cr_s = if cr_field != 0 { format!("cr{},", cr_field) } else { String::new() };
                let lk_s = if lk != 0 { "l" } else { "" };
                return format!("b{}lr{} {}", cond, lk_s, cr_s).trim().to_string();
            }
            format!("bclr{} {},{}", if lk != 0 { "l" } else { "" }, bo, bi)
        }
        528 => {
            if bo == 20 && bi == 0 { return if lk != 0 { "bctrl".into() } else { "bctr".into() }; }
            format!("bcctr{} {},{}", if lk != 0 { "l" } else { "" }, bo, bi)
        }
        257 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crand {},{},{}", bt, ba, bb) }
        449 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("cror {},{},{}", bt, ba, bb) }
        193 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crxor {},{},{}", bt, ba, bb) }
        225 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crnand {},{},{}", bt, ba, bb) }
        33  => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crnor {},{},{}", bt, ba, bb) }
        289 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("creqv {},{},{}", bt, ba, bb) }
        129 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crandc {},{},{}", bt, ba, bb) }
        417 => { let ba = (w >> 16) & 0x1F; let bb = (w >> 11) & 0x1F; let bt = (w >> 21) & 0x1F; format!("crorc {},{},{}", bt, ba, bb) }
        0 => { let bf = (w >> 23) & 7; let bfa = (w >> 18) & 7; format!("mcrf cr{},cr{}", bf, bfa) }
        150 => "isync".into(),
        _ => format!(".long 0x{:08X}", w),
    }
}

fn dis_opcode31(w: u32) -> String {
    let xo = (w >> 1) & 0x3FF;
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let rb = (w >> 11) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    match xo {
        266 => format!("add{} r{},r{},r{}", dot, rd, ra, rb),
        10  => format!("addc{} r{},r{},r{}", dot, rd, ra, rb),
        138 => format!("adde{} r{},r{},r{}", dot, rd, ra, rb),
        40  => format!("subf{} r{},r{},r{}", dot, rd, ra, rb),
        8   => format!("subfc{} r{},r{},r{}", dot, rd, ra, rb),
        136 => format!("subfe{} r{},r{},r{}", dot, rd, ra, rb),
        104 => format!("neg{} r{},r{}", dot, rd, ra),
        235 => format!("mullw{} r{},r{},r{}", dot, rd, ra, rb),
        491 => format!("divw{} r{},r{},r{}", dot, rd, ra, rb),
        459 => format!("divwu{} r{},r{},r{}", dot, rd, ra, rb),
        28  => format!("and{} r{},r{},r{}", dot, ra, rd, rb),
        444 => {
            if rd == rb { format!("mr{} r{},r{}", dot, ra, rd) }
            else { format!("or{} r{},r{},r{}", dot, ra, rd, rb) }
        }
        316 => format!("xor{} r{},r{},r{}", dot, ra, rd, rb),
        124 => format!("nor{} r{},r{},r{}", dot, ra, rd, rb),
        476 => format!("nand{} r{},r{},r{}", dot, ra, rd, rb),
        284 => format!("eqv{} r{},r{},r{}", dot, ra, rd, rb),
        60  => format!("andc{} r{},r{},r{}", dot, ra, rd, rb),
        412 => format!("orc{} r{},r{},r{}", dot, ra, rd, rb),
        954 => format!("extsb{} r{},r{}", dot, ra, rd),
        922 => format!("extsh{} r{},r{}", dot, ra, rd),
        26  => format!("cntlzw{} r{},r{}", dot, ra, rd),
        24  => format!("slw{} r{},r{},r{}", dot, ra, rd, rb),
        536 => format!("srw{} r{},r{},r{}", dot, ra, rd, rb),
        792 => format!("sraw{} r{},r{},r{}", dot, ra, rd, rb),
        824 => { let sh = rb; format!("srawi{} r{},r{},{}", dot, ra, rd, sh) }
        0   => {
            let bf = (w >> 23) & 7;
            if bf == 0 { format!("cmpw r{},r{}", ra, rb) }
            else { format!("cmpw cr{},r{},r{}", bf, ra, rb) }
        }
        32  => {
            let bf = (w >> 23) & 7;
            if bf == 0 { format!("cmplw r{},r{}", ra, rb) }
            else { format!("cmplw cr{},r{},r{}", bf, ra, rb) }
        }
        23  => format!("lwzx r{},r{},r{}", rd, ra, rb),
        87  => format!("lbzx r{},r{},r{}", rd, ra, rb),
        279 => format!("lhzx r{},r{},r{}", rd, ra, rb),
        343 => format!("lhax r{},r{},r{}", rd, ra, rb),
        151 => format!("stwx r{},r{},r{}", rd, ra, rb),
        215 => format!("stbx r{},r{},r{}", rd, ra, rb),
        407 => format!("sthx r{},r{},r{}", rd, ra, rb),
        339 => {
            let spr = ((w >> 16) & 0x1F) | (((w >> 11) & 0x1F) << 5);
            if let Some(name) = spr_name(spr) { format!("mf{} r{}", name, rd) }
            else { format!("mfspr r{},{}", rd, spr) }
        }
        467 => {
            let spr = ((w >> 16) & 0x1F) | (((w >> 11) & 0x1F) << 5);
            if let Some(name) = spr_name(spr) { format!("mt{} r{}", name, rd) }
            else { format!("mtspr {},r{}", spr, rd) }
        }
        19  => format!("mfcr r{}", rd),
        144 => { let fxm = (w >> 12) & 0xFF; format!("mtcrf {},r{}", fxm, rd) }
        83  => format!("mfmsr r{}", rd),
        146 => format!("mtmsr r{}", rd),
        598 => "sync".into(),
        854 => "eieio".into(),
        86  => format!("dcbf r{},r{}", ra, rb),
        246 => format!("dcbtst r{},r{}", ra, rb),
        278 => format!("dcbt r{},r{}", ra, rb),
        _ => format!(".long 0x{:08X}", w),
    }
}

fn dis_opcode59(w: u32) -> String {
    let xo = (w >> 1) & 0x1F;
    let frd = (w >> 21) & 0x1F; let fra = (w >> 16) & 0x1F;
    let frb = (w >> 11) & 0x1F; let frc = (w >> 6) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    match xo {
        21 => format!("fadds{} f{},f{},f{}", dot, frd, fra, frb),
        20 => format!("fsubs{} f{},f{},f{}", dot, frd, fra, frb),
        25 => format!("fmuls{} f{},f{},f{}", dot, frd, fra, frc),
        18 => format!("fdivs{} f{},f{},f{}", dot, frd, fra, frb),
        29 => format!("fmadds{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
        28 => format!("fmsubs{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
        _ => format!(".long 0x{:08X}", w),
    }
}

fn dis_opcode63(w: u32) -> String {
    let xo_full = (w >> 1) & 0x3FF;
    let xo_short = (w >> 1) & 0x1F;
    let frd = (w >> 21) & 0x1F; let fra = (w >> 16) & 0x1F;
    let frb = (w >> 11) & 0x1F; let frc = (w >> 6) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    match xo_full {
        0   => { let bf = (w >> 23) & 7; format!("fcmpu cr{},f{},f{}", bf, fra, frb) }
        32  => { let bf = (w >> 23) & 7; format!("fcmpo cr{},f{},f{}", bf, fra, frb) }
        40  => format!("fneg{} f{},f{}", dot, frd, frb),
        264 => format!("fabs{} f{},f{}", dot, frd, frb),
        72  => format!("fmr{} f{},f{}", dot, frd, frb),
        136 => format!("fnabs{} f{},f{}", dot, frd, frb),
        _ => {
            match xo_short {
                21 => format!("fadd{} f{},f{},f{}", dot, frd, fra, frb),
                20 => format!("fsub{} f{},f{},f{}", dot, frd, fra, frb),
                25 => format!("fmul{} f{},f{},f{}", dot, frd, fra, frc),
                18 => format!("fdiv{} f{},f{},f{}", dot, frd, fra, frb),
                29 => format!("fmadd{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
                28 => format!("fmsub{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
                _ => format!(".long 0x{:08X}", w),
            }
        }
    }
}

fn parse_reg(s: &str) -> Result<u32, String> {
    let s = s.trim().trim_end_matches(',');
    if let Some(n) = s.strip_prefix('r') {
        n.parse::<u32>().map_err(|_| format!("Invalid register: {}", s))
            .and_then(|v| if v < 32 { Ok(v) } else { Err(format!("Register out of range: {}", s)) })
    } else {
        Err(format!("Expected register, got: {}", s))
    }
}

fn parse_freg(s: &str) -> Result<u32, String> {
    let s = s.trim().trim_end_matches(',');
    if let Some(n) = s.strip_prefix('f') {
        n.parse::<u32>().map_err(|_| format!("Invalid FP register: {}", s))
            .and_then(|v| if v < 32 { Ok(v) } else { Err(format!("FP register out of range: {}", s)) })
    } else {
        Err(format!("Expected FP register, got: {}", s))
    }
}

fn parse_creg(s: &str) -> Result<u32, String> {
    let s = s.trim().trim_end_matches(',');
    if let Some(n) = s.strip_prefix("cr") {
        n.parse::<u32>().map_err(|_| format!("Invalid CR field: {}", s))
            .and_then(|v| if v < 8 { Ok(v) } else { Err(format!("CR field out of range: {}", s)) })
    } else {
        Err(format!("Expected CR field, got: {}", s))
    }
}

fn parse_imm(s: &str) -> Result<i64, String> {
    let s = s.trim().trim_end_matches(',');
    if s.starts_with("0x") || s.starts_with("0X") {
        let hex = &s[2..];
        if let Ok(v) = u64::from_str_radix(hex, 16) {
            return Ok(v as i64);
        }
        Err(format!("Invalid hex: {}", s))
    } else if s.starts_with("-0x") || s.starts_with("-0X") {
        let hex = &s[3..];
        u64::from_str_radix(hex, 16)
            .map(|v| -(v as i64))
            .map_err(|_| format!("Invalid hex: {}", s))
    } else if s.starts_with('-') {
        s.parse::<i64>().map_err(|_| format!("Invalid immediate: {}", s))
    } else if let Ok(v) = s.parse::<u64>() {
        Ok(v as i64)
    } else {
        s.parse::<i64>().map_err(|_| format!("Invalid immediate: {}", s))
    }
}

fn parse_uimm(s: &str) -> Result<u32, String> {
    parse_imm(s).map(|v| v as u32)
}

fn parse_d_ra(s: &str) -> Result<(i16, u32), String> {
    let s = s.trim();
    if let Some(paren) = s.find('(') {
        let offset_str = &s[..paren];
        let reg_str = s[paren+1..].trim_end_matches(')');
        let offset = parse_imm(offset_str)? as i16;
        let ra = parse_reg(reg_str)?;
        Ok((offset, ra))
    } else {
        Err(format!("Expected offset(rN) format, got: {}", s))
    }
}

pub fn assemble_instruction(line: &str) -> Result<u32, String> {
    let line = line.trim();
    if line.is_empty() {
        return Err("empty or comment".into());
    }

    if let Some(stripped) = line.strip_prefix(".long") {
        let val_str = stripped.trim();
        return parse_uimm(val_str);
    }
    let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
    let mnem = parts[0].to_lowercase();
    let ops_str = if parts.len() > 1 { parts[1].trim() } else { "" };
    let ops: Vec<&str> = if ops_str.is_empty() {
        vec![]
    } else {
        let mut result = Vec::new();
        let mut depth = 0;
        let mut start = 0;
        for (i, c) in ops_str.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                ',' if depth == 0 => {
                    result.push(ops_str[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }
        result.push(ops_str[start..].trim());
        result
    };

    match mnem.as_str() {
        "nop" => Ok(0x60000000),
        "blr" => Ok(0x4E800020),
        "blrl" => Ok(0x4E800021),
        "bctr" => Ok(0x4E800420),
        "bctrl" => Ok(0x4E800421),
        "sc" => Ok(0x44000002),
        "sync" => Ok(0x7C0004AC),
        "isync" => Ok(0x4C00012C),
        "eieio" => Ok(0x7C0006AC),

        "li" => {
            let rd = parse_reg(ops[0])?;
            let imm = parse_imm(ops[1])? as i16 as u16;
            Ok(((14 << 26) | (rd << 21)) | imm as u32)
        }
        "lis" => {
            let rd = parse_reg(ops[0])?;
            let imm = parse_imm(ops[1])? as i16 as u16;
            Ok(((15 << 26) | (rd << 21)) | imm as u32)
        }
        "addi" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((14 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "addis" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((15 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "addic" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((12 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "addic." => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((13 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "subi" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = -(parse_imm(ops[2])?) as i16 as u16;
            Ok((14 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "subfic" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((8 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "mulli" => {
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let imm = parse_imm(ops[2])? as i16 as u16;
            Ok((7 << 26) | (rd << 21) | (ra << 16) | imm as u32)
        }
        "cmpwi" => {
            let (bf, ra, imm) = if ops.len() == 3 {
                (parse_creg(ops[0])?, parse_reg(ops[1])?, parse_imm(ops[2])? as i16 as u16)
            } else {
                (0, parse_reg(ops[0])?, parse_imm(ops[1])? as i16 as u16)
            };
            Ok((11 << 26) | (bf << 23) | (ra << 16) | imm as u32)
        }
        "cmplwi" => {
            let (bf, ra, imm) = if ops.len() == 3 {
                (parse_creg(ops[0])?, parse_reg(ops[1])?, parse_uimm(ops[2])? & 0xFFFF)
            } else {
                (0, parse_reg(ops[0])?, parse_uimm(ops[1])? & 0xFFFF)
            };
            Ok((10 << 26) | (bf << 23) | (ra << 16) | imm)
        }
        "ori" => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((24 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "oris" => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((25 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "xori" => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((26 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "xoris" => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((27 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "andi." => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((28 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "andis." => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let imm = parse_uimm(ops[2])? & 0xFFFF;
            Ok((29 << 26) | (rs << 21) | (ra << 16) | imm)
        }
        "mr" => {
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            Ok((31 << 26) | (rs << 21) | (ra << 16) | (rs << 11) | (444 << 1))
        }

        "rlwinm" | "rlwinm." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let sh = parse_uimm(ops[2])? & 0x1F;
            let mb = parse_uimm(ops[3])? & 0x1F;
            let me = parse_uimm(ops[4])? & 0x1F;
            Ok((21 << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (mb << 6) | (me << 1) | rc)
        }
        "rlwimi" | "rlwimi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let sh = parse_uimm(ops[2])? & 0x1F;
            let mb = parse_uimm(ops[3])? & 0x1F;
            let me = parse_uimm(ops[4])? & 0x1F;
            Ok((20 << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (mb << 6) | (me << 1) | rc)
        }
        "rlwnm" | "rlwnm." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let rb = parse_reg(ops[2])?;
            let mb = parse_uimm(ops[3])? & 0x1F;
            let me = parse_uimm(ops[4])? & 0x1F;
            Ok((23 << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (mb << 6) | (me << 1) | rc)
        }
        "slwi" | "slwi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let n = parse_uimm(ops[2])? & 0x1F;
            Ok(((21 << 26) | (rs << 21) | (ra << 16) | (n << 11)) | ((31 - n) << 1) | rc)
        }
        "srwi" | "srwi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let n = parse_uimm(ops[2])? & 0x1F;
            let sh = 32 - n;
            Ok((21 << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (n << 6) | (31 << 1) | rc)
        }
        "clrlwi" | "clrlwi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let n = parse_uimm(ops[2])? & 0x1F;
            Ok(((21 << 26) | (rs << 21) | (ra << 16)) | (n << 6) | (31 << 1) | rc)
        }
        "clrrwi" | "clrrwi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let n = parse_uimm(ops[2])? & 0x1F;
            Ok(((21 << 26) | (rs << 21) | (ra << 16)) | ((31 - n) << 1) | rc)
        }
        "rotlwi" | "rotlwi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let n = parse_uimm(ops[2])? & 0x1F;
            Ok(((21 << 26) | (rs << 21) | (ra << 16) | (n << 11)) | (31 << 1) | rc)
        }

        "lwz" | "lwzu" | "lbz" | "lbzu" | "lhz" | "lhzu" | "lha" | "lhau" |
        "stw" | "stwu" | "stb" | "stbu" | "sth" | "sthu" |
        "lfs" | "lfsu" | "lfd" | "lfdu" | "stfs" | "stfsu" | "stfd" | "stfdu" => {
            let opcode_num = match mnem.as_str() {
                "lwz" => 32, "lwzu" => 33, "lbz" => 34, "lbzu" => 35,
                "lhz" => 40, "lhzu" => 41, "lha" => 42, "lhau" => 43,
                "stw" => 36, "stwu" => 37, "stb" => 38, "stbu" => 39,
                "sth" => 44, "sthu" => 45,
                "lfs" => 48, "lfsu" => 49, "lfd" => 50, "lfdu" => 51,
                "stfs" => 52, "stfsu" => 53, "stfd" => 54, "stfdu" => 55,
                _ => return Err(format!("Unknown mnemonic: {}", mnem)),
            };
            let is_float = mnem.contains('f');
            let rt = if is_float { parse_freg(ops[0])? } else { parse_reg(ops[0])? };
            let (d, ra) = parse_d_ra(ops[1])?;
            Ok((opcode_num << 26) | (rt << 21) | (ra << 16) | (d as u16 as u32))
        }

        "add" | "add." | "addc" | "addc." | "adde" | "adde." |
        "subf" | "subf." | "subfc" | "subfc." | "subfe" | "subfe." |
        "mullw" | "mullw." | "divw" | "divw." | "divwu" | "divwu." |
        "and" | "and." | "or" | "or." | "xor" | "xor." |
        "nor" | "nor." | "nand" | "nand." | "eqv" | "eqv." |
        "andc" | "andc." | "orc" | "orc." |
        "slw" | "slw." | "srw" | "srw." | "sraw" | "sraw." => {
            let base = mnem.trim_end_matches('.');
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let xo = match base {
                "add" => 266, "addc" => 10, "adde" => 138,
                "subf" => 40, "subfc" => 8, "subfe" => 136,
                "mullw" => 235, "divw" => 491, "divwu" => 459,
                "and" => 28, "or" => 444, "xor" => 316,
                "nor" => 124, "nand" => 476, "eqv" => 284,
                "andc" => 60, "orc" => 412,
                "slw" => 24, "srw" => 536, "sraw" => 792,
                _ => return Err(format!("Unknown: {}", mnem)),
            };
            let is_logical = matches!(base, "and"|"or"|"xor"|"nor"|"nand"|"eqv"|"andc"|"orc"|"slw"|"srw"|"sraw");
            if is_logical {
                let ra = parse_reg(ops[0])?;
                let rs = parse_reg(ops[1])?;
                let rb = parse_reg(ops[2])?;
                Ok((31 << 26) | (rs << 21) | (ra << 16) | (rb << 11) | (xo << 1) | rc)
            } else {
                let rd = parse_reg(ops[0])?;
                let ra = parse_reg(ops[1])?;
                let rb = parse_reg(ops[2])?;
                Ok((31 << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (xo << 1) | rc)
            }
        }

        "neg" | "neg." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            Ok((31 << 26) | (rd << 21) | (ra << 16) | (104 << 1) | rc)
        }

        "extsb" | "extsb." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            Ok((31 << 26) | (rs << 21) | (ra << 16) | (954 << 1) | rc)
        }
        "extsh" | "extsh." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            Ok((31 << 26) | (rs << 21) | (ra << 16) | (922 << 1) | rc)
        }
        "cntlzw" | "cntlzw." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            Ok((31 << 26) | (rs << 21) | (ra << 16) | (26 << 1) | rc)
        }
        "srawi" | "srawi." => {
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let ra = parse_reg(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let sh = parse_uimm(ops[2])? & 0x1F;
            Ok((31 << 26) | (rs << 21) | (ra << 16) | (sh << 11) | (824 << 1) | rc)
        }

        "cmpw" => {
            let (bf, ra, rb) = if ops.len() == 3 {
                (parse_creg(ops[0])?, parse_reg(ops[1])?, parse_reg(ops[2])?)
            } else {
                (0, parse_reg(ops[0])?, parse_reg(ops[1])?)
            };
            Ok((31 << 26) | (bf << 23) | (ra << 16) | (rb << 11) )
        }
        "cmplw" => {
            let (bf, ra, rb) = if ops.len() == 3 {
                (parse_creg(ops[0])?, parse_reg(ops[1])?, parse_reg(ops[2])?)
            } else {
                (0, parse_reg(ops[0])?, parse_reg(ops[1])?)
            };
            Ok((31 << 26) | (bf << 23) | (ra << 16) | (rb << 11) | (32 << 1))
        }

        "lwzx" | "lbzx" | "lhzx" | "lhax" | "stwx" | "stbx" | "sthx" => {
            let xo = match mnem.as_str() {
                "lwzx" => 23, "lbzx" => 87, "lhzx" => 279, "lhax" => 343,
                "stwx" => 151, "stbx" => 215, "sthx" => 407,
                _ => return Err(format!("Unknown: {}", mnem)),
            };
            let rd = parse_reg(ops[0])?;
            let ra = parse_reg(ops[1])?;
            let rb = parse_reg(ops[2])?;
            Ok((31 << 26) | (rd << 21) | (ra << 16) | (rb << 11) | (xo << 1))
        }

        "mflr" => { let rd = parse_reg(ops[0])?; Ok((31 << 26) | (rd << 21) | (8 << 16) | (339 << 1)) }
        "mtlr" => { let rs = parse_reg(ops[0])?; Ok((31 << 26) | (rs << 21) | (8 << 16) | (467 << 1)) }
        "mfctr" => { let rd = parse_reg(ops[0])?; Ok((31 << 26) | (rd << 21) | (9 << 16) | (339 << 1)) }
        "mtctr" => { let rs = parse_reg(ops[0])?; Ok((31 << 26) | (rs << 21) | (9 << 16) | (467 << 1)) }
        "mfxer" => { let rd = parse_reg(ops[0])?; Ok((31 << 26) | (rd << 21) | (1 << 16) | (339 << 1)) }
        "mtxer" => { let rs = parse_reg(ops[0])?; Ok((31 << 26) | (rs << 21) | (1 << 16) | (467 << 1)) }
        "mfspr" => {
            let rd = parse_reg(ops[0])?;
            let spr_val = parse_uimm(ops[1])?;
            let spr_lo = spr_val & 0x1F; let spr_hi = (spr_val >> 5) & 0x1F;
            Ok((31 << 26) | (rd << 21) | (spr_lo << 16) | (spr_hi << 11) | (339 << 1))
        }
        "mtspr" => {
            let spr_val = parse_uimm(ops[0])?;
            let rs = parse_reg(ops[1])?;
            let spr_lo = spr_val & 0x1F; let spr_hi = (spr_val >> 5) & 0x1F;
            Ok((31 << 26) | (rs << 21) | (spr_lo << 16) | (spr_hi << 11) | (467 << 1))
        }
        "mfcr" => { let rd = parse_reg(ops[0])?; Ok((31 << 26) | (rd << 21) | (19 << 1)) }
        "mtcrf" => {
            let fxm = parse_uimm(ops[0])? & 0xFF;
            let rs = parse_reg(ops[1])?;
            Ok((31 << 26) | (rs << 21) | (fxm << 12) | (144 << 1))
        }
        "mfmsr" => { let rd = parse_reg(ops[0])?; Ok((31 << 26) | (rd << 21) | (83 << 1)) }
        "mtmsr" => { let rs = parse_reg(ops[0])?; Ok((31 << 26) | (rs << 21) | (146 << 1)) }

        "b" => { let target = parse_imm(ops[0])? as u32 & 0x03FFFFFC; Ok((18 << 26) | target) }
        "bl" => { let target = parse_imm(ops[0])? as u32 & 0x03FFFFFC; Ok((18 << 26) | target | 1) }
        "ba" => { let target = parse_imm(ops[0])? as u32 & 0x03FFFFFC; Ok((18 << 26) | target | 2) }
        "bla" => { let target = parse_imm(ops[0])? as u32 & 0x03FFFFFC; Ok((18 << 26) | target | 3) }

        "bc" | "bcl" | "bca" | "bcla" => {
            let lk = if mnem.contains('l') { 1u32 } else { 0 };
            let aa = if mnem.contains('a') { 1u32 } else { 0 };
            let bo = parse_uimm(ops[0])? & 0x1F;
            let bi = parse_uimm(ops[1])? & 0x1F;
            let target = parse_imm(ops[2])? as u32 & 0xFFFC;
            Ok((16 << 26) | (bo << 21) | (bi << 16) | target | (aa << 1) | lk)
        }
        "bclr" | "bclrl" => {
            let lk = if mnem.ends_with('l') { 1u32 } else { 0 };
            let bo = parse_uimm(ops[0])? & 0x1F;
            let bi = parse_uimm(ops[1])? & 0x1F;
            Ok((19 << 26) | (bo << 21) | (bi << 16) | (16 << 1) | lk)
        }
        "bcctr" | "bcctrl" => {
            let lk = if mnem.ends_with('l') { 1u32 } else { 0 };
            let bo = parse_uimm(ops[0])? & 0x1F;
            let bi = parse_uimm(ops[1])? & 0x1F;
            Ok((19 << 26) | (bo << 21) | (bi << 16) | (528 << 1) | lk)
        }

        "beq" | "beql" | "bne" | "bnel" | "blt" | "bltl" | "bgt" | "bgtl" |
        "ble" | "blel" | "bge" | "bgel" | "bso" | "bsol" | "bns" | "bnsl" |
        "beqa" | "beqla" | "bnea" | "bnela" | "blta" | "bltla" | "bgta" | "bgtla" |
        "blea" | "blela" | "bgea" | "bgela" => {
            let base_mnem = mnem.trim_end_matches('a').trim_end_matches('l');
            let lk = if mnem.contains('l') { 1u32 } else { 0 };
            let aa = if mnem.ends_with('a') || (mnem.ends_with("la")) { 1u32 } else { 0 };
            let (bo, bi_field) = match base_mnem {
                "beq" => (12, 2), "bne" => (4, 2),
                "blt" => (12, 0), "bge" => (4, 0),
                "bgt" => (12, 1), "ble" => (4, 1),
                "bso" => (12, 3), "bns" => (4, 3),
                _ => return Err(format!("Unknown branch: {}", mnem)),
            };
            let (cr_field, target_str) = if ops.len() >= 2 && ops[0].starts_with("cr") {
                (parse_creg(ops[0])?, ops[1])
            } else {
                (0u32, ops[0])
            };
            let bi = (cr_field << 2) | bi_field;
            let target = parse_imm(target_str)? as u32 & 0xFFFC;
            Ok((16 << 26) | (bo << 21) | (bi << 16) | target | (aa << 1) | lk)
        }

        "beqlr" | "bnelr" | "bltlr" | "bgtlr" | "blelr" | "bgelr" |
        "beqlrl" | "bnelrl" | "bltlrl" | "bgtlrl" | "blelrl" | "bgelrl" => {
            let base = mnem.strip_suffix("lrl").or_else(|| mnem.strip_suffix("lr")).unwrap_or(&mnem);
            let lk = if mnem.ends_with("lrl") { 1u32 } else { 0 };
            let (bo, bi_field) = match base {
                "beq" => (12, 2), "bne" => (4, 2),
                "blt" => (12, 0), "bge" => (4, 0),
                "bgt" => (12, 1), "ble" => (4, 1),
                _ => return Err(format!("Unknown branch: {}", mnem)),
            };
            let cr_field = if !ops.is_empty() && ops[0].starts_with("cr") {
                parse_creg(ops[0])?
            } else { 0 };
            let bi = (cr_field << 2) | bi_field;
            Ok((19 << 26) | (bo << 21) | (bi << 16) | (16 << 1) | lk)
        }

        "bdnz" | "bdnzl" | "bdnza" | "bdnzla" => {
            let lk = if mnem.contains('l') { 1u32 } else { 0 };
            let aa = if mnem.contains('a') { 1u32 } else { 0 };
            let target = parse_imm(ops[0])? as u32 & 0xFFFC;
            Ok((16 << 26) | (16 << 21) | target | (aa << 1) | lk)
        }
        "bdz" | "bdzl" | "bdza" | "bdzla" => {
            let lk = if mnem.contains('l') { 1u32 } else { 0 };
            let aa = if mnem.contains('a') { 1u32 } else { 0 };
            let target = parse_imm(ops[0])? as u32 & 0xFFFC;
            Ok((16 << 26) | (18 << 21) | target | (aa << 1) | lk)
        }

        "crand" | "cror" | "crxor" | "crnand" | "crnor" | "creqv" | "crandc" | "crorc" => {
            let xo = match mnem.as_str() {
                "crand" => 257, "cror" => 449, "crxor" => 193, "crnand" => 225,
                "crnor" => 33, "creqv" => 289, "crandc" => 129, "crorc" => 417,
                _ => unreachable!(),
            };
            let bt = parse_uimm(ops[0])? & 0x1F;
            let ba = parse_uimm(ops[1])? & 0x1F;
            let bb = parse_uimm(ops[2])? & 0x1F;
            Ok((19 << 26) | (bt << 21) | (ba << 16) | (bb << 11) | (xo << 1))
        }

        "fadd" | "fadd." | "fadds" | "fadds." |
        "fsub" | "fsub." | "fsubs" | "fsubs." |
        "fdiv" | "fdiv." | "fdivs" | "fdivs." => {
            let base = mnem.trim_end_matches('.').trim_end_matches('s');
            let is_single = mnem.trim_end_matches('.').ends_with('s');
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let primary = if is_single { 59 } else { 63 };
            let xo = match base { "fadd" => 21, "fsub" => 20, "fdiv" => 18, _ => unreachable!() };
            let frd = parse_freg(ops[0])?;
            let fra = parse_freg(ops[1])?;
            let frb = parse_freg(ops[2])?;
            Ok((primary << 26) | (frd << 21) | (fra << 16) | (frb << 11) | (xo << 1) | rc)
        }
        "fmul" | "fmul." | "fmuls" | "fmuls." => {
            let is_single = mnem.trim_end_matches('.').ends_with('s');
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let primary = if is_single { 59 } else { 63 };
            let frd = parse_freg(ops[0])?;
            let fra = parse_freg(ops[1])?;
            let frc = parse_freg(ops[2])?;
            Ok((primary << 26) | (frd << 21) | (fra << 16) | (frc << 6) | (25 << 1) | rc)
        }
        "fmadd" | "fmadd." | "fmadds" | "fmadds." |
        "fmsub" | "fmsub." | "fmsubs" | "fmsubs." => {
            let base = mnem.trim_end_matches('.').trim_end_matches('s');
            let is_single = mnem.trim_end_matches('.').ends_with('s');
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let primary = if is_single { 59 } else { 63 };
            let xo = match base { "fmadd" => 29, "fmsub" => 28, _ => unreachable!() };
            let frd = parse_freg(ops[0])?;
            let fra = parse_freg(ops[1])?;
            let frc = parse_freg(ops[2])?;
            let frb = parse_freg(ops[3])?;
            Ok((primary << 26) | (frd << 21) | (fra << 16) | (frb << 11) | (frc << 6) | (xo << 1) | rc)
        }
        "fneg" | "fneg." | "fabs" | "fabs." | "fmr" | "fmr." | "fnabs" | "fnabs." => {
            let base = mnem.trim_end_matches('.');
            let rc = if mnem.ends_with('.') { 1u32 } else { 0 };
            let xo = match base { "fneg" => 40, "fabs" => 264, "fmr" => 72, "fnabs" => 136, _ => unreachable!() };
            let frd = parse_freg(ops[0])?;
            let frb = parse_freg(ops[1])?;
            Ok((63 << 26) | (frd << 21) | (frb << 11) | (xo << 1) | rc)
        }
        "fcmpu" => {
            let bf = parse_creg(ops[0])?;
            let fra = parse_freg(ops[1])?;
            let frb = parse_freg(ops[2])?;
            Ok((63 << 26) | (bf << 23) | (fra << 16) | (frb << 11))
        }
        "fcmpo" => {
            let bf = parse_creg(ops[0])?;
            let fra = parse_freg(ops[1])?;
            let frb = parse_freg(ops[2])?;
            Ok((63 << 26) | (bf << 23) | (fra << 16) | (frb << 11) | (32 << 1))
        }

        "dcbf" | "dcbt" | "dcbtst" => {
            let xo = match mnem.as_str() { "dcbf" => 86, "dcbt" => 278, "dcbtst" => 246, _ => unreachable!() };
            let ra = parse_reg(ops[0])?;
            let rb = parse_reg(ops[1])?;
            Ok((31 << 26) | (ra << 16) | (rb << 11) | (xo << 1))
        }

        _ => Err(format!("Unknown mnemonic: {}", mnem)),
    }
}

pub fn gecko_to_asm(gecko_text: &str) -> Result<String, String> {
    let mut output = Vec::new();
    let hex_chars: String = gecko_text.chars().filter(|c| c.is_ascii_hexdigit() || c.is_whitespace() || *c == '\n').collect();
    let words: Vec<&str> = hex_chars.split_whitespace().collect();

    let mut i = 0;
    while i < words.len() {
        let w = words[i].to_uppercase();
        if w.len() != 8 {
            i += 1;
            continue;
        }

        if let Some(stripped) = w.strip_prefix("C2") {
            let addr = u32::from_str_radix(stripped, 16).map_err(|e| format!("Invalid hex: {}", e))?;
            output.push(format!("; ASM patch at 0x{:08X}", addr));
            i += 1;
            if i < words.len() {
                i += 1;
            }
            while i < words.len() {
                let hi = words[i].to_uppercase();
                if hi == "00000000"
                    && i + 1 < words.len() && words[i + 1].to_uppercase() == "00000000" {
                        i += 2;
                        break;
                    }
                if let Ok(val) = u32::from_str_radix(&hi, 16) {
                    output.push(disassemble_instruction(val));
                }
                i += 1;
            }
        } else if w == "00000000" {
            i += 1;
        } else {
            let addr = u32::from_str_radix(&w, 16).map_err(|e| format!("Invalid hex: {}", e))?;
            i += 1;
            if i < words.len() {
                let val_word = words[i].to_uppercase();
                if let Ok(val) = u32::from_str_radix(&val_word, 16) {
                    let asm = disassemble_instruction(val);
                    output.push(format!("0x{:08X} = {}", addr, asm));
                    i += 1;
                } else {
                    output.push(format!("; Invalid value: {}", val_word));
                    i += 1;
                }
            }
        }
    }
    Ok(output.join("\n"))
}

pub fn cemu_asm_to_gecko(asm_text: &str, fallback_address: u32) -> Result<String, String> {
    let mut output = Vec::new();

    for line in asm_text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(eq_pos) = line.find('=') {
            let addr_part = line[..eq_pos].trim();
            let asm_part = line[eq_pos + 1..].trim();

            let addr_str = addr_part.trim_start_matches("0x").trim_start_matches("0X");
            if let Ok(address) = u32::from_str_radix(addr_str, 16)
                && !asm_part.is_empty() {
                    let encoded = assemble_instruction(asm_part)?;
                    output.push(format!("{:08X} {:08X}", address, encoded));
                    continue;
                }
        }

        let encoded = assemble_instruction(line)?;
        output.push(format!("{:08X} {:08X}", fallback_address, encoded));
    }

    if output.is_empty() {
        return Err("No instructions to assemble".into());
    }

    Ok(output.join("\n"))
}

pub fn console_addr_to_cemu(addr: u32) -> u32 {
    if addr >= 0x0C000000 {
        addr - 0x0C000000
    } else {
        addr
    }
}

pub fn cemu_addr_to_console(addr: u32) -> u32 {
    addr + 0x0C000000
}

pub fn gecko_convert_addresses(gecko_text: &str, to_cemu: bool) -> String {
    let mut output = Vec::new();
    let lines: Vec<&str> = gecko_text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() {
            output.push(lines[i].to_string());
            i += 1;
            continue;
        }

        let words: Vec<&str> = trimmed.split_whitespace().collect();
        if words.is_empty() || words[0].len() != 8 {
            output.push(lines[i].to_string());
            i += 1;
            continue;
        }

        let first = words[0].to_uppercase();

        if let Some(stripped) = first.strip_prefix("C2") {
            if let Ok(addr) = u32::from_str_radix(stripped, 16) {
                let new_addr = if to_cemu {
                    console_addr_to_cemu(addr)
                } else {
                    cemu_addr_to_console(addr)
                };
                let rest = if words.len() > 1 {
                    format!(" {}", words[1..].join(" "))
                } else {
                    String::new()
                };
                output.push(format!("C2{:06X}{}", new_addr & 0x00FFFFFF, rest));
            } else {
                output.push(lines[i].to_string());
            }
        } else if first == "00000000" {
            output.push(lines[i].to_string());
        } else if let Ok(addr) = u32::from_str_radix(&first, 16) {
            let new_addr = if to_cemu {
                console_addr_to_cemu(addr)
            } else {
                cemu_addr_to_console(addr)
            };
            let rest = if words.len() > 1 {
                format!(" {}", words[1..].join(" "))
            } else {
                String::new()
            };
            output.push(format!("{:08X}{}", new_addr, rest));
        } else {
            output.push(lines[i].to_string());
        }
        i += 1;
    }
    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(disassemble_instruction(0x38600001), "li r3,1");
    }

    #[test]
    fn test_nop() {
        assert_eq!(assemble_instruction("nop").unwrap_or(0), 0x60000000);
        assert_eq!(disassemble_instruction(0x60000000), "nop");
    }

    #[test]
    fn test_blr() {
        assert_eq!(assemble_instruction("blr").unwrap_or(0), 0x4E800020);
        assert_eq!(disassemble_instruction(0x4E800020), "blr");
    }

    #[test]
    fn test_stw() {
        let val = assemble_instruction("stw r3,0(r1)").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "stw r3,0(r1)");
    }

    #[test]
    fn test_lwz() {
        let val = assemble_instruction("lwz r3,4(r1)").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "lwz r3,4(r1)");
    }

    #[test]
    fn test_addi() {
        let val = assemble_instruction("addi r3,r4,100").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "addi r3,r4,100");
    }

    #[test]
    fn test_lis() {
        let val = assemble_instruction("lis r3,0x1234").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "lis r3,4660");
    }

    #[test]
    fn test_roundtrip_add() {
        let val = assemble_instruction("add r3,r4,r5").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "add r3,r4,r5");
    }

    #[test]
    fn test_roundtrip_mr() {
        let val = assemble_instruction("mr r3,r4").unwrap_or(0);
        assert_eq!(disassemble_instruction(val), "mr r3,r4");
    }

    #[test]
    fn test_gecko_roundtrip() {
        let asm_text = "li r3,1";
        let gecko = cemu_asm_to_gecko(asm_text, 0x12345678).unwrap_or("".to_string());
        assert!(gecko.contains("12345678"));
        let back = gecko_to_asm(&gecko).unwrap_or("".to_string());
        assert!(back.contains("li r3,1"));
    }
}
