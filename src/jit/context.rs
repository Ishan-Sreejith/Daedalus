
use crate::jit::memory::JitMemory;
use std::collections::HashMap;

pub struct JitContext {
    functions: HashMap<String, u64>,
    code_blocks: Vec<JitMemory>,
    literal_bytes: Vec<Vec<u8>>,
}

impl JitContext {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            code_blocks: Vec::new(),
            literal_bytes: Vec::new(),
        }
    }

    pub fn register_function(&mut self, name: &str, addr: u64) {
        self.functions.insert(name.to_string(), addr);
    }

    pub fn get_function_addr(&self, name: &str) -> Option<u64> {
        self.functions.get(name).copied()
    }

    pub fn add_code_block(&mut self, block: JitMemory) {
        self.code_blocks.push(block);
    }

    pub fn intern_bytes(&mut self, bytes: Vec<u8>) -> (*const u8, usize) {
        self.literal_bytes.push(bytes);
        let last = self.literal_bytes.last().expect("just pushed");
        (last.as_ptr(), last.len())
    }
}
