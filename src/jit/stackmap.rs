#![allow(dead_code)]

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Safepoint {
    pub offset: usize,
    pub register_mask: u32,
    pub stack_slots: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct StackMap {
    pub name: String,
    pub safepoints: Vec<Safepoint>,
    pub frame_size: usize,
}

impl StackMap {
    pub fn new(name: &str, frame_size: usize) -> Self {
        Self {
            name: name.to_string(),
            safepoints: Vec::new(),
            frame_size,
        }
    }

    pub fn register_safepoint(&mut self, offset: usize, register_mask: u32, stack_slots: Vec<i32>) {
        self.safepoints.push(Safepoint {
            offset,
            register_mask,
            stack_slots,
        });
    }

    pub fn serialize(&self) -> String {
        format!("StackMap({},frame_size={})", self.name, self.frame_size)
    }
}

pub struct GCMetadata {
    var_types: HashMap<String, bool>,
}

impl GCMetadata {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
        }
    }

    pub fn mark_pointer(&mut self, var: &str) {
        self.var_types.insert(var.to_string(), true);
    }

    pub fn mark_value(&mut self, var: &str) {
        self.var_types.insert(var.to_string(), false);
    }

    pub fn is_pointer(&self, var: &str) -> bool {
        self.var_types.get(var).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safepoint_creation() {
        let sp = Safepoint {
            offset: 100,
            register_mask: 0b11, // x0 and x1 are pointers
            stack_slots: vec![0, 8],
        };
        assert_eq!(sp.offset, 100);
    }

    #[test]
    fn test_stack_map_creation() {
        let mut sm = StackMap::new("test_func", 64);
        sm.register_safepoint(32, 0b11, vec![0, 8]);
        assert_eq!(sm.safepoints.len(), 1);
    }

    #[test]
    fn test_gc_metadata() {
        let mut gc = GCMetadata::new();
        gc.mark_pointer("list");
        gc.mark_value("count");
        assert!(gc.is_pointer("list"));
        assert!(!gc.is_pointer("count"));
    }
}
