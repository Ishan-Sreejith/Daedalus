#![allow(dead_code)]

use crate::jit::encoder::{encode_mov_imm, Reg};

const LIST_LEN_OFFSET: i32 = 0;

const LIST_CAP_OFFSET: i32 = 8;

const LIST_DATA_OFFSET: i32 = 16;

pub struct HeapAllocator;

impl HeapAllocator {
    pub fn emit_list_alloc(capacity: u16) -> Vec<u8> {
        let mut code = Vec::new();

        let element_size = 8u32;
        let header_size = 24u32;
        let total_size = header_size + (capacity as u32 * element_size);

        if total_size <= u16::MAX as u32 {
            code.extend_from_slice(&encode_mov_imm(Reg::X(0), total_size as u16).to_le_bytes());
        } else {
            return code;
        }


        code
    }

    pub fn emit_list_store() -> Vec<u8> {
        let code = Vec::new();


        code
    }

    pub fn emit_list_load() -> Vec<u8> {
        let code = Vec::new();


        code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_allocator_list_alloc() {
        let code = HeapAllocator::emit_list_alloc(10);
        assert!(code.len() > 0);
    }
}
