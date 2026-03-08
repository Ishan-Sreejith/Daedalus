#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    X(u8), // 64-bit register (0-30)
    W(u8), // 32-bit register (0-30)
    SP,    // Stack Pointer (X31)
    XZR,   // Zero Register (X31)
}

impl Reg {
    pub fn encode(&self) -> u8 {
        match self {
            Reg::X(r) | Reg::W(r) => *r,
            Reg::SP | Reg::XZR => 31,
        }
    }

    pub fn from_encoded(val: u8, is_64bit: bool) -> Self {
        if val == 31 {
            if is_64bit {
                Reg::XZR
            } else {
                Reg::SP
            } // SP for 32-bit context, XZR for 64-bit
        } else if is_64bit {
            Reg::X(val)
        } else {
            Reg::W(val)
        }
    }
}

pub fn encode_mov_imm(rd: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();

    let mut instruction: u32 = if sf == 1 { 0xD2800000 } else { 0x52800000 };

    instruction |= (imm as u32) << 5; // imm16
    instruction |= rd_enc as u32; // Rd

    instruction
}

pub fn encode_ret() -> u32 {
    0xD65F03C0 // RET X30 (Link Register)
}

pub fn encode_add_imm(rd: Reg, rn: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0x91000000 } else { 0x11000000 };

    instruction |= (imm as u32) << 10; // imm12
    instruction |= (rn_enc as u32) << 5; // Rn
    instruction |= rd_enc as u32; // Rd

    instruction
}

pub fn encode_add_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0x8B000000 } else { 0x0B000000 };

    instruction |= (rm_enc as u32) << 16; // Rm
    instruction |= (rn_enc as u32) << 5; // Rn
    instruction |= rd_enc as u32; // Rd

    instruction
}

pub fn encode_stp_fp_lr() -> u32 {
    0xA9BF7BFD
}

pub fn encode_ldp_fp_lr() -> u32 {
    0xA8C17BFD
}

pub fn encode_str_imm(rt: Reg, rn: Reg, offset_bytes: u16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_str_imm supports only 64-bit X registers"
    );
    assert_eq!(
        offset_bytes % 8,
        0,
        "STR immediate offset must be 8-byte aligned"
    );
    let imm12 = (offset_bytes / 8) as u32;
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    0xF9000000 | (imm12 << 10) | (rn_enc << 5) | rt_enc
}

pub fn encode_ldr_imm(rt: Reg, rn: Reg, offset_bytes: u16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_ldr_imm supports only 64-bit X registers"
    );
    assert_eq!(
        offset_bytes % 8,
        0,
        "LDR immediate offset must be 8-byte aligned"
    );
    let imm12 = (offset_bytes / 8) as u32;
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    0xF9400000 | (imm12 << 10) | (rn_enc << 5) | rt_enc
}

pub fn encode_stur(rt: Reg, rn: Reg, offset_bytes: i16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_stur supports only 64-bit X registers"
    );
    assert!(
        matches!(rn, Reg::X(_) | Reg::SP),
        "encode_stur base must be Xn or SP"
    );
    assert!(
        (-256..=255).contains(&offset_bytes),
        "STUR offset must fit signed 9-bit"
    );
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    let imm9 = (offset_bytes as i32 & 0x1FF) as u32;
    0xF8000000 | (imm9 << 12) | (rn_enc << 5) | rt_enc
}

pub fn encode_ldur(rt: Reg, rn: Reg, offset_bytes: i16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_ldur supports only 64-bit X registers"
    );
    assert!(
        matches!(rn, Reg::X(_) | Reg::SP),
        "encode_ldur base must be Xn or SP"
    );
    assert!(
        (-256..=255).contains(&offset_bytes),
        "LDUR offset must fit signed 9-bit"
    );
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    let imm9 = (offset_bytes as i32 & 0x1FF) as u32;
    0xF8400000 | (imm9 << 12) | (rn_enc << 5) | rt_enc
}
pub fn encode_sub_imm(rd: Reg, rn: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0xD1000000 } else { 0x51000000 };

    instruction |= (imm as u32) << 10;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

pub fn encode_sub_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0xCB000000 } else { 0x4B000000 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

pub fn encode_mul_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0x9B007C00 } else { 0x1B007C00 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

pub fn encode_sdiv_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0x9AC00C00 } else { 0x1AC00C00 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

pub fn encode_bl(offset: i32) -> u32 {
    let imm26 = ((offset >> 2) & 0x3FFFFFF) as u32;
    0x94000000 | imm26
}

pub fn encode_blr(rn: Reg) -> u32 {
    let rn_enc = rn.encode();
    0xD63F0000 | ((rn_enc as u32) << 5)
}

pub fn encode_mov64(reg: Reg, value: u64) -> [u32; 4] {
    let reg_enc = reg.encode();

    let imm0 = (value & 0xFFFF) as u32;
    let imm1 = ((value >> 16) & 0xFFFF) as u32;
    let imm2 = ((value >> 32) & 0xFFFF) as u32;
    let imm3 = ((value >> 48) & 0xFFFF) as u32;

    let instr0 = 0xD2800000 | (0 << 21) | (imm0 << 5) | (reg_enc as u32); // MOVZ hw=0
    let instr1 = 0xF2800000 | (1 << 21) | (imm1 << 5) | (reg_enc as u32); // MOVK hw=1
    let instr2 = 0xF2800000 | (2 << 21) | (imm2 << 5) | (reg_enc as u32); // MOVK hw=2
    let instr3 = 0xF2800000 | (3 << 21) | (imm3 << 5) | (reg_enc as u32); // MOVK hw=3

    [instr0, instr1, instr2, instr3]
}

pub fn encode_cmp_imm(rn: Reg, imm: u16) -> u32 {
    let sf = match rn {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0xF1000000 } else { 0x71000000 };

    instruction |= (imm as u32) << 10;
    instruction |= (rn_enc as u32) << 5;
    instruction |= 31; // XZR destination

    instruction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_mov_imm() {
        assert_eq!(encode_mov_imm(Reg::X(0), 42), 0xD2800540);
        assert_eq!(encode_mov_imm(Reg::W(1), 10), 0x52800141);
    }

    #[test]
    fn test_encode_ret() {
        assert_eq!(encode_ret(), 0xD65F03C0);
    }

    #[test]
    fn test_encode_add_imm() {
        assert_eq!(encode_add_imm(Reg::X(0), Reg::X(0), 1), 0x91000400);
        assert_eq!(encode_add_imm(Reg::X(1), Reg::X(2), 10), 0x91002841);
    }

    #[test]
    fn test_encode_add_reg() {
        assert_eq!(encode_add_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x8B020020);
        assert_eq!(encode_add_reg(Reg::W(3), Reg::W(4), Reg::W(5)), 0x0B050083);
    }

    #[test]
    fn test_encode_prologue_epilogue() {
        assert_eq!(encode_stp_fp_lr(), 0xA9BF7BFD);
        assert_eq!(encode_ldp_fp_lr(), 0xA8C17BFD);
    }

    #[test]
    fn test_encode_sub_imm() {
        assert_eq!(encode_sub_imm(Reg::X(0), Reg::X(0), 1), 0xD1000400);
    }

    #[test]
    fn test_encode_sub_reg() {
        assert_eq!(encode_sub_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0xCB020020);
    }

    #[test]
    fn test_encode_mul_reg() {
        assert_eq!(encode_mul_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x9B027C20);
    }

    #[test]
    fn test_encode_sdiv_reg() {
        assert_eq!(encode_sdiv_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x9AC20C20);
    }

    #[test]
    fn test_encode_bl() {
        assert_eq!(
            encode_bl(0x1000),
            0x94000000 | ((0x1000 >> 2) & 0x3FFFFFF) as u32
        );
    }

    #[test]
    fn test_encode_blr() {
        assert_eq!(encode_blr(Reg::X(1)), 0xD63F0000 | (1 << 5));
    }

    #[test]
    fn test_encode_mov64() {
        let result = encode_mov64(Reg::X(0), 0x123456789ABCDEF0);
        assert_eq!(result[0], 0xD29BDE00);
        assert_eq!(result[1], 0xF2B35780);
        assert_eq!(result[2], 0xF2CACF00);
        assert_eq!(result[3], 0xF2E24680);
    }
}
