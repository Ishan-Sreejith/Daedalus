#![allow(dead_code)]

use std::collections::HashMap;

use crate::jit::branching::{
    encode_b, encode_b_eq, encode_b_gt, encode_b_lt, encode_b_ne, encode_cmp_reg,
};
use crate::jit::encoder::{
    encode_add_imm, encode_add_reg, encode_blr, encode_ldur, encode_mov64, encode_mov_imm,
    encode_mul_reg, encode_sdiv_reg, encode_stur, encode_sub_reg, Reg,
};

#[derive(Debug, Clone, Copy)]
pub enum ConditionCode {
    Eq, // Equal (Z set)
    Ne, // Not equal (Z clear)
    Lt, // Less than (N != V)
    Le, // Less than or equal (Z set or N != V)
    Gt, // Greater than (Z clear and N == V)
    Ge, // Greater than or equal (N == V)
}

impl ConditionCode {
    pub fn to_bits(self) -> u32 {
        match self {
            ConditionCode::Eq => 0b0000,
            ConditionCode::Ne => 0b0001,
            ConditionCode::Lt => 0b1011,
            ConditionCode::Le => 0b1101,
            ConditionCode::Gt => 0b1100,
            ConditionCode::Ge => 0b1010,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Location {
    Register(u8),
    Stack(i32), // Offset from FP (e.g., -8, -16)
}

pub struct RegisterMap {
    pub var_map: HashMap<String, Location>,
    reg_free: [bool; 8],
    stack_offset: i32,
}

impl RegisterMap {
    pub fn new() -> Self {
        Self {
            var_map: HashMap::new(),
            reg_free: [true; 8],
            stack_offset: -16, // Start after FP/LR save area
        }
    }

    pub fn alloc(&mut self, var: &str) -> Result<Location, String> {
        if let Some(&loc) = self.var_map.get(var) {
            return Ok(loc);
        }

        for i in 0..8 {
            if self.reg_free[i] {
                self.reg_free[i] = false;
                let loc = Location::Register(i as u8);
                self.var_map.insert(var.to_string(), loc);
                return Ok(loc);
            }
        }

        let offset = self.stack_offset;
        self.stack_offset -= 8; // 8 bytes per slot
        let loc = Location::Stack(offset);
        self.var_map.insert(var.to_string(), loc);
        Ok(loc)
    }

    pub fn free(&mut self, loc: Location) {
        if let Location::Register(reg) = loc {
            if (reg as usize) < 8 {
                self.reg_free[reg as usize] = true;
            }
        }
    }

    pub fn get(&self, var: &str) -> Option<Location> {
        self.var_map.get(var).copied()
    }

    pub fn clear(&mut self) {
        self.var_map.clear();
        self.reg_free = [true; 8];
        self.stack_offset = -16;
    }

    pub fn stack_frame_bytes(&self) -> u16 {
        if self.stack_offset >= -16 {
            return 0;
        }

        let used_i32 = (-self.stack_offset) - 8;
        if used_i32 <= 0 {
            return 0;
        }

        let mut bytes = if used_i32 > u16::MAX as i32 {
            u16::MAX
        } else {
            used_i32 as u16
        };

        let rem = bytes % 16;
        if rem != 0 {
            bytes = bytes.saturating_add(16 - rem);
        }

        bytes
    }
}

pub struct ArithmeticEncoder {
    buf: Vec<u8>,
}

impl ArithmeticEncoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn emit_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn emit_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }


    fn raw_ldr(&mut self, rt: u8, base: u8, offset: i32) {
        let base_reg = if base == 31 { Reg::SP } else { Reg::X(base) };
        match i16::try_from(offset) {
            Ok(off) if (-256..=255).contains(&off) => {
                self.emit_u32_le(encode_ldur(Reg::X(rt), base_reg, off))
            }
            Ok(off) => {
                for instr in encode_mov64(Reg::X(12), off as i64 as u64) {
                    self.emit_u32_le(instr);
                }
                self.emit_u32_le(encode_add_reg(Reg::X(12), Reg::X(12), base_reg));
                self.emit_u32_le(encode_ldur(Reg::X(rt), Reg::X(12), 0));
            }
            Err(_) => self.emit_u32_le(encode_mov_imm(Reg::X(rt), 0)),
        }
    }

    fn raw_str(&mut self, rt: u8, base: u8, offset: i32) {
        let base_reg = if base == 31 { Reg::SP } else { Reg::X(base) };
        if let Ok(off) = i16::try_from(offset) {
            if (-256..=255).contains(&off) {
                self.emit_u32_le(encode_stur(Reg::X(rt), base_reg, off));
            } else {
                for instr in encode_mov64(Reg::X(12), off as i64 as u64) {
                    self.emit_u32_le(instr);
                }
                self.emit_u32_le(encode_add_reg(Reg::X(12), Reg::X(12), base_reg));
                self.emit_u32_le(encode_stur(Reg::X(rt), Reg::X(12), 0));
            }
        }
    }


    pub fn load_to_reg(&mut self, target_reg: u8, loc: Location) {
        match loc {
            Location::Register(r) => {
                if r != target_reg {
                    self.emit_u32_le(encode_add_imm(Reg::X(target_reg), Reg::X(r), 0));
                }
            }
            Location::Stack(offset) => {
                self.raw_ldr(target_reg, 29, offset);
            }
        }
    }

    pub fn store_from_reg(&mut self, src_reg: u8, loc: Location) {
        match loc {
            Location::Register(r) => {
                if r != src_reg {
                    self.emit_u32_le(encode_add_imm(Reg::X(r), Reg::X(src_reg), 0));
                }
            }
            Location::Stack(offset) => {
                self.raw_str(src_reg, 29, offset);
            }
        }
    }


    pub fn emit_mov_imm(&mut self, dest: Location, imm: u16) {
        self.emit_u32_le(encode_mov_imm(Reg::X(9), imm));
        self.store_from_reg(9, dest);
    }

    pub fn emit_mov(&mut self, dest: Location, src: Location) {
        self.load_to_reg(9, src);
        self.store_from_reg(9, dest);
    }

    pub fn emit_add_imm(&mut self, dest: u8, src: u8, imm: u16) {
        self.emit_u32_le(encode_add_imm(Reg::X(dest), Reg::X(src), imm));
    }

    pub fn emit_add(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left); // x9 = left
        self.load_to_reg(10, right); // x10 = right
        self.emit_u32_le(encode_add_reg(Reg::X(9), Reg::X(9), Reg::X(10))); // x9 = x9 + x10
        self.store_from_reg(9, dest); // dest = x9
    }

    pub fn emit_sub(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_sub_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_mul(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_mul_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_div(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_sdiv_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_cmp(&mut self, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_cmp_reg(Reg::X(9), Reg::X(10)));
    }

    pub fn emit_cset(&mut self, dest: Location, cond: ConditionCode) {



        let inverted_cond = match cond {
            ConditionCode::Eq => 0b0001, // NE
            ConditionCode::Ne => 0b0000, // EQ
            ConditionCode::Lt => 0b1010, // GE
            ConditionCode::Le => 0b1100, // GT
            ConditionCode::Gt => 0b1101, // LE
            ConditionCode::Ge => 0b1011, // LT
        };

        let instr = 0x9A9F07E0 | (inverted_cond << 12) | (9u32 << 0); // Use x9 as temp
        self.emit_u32_le(instr);
        self.store_from_reg(9, dest);
    }

    pub fn emit_b(&mut self, offset: i32) {
        self.emit_u32_le(encode_b(offset));
    }
    pub fn emit_b_eq(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_eq(offset));
    }
    pub fn emit_b_ne(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_ne(offset));
    }
    pub fn emit_b_lt(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_lt(offset));
    }
    pub fn emit_b_gt(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_gt(offset));
    }

    pub fn emit_call(&mut self, addr: u64) {
        let instructions = encode_mov64(Reg::X(9), addr);
        for instr in instructions {
            self.emit_u32_le(instr);
        }
        self.emit_u32_le(encode_blr(Reg::X(9)));
    }

    pub fn emit_call_arg2(&mut self, addr: u64, arg0: u64, arg1: u64) {
        let instructions0 = encode_mov64(Reg::X(0), arg0);
        for instr in instructions0 {
            self.emit_u32_le(instr);
        }
        let instructions1 = encode_mov64(Reg::X(1), arg1);
        for instr in instructions1 {
            self.emit_u32_le(instr);
        }
        self.emit_call(addr);
    }

    pub fn move_to_phys_reg(&mut self, dest_phys: u8, src: Location) {
        self.load_to_reg(dest_phys, src);
    }

    pub fn move_from_phys_reg(&mut self, dest: Location, src_phys: u8) {
        self.store_from_reg(src_phys, dest);
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_map_alloc() {
        let mut map = RegisterMap::new();
        let r1 = map.alloc("x").unwrap();
        let r2 = map.alloc("y").unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_arithmetic_encoder_emit() {
        let mut enc = ArithmeticEncoder::new();
        enc.emit_mov_imm(Location::Register(0), 42);
        assert_eq!(enc.len(), 8); // mov x9, #42; mov x0, x9
    }
}
