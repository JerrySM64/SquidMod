pub(super) fn sign_extend(val: u32, bits: u32) -> i32 {
    let shift = 32 - bits;
    ((val << shift) as i32) >> shift
}

pub(super) fn hex_imm(val: i32) -> String {
    if val < 0 {
        format!("-0x{:x}", -(val as i64))
    } else {
        format!("0x{:x}", val)
    }
}

pub(super) fn raw_word(word: u32) -> String {
    format!(".int 0x{:08X}", word)
}

pub(super) fn spr_name(spr: u32) -> Option<&'static str> {
    match spr {
        8 => Some("lr"),
        9 => Some("ctr"),
        1 => Some("xer"),
        _ => None,
    }
}

pub(super) fn cond_name(bo: u32, bi_field: u32) -> Option<(&'static str, bool)> {
    let name = match (bo & !1, bi_field) {
        (12, 0) => "lt",
        (4, 0) => "ge",
        (12, 1) => "gt",
        (4, 1) => "le",
        (12, 2) => "eq",
        (4, 2) => "ne",
        (12, 3) => "so",
        (4, 3) => "ns",
        _ => return None,
    };
    Some((name, bo & 1 == 1))
}

pub(super) fn load_store_mnemonic(opcode: u32) -> Option<(&'static str, bool)> {
    let name = match opcode {
        32 => "lwz",
        33 => "lwzu",
        34 => "lbz",
        35 => "lbzu",
        36 => "stw",
        37 => "stwu",
        38 => "stb",
        39 => "stbu",
        40 => "lhz",
        41 => "lhzu",
        42 => "lha",
        43 => "lhau",
        44 => "sth",
        45 => "sthu",
        46 => "lmw",
        47 => "stmw",
        48 => "lfs",
        49 => "lfsu",
        50 => "lfd",
        51 => "lfdu",
        52 => "stfs",
        53 => "stfsu",
        54 => "stfd",
        55 => "stfdu",
        _ => return None,
    };
    Some((name, opcode >= 48))
}

pub(super) fn load_store_opcode(mnem: &str) -> Option<(u32, bool)> {
    (32..=55u32).find_map(|opcode| match load_store_mnemonic(opcode) {
        Some((name, float)) if name == mnem => Some((opcode, float)),
        _ => None,
    })
}

pub fn branch_target(word: u32, addr: u32) -> Option<(u32, bool)> {
    let disp: i32 = match word >> 26 {
        18 => sign_extend(word & 0x03FF_FFFC, 26),
        16 => sign_extend(word & 0xFFFC, 16),
        _ => return None,
    };
    let disp = disp as u32;
    let absolute = (word >> 1) & 1 == 1;
    let target = if absolute { disp } else { addr.wrapping_add(disp) };
    Some((target, absolute))
}
