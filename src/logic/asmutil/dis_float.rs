use super::isa::raw_word;

pub(super) fn dis_opcode59(w: u32) -> String {
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
        _ => raw_word(w),
    }
}

pub(super) fn dis_opcode63(w: u32) -> String {
    let xo_full = (w >> 1) & 0x3FF;
    let xo_short = (w >> 1) & 0x1F;
    let frd = (w >> 21) & 0x1F; let fra = (w >> 16) & 0x1F;
    let frb = (w >> 11) & 0x1F; let frc = (w >> 6) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    match xo_full {
        0   => { let bf = (w >> 23) & 7; format!("fcmpu cr{},f{},f{}", bf, fra, frb) }
        32  => { let bf = (w >> 23) & 7; format!("fcmpo cr{},f{},f{}", bf, fra, frb) }
        12  => format!("frsp{} f{},f{}", dot, frd, frb),
        15  => format!("fctiwz{} f{},f{}", dot, frd, frb),
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
                _ => raw_word(w),
            }
        }
    }
}

pub(super) fn dis_paired(w: u32) -> String {
    let frd = (w >> 21) & 0x1F; let fra = (w >> 16) & 0x1F;
    let frb = (w >> 11) & 0x1F; let frc = (w >> 6) & 0x1F;
    let rc = w & 1; let dot = if rc != 0 { "." } else { "" };
    match (w >> 1) & 0x3FF {
        528 => format!("ps_merge00{} f{},f{},f{}", dot, frd, fra, frb),
        560 => format!("ps_merge01{} f{},f{},f{}", dot, frd, fra, frb),
        592 => format!("ps_merge10{} f{},f{},f{}", dot, frd, fra, frb),
        624 => format!("ps_merge11{} f{},f{},f{}", dot, frd, fra, frb),
        _ => match (w >> 1) & 0x1F {
            21 => format!("ps_add{} f{},f{},f{}", dot, frd, fra, frb),
            20 => format!("ps_sub{} f{},f{},f{}", dot, frd, fra, frb),
            18 => format!("ps_div{} f{},f{},f{}", dot, frd, fra, frb),
            25 => format!("ps_mul{} f{},f{},f{}", dot, frd, fra, frc),
            29 => format!("ps_madd{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
            28 => format!("ps_msub{} f{},f{},f{},f{}", dot, frd, fra, frc, frb),
            _ => raw_word(w),
        },
    }
}

