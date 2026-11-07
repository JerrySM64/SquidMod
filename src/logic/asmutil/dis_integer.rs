use super::isa::{hex_imm, raw_word, sign_extend, spr_name};

pub(super) fn dis_addi(w: u32) -> String {
    let rd = (w >> 21) & 0x1F;
    let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    if ra == 0 { format!("li r{},{}", rd, imm) }
    else if imm < 0 { format!("subi r{},r{},{}", rd, ra, -imm) }
    else { format!("addi r{},r{},{}", rd, ra, imm) }
}

pub(super) fn dis_addis(w: u32) -> String {
    let rd = (w >> 21) & 0x1F;
    let ra = (w >> 16) & 0x1F;
    if ra == 0 { format!("lis r{},0x{:x}", rd, w & 0xFFFF) }
    else { format!("addis r{},r{},{}", rd, ra, hex_imm(sign_extend(w & 0xFFFF, 16))) }
}

pub(super) fn dis_addic(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("addic r{},r{},{}", rd, ra, imm)
}

pub(super) fn dis_addic_dot(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("addic. r{},r{},{}", rd, ra, imm)
}

pub(super) fn dis_mulli(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("mulli r{},r{},{}", rd, ra, imm)
}

pub(super) fn dis_subfic(w: u32) -> String {
    let rd = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    format!("subfic r{},r{},{}", rd, ra, imm)
}

pub(super) fn dis_cmpi(w: u32) -> String {
    let bf = (w >> 23) & 0x7; let ra = (w >> 16) & 0x1F;
    let imm = sign_extend(w & 0xFFFF, 16);
    if bf == 0 { format!("cmpwi r{},{}", ra, imm) }
    else { format!("cmpwi cr{},r{},{}", bf, ra, imm) }
}

pub(super) fn dis_cmpli(w: u32) -> String {
    let bf = (w >> 23) & 0x7; let ra = (w >> 16) & 0x1F;
    let imm = w & 0xFFFF;
    if bf == 0 { format!("cmplwi r{},{}", ra, imm) }
    else { format!("cmplwi cr{},r{},{}", bf, ra, imm) }
}

pub(super) fn dis_ori(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    if rs == 0 && ra == 0 && imm == 0 { "nop".to_string() }
    else { format!("ori r{},r{},0x{:x}", ra, rs, imm) }
}

pub(super) fn dis_oris(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("oris r{},r{},0x{:x}", ra, rs, imm)
}

pub(super) fn dis_xori(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("xori r{},r{},0x{:x}", ra, rs, imm)
}

pub(super) fn dis_xoris(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("xoris r{},r{},0x{:x}", ra, rs, imm)
}

pub(super) fn dis_andi_dot(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("andi. r{},r{},0x{:x}", ra, rs, imm)
}

pub(super) fn dis_andis_dot(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let imm = w & 0xFFFF;
    format!("andis. r{},r{},0x{:x}", ra, rs, imm)
}

pub(super) fn dis_rlwinm(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let sh = (w >> 11) & 0x1F; let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1;
    let dot = if rc != 0 { "." } else { "" };
    if mb == 0 && me == 31 - sh { return format!("slwi{} r{},r{},{}", dot, ra, rs, sh); }
    if me == 31 && sh != 0 && mb == 32 - sh { return format!("srwi{} r{},r{},{}", dot, ra, rs, 32 - sh); }
    if sh == 0 && me == 31 { return format!("clrlwi{} r{},r{},{}", dot, ra, rs, mb); }
    if sh == 0 && mb == 0 { return format!("clrrwi{} r{},r{},{}", dot, ra, rs, 31 - me); }
    if mb == 0 && me == 31 { return format!("rotlwi{} r{},r{},{}", dot, ra, rs, sh); }
    format!("rlwinm{} r{},r{},{},{},{}", dot, ra, rs, sh, mb, me)
}

pub(super) fn dis_rlwimi(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let sh = (w >> 11) & 0x1F; let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    format!("rlwimi{} r{},r{},{},{},{}", dot, ra, rs, sh, mb, me)
}

pub(super) fn dis_rlwnm(w: u32) -> String {
    let rs = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F; let rb = (w >> 11) & 0x1F;
    let mb = (w >> 6) & 0x1F; let me = (w >> 1) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    format!("rlwnm{} r{},r{},r{},{},{}", dot, ra, rs, rb, mb, me)
}

pub(super) fn dis_load_store_imm(w: u32, mnem: &str, float: bool) -> String {
    let rt = (w >> 21) & 0x1F; let ra = (w >> 16) & 0x1F;
    let d = sign_extend(w & 0xFFFF, 16);
    let prefix = if float { 'f' } else { 'r' };
    format!("{} {}{},{}(r{})", mnem, prefix, rt, d, ra)
}


pub(super) fn dis_opcode31(w: u32) -> String {
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
        75  => format!("mulhw{} r{},r{},r{}", dot, rd, ra, rb),
        11  => format!("mulhwu{} r{},r{},r{}", dot, rd, ra, rb),
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
        824 => format!("srawi{} r{},r{},{}", dot, ra, rd, rb),
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
        55  => format!("lwzux r{},r{},r{}", rd, ra, rb),
        87  => format!("lbzx r{},r{},r{}", rd, ra, rb),
        119 => format!("lbzux r{},r{},r{}", rd, ra, rb),
        279 => format!("lhzx r{},r{},r{}", rd, ra, rb),
        311 => format!("lhzux r{},r{},r{}", rd, ra, rb),
        343 => format!("lhax r{},r{},r{}", rd, ra, rb),
        375 => format!("lhaux r{},r{},r{}", rd, ra, rb),
        151 => format!("stwx r{},r{},r{}", rd, ra, rb),
        183 => format!("stwux r{},r{},r{}", rd, ra, rb),
        215 => format!("stbx r{},r{},r{}", rd, ra, rb),
        247 => format!("stbux r{},r{},r{}", rd, ra, rb),
        407 => format!("sthx r{},r{},r{}", rd, ra, rb),
        439 => format!("sthux r{},r{},r{}", rd, ra, rb),
        535 => format!("lfsx f{},r{},r{}", rd, ra, rb),
        567 => format!("lfsux f{},r{},r{}", rd, ra, rb),
        599 => format!("lfdx f{},r{},r{}", rd, ra, rb),
        631 => format!("lfdux f{},r{},r{}", rd, ra, rb),
        663 => format!("stfsx f{},r{},r{}", rd, ra, rb),
        695 => format!("stfsux f{},r{},r{}", rd, ra, rb),
        727 => format!("stfdx f{},r{},r{}", rd, ra, rb),
        759 => format!("stfdux f{},r{},r{}", rd, ra, rb),
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
        _ => raw_word(w),
    }
}

