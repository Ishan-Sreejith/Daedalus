#![allow(dead_code)]

use std::io;

use crate::jit::encoder::{encode_ldp_fp_lr, encode_mov_imm, encode_ret, encode_stp_fp_lr, Reg};
use crate::jit::memory::JitMemory;

pub struct CodeEmitter {
    buf: Vec<u8>,
}

impl CodeEmitter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn emit_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn emit_mov_imm(&mut self, rd: Reg, imm: u16) {
        self.emit_u32_le(encode_mov_imm(rd, imm));
    }

    pub fn emit_ret(&mut self) {
        self.emit_u32_le(encode_ret());
    }

    pub fn emit_prologue(&mut self) {
        self.emit_u32_le(encode_stp_fp_lr());
    }

    pub fn emit_epilogue(&mut self) {
        self.emit_u32_le(encode_ldp_fp_lr());
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
}

pub struct JitFunction {
    mem: JitMemory,
    len: usize,
}

impl JitFunction {
    pub fn from_returning_u16(value: u16) -> io::Result<Self> {
        let mut emitter = CodeEmitter::new();
        emitter.emit_prologue();
        emitter.emit_mov_imm(Reg::X(0), value);
        emitter.emit_epilogue();
        emitter.emit_ret();

        let bytes = emitter.into_bytes();
        let mut mem = JitMemory::new(bytes.len())?;
        mem.write_code(0, &bytes)?;
        mem.make_executable()?;

        Ok(Self {
            mem,
            len: bytes.len(),
        })
    }

    pub fn call_i64(&self) -> i64 {
        let func: extern "C" fn() -> i64 = unsafe { std::mem::transmute(self.mem.as_ptr()) };
        func()
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_trampoline_returns_value() {
        let jit = JitFunction::from_returning_u16(42).unwrap();
        assert_eq!(jit.call_i64(), 42);
    }
}
