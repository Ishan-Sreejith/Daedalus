use std::collections::HashMap;
use std::fmt;
use std::io::{self, Read, Write};

#[derive(Default, Debug)]
struct Flags {
    n: bool, // Negative
    z: bool, // Zero
    c: bool, // Carry
    v: bool, // Overflow
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Reg(usize),
    Imm(i64),
    // For memory operands like [sp, #16]
    Mem {
        base: usize,
        offset: i64,
        writeback: bool,
    },
    // For indexed memory operands like [x21, x22, lsl #3]
    MemIndexed { base: usize, index: usize, lsl: u8 },
    // For adrp/add label combos
    Label(usize),
    // External symbol labels (for BL syscall shims) - we'll need to handle this differently
    LabelName(usize), // Changed from String to usize for Copy trait
}

#[derive(Debug, Clone, Copy)]
enum OpCode {
    ADD, SUB, MUL, SDIV, UDIV, MOD, MSUB, NEG, UXTW,
    FADD, FSUB, FMUL, FDIV, FMOV,
    AND, ORR, EOR, MVN, LSL, LSR,
    MOV, LDR, LDRB, STR, STRB, STP, LDP,
    B, BL, RET,
    CMP, CSET,
    BCond(Condition),
    CBZ, CBNZ,
    ADRP,
    SVC,
    NOP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Condition {
    EQ, NE, LT, GT, LE, GE,
    HI, LS, HS, LO, MI, PL, VS, VC,
}

#[derive(Debug, Clone)]
pub(crate) struct Instruction {
    opcode: OpCode,
    // Using Vec for flexibility; most instructions use 2-3 operands.
    operands: Vec<Operand>,
}

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.opcode)?;
        if !self.operands.is_empty() {
            write!(f, " {:?}", self.operands)?;
        }
        Ok(())
    }
}

/// ARM64 Virtual Machine
pub struct VM {
    /// General purpose registers x0-x30 and SP (x31)
    pub registers: [i64; 32],
    /// Floating point registers d0-d31
    pub fp_registers: [f64; 32],
    /// Stack pointer (sp)
    pub sp: i64,
    /// Program counter
    pub pc: i64,
    /// Memory (2MB: 1MB for stack, 1MB for heap/data)
    pub memory: Vec<u8>,
    /// Data segment offset in memory
    pub data_segment_offset: usize,
    /// Labels for jumps (code and data)
    pub labels: HashMap<String, usize>,
    /// Label names lookup table (for LabelName operands)
    pub label_names: HashMap<usize, String>,
    /// Program instructions (now in a bytecode format)
    pub program: Vec<Instruction>,
    /// For debugging: map PC to original source line
    pub debug_info: HashMap<usize, String>,
    /// Execution state
    pub running: bool,
    /// Step mode
    pub step_mode: bool,
    /// Heap allocator pointer
    pub heap_ptr: usize,
    /// Tracked heap allocations (ptr -> size)
    allocations: HashMap<usize, usize>,
    /// Freed heap blocks available for reuse (ptr, size)
    free_list: Vec<(usize, usize)>,
    /// Condition flags
    flags: Flags,
}

impl VM {
    pub fn new() -> Self {
        VM {
            registers: [0; 32],
            fp_registers: [0.0; 32],
            sp: (1024 * 1024) as i64, // Stack starts at 1MB and grows down
            pc: 0,
            memory: vec![0; 2 * 1024 * 1024],
            data_segment_offset: 1024 * 1024, // Data segment starts at 1MB
            labels: HashMap::new(),
            label_names: HashMap::new(),
            program: Vec::new(),
            debug_info: HashMap::new(),
            running: false,
            step_mode: false,
            heap_ptr: 1024 * 1024, // Heap starts at 1MB
            allocations: HashMap::new(),
            free_list: Vec::new(),
            flags: Flags::default(),
        }
    }

    pub fn load_program(&mut self, asm: &str) -> Result<(), String> {
        // Reset state
        self.reset_state();

        let mut text_section_lines = Vec::new();
        let mut data_section_lines = Vec::new();
        let mut current_section = ".text";

        for line in asm.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") || line.starts_with(".global") {
                continue;
            }
            if line == ".data" { current_section = ".data"; continue; }
            if line == ".text" { current_section = ".text"; continue; }

            if current_section == ".data" {
                data_section_lines.push(line);
            } else {
                text_section_lines.push(line);
            }
        }

        // Process data section first to populate data labels
        self.process_data_section(&data_section_lines)?;

        // Process text section to populate code labels and then parse instructions
        self.process_text_section(&text_section_lines)?;

        self.pc = self.labels.get("_main").cloned().unwrap_or(0) as i64;
        // Treat returning from _main as program termination.
        self.registers[30] = self.program.len() as i64;
        self.running = true;
        Ok(())
    }

    fn reset_state(&mut self) {
        self.registers = [0; 32];
        self.fp_registers = [0.0; 32];
        self.sp = (1024 * 1024) as i64;
        self.pc = 0;
        self.running = false;
        self.step_mode = false;
        self.heap_ptr = 1024 * 1024;
        self.allocations.clear();
        self.free_list.clear();
        self.memory.fill(0);
        self.program.clear();
        self.labels.clear();
        self.label_names.clear();
        self.debug_info.clear();
    }

    fn process_data_section(&mut self, lines: &[&str]) -> Result<(), String> {
        let mut data_ptr = self.heap_ptr;
        for line in lines {
            if let Some(rest) = line.strip_prefix(".align") {
                let n = rest.trim().parse::<usize>().unwrap_or(0);
                let align = 1usize.checked_shl(n as u32).unwrap_or(1).max(1);
                data_ptr = (data_ptr + align - 1) & !(align - 1);
                continue;
            }

            if let Some(first) = line.split_whitespace().next() {
                if first.ends_with(':') {
                    let label = first.trim_end_matches(':').trim().to_string();
                    self.labels.insert(label, data_ptr);
                    let rest = line[first.len()..].trim();
                    if !rest.is_empty() {
                        write_data_directive(self, rest, &mut data_ptr)?;
                    }
                    continue;
                }
            }

            if line.starts_with(".asciz") || line.starts_with(".quad") {
                write_data_directive(self, line, &mut data_ptr)?;
                continue;
            }
        }
        self.heap_ptr = (data_ptr + 15) & !15;
        Ok(())
    }

    fn process_text_section(&mut self, lines: &[&str]) -> Result<(), String> {
        // First pass: find all code labels
        let mut line_num = 0;
        for line in lines {
            if line.starts_with('.') && !line.ends_with(':') { continue; }
            if line.ends_with(':') {
                let label = line.trim_end_matches(':').to_string();
                self.labels.insert(label, line_num);
            } else {
                line_num += 1;
            }
        }

        // Second pass: parse instructions
        for line in lines {
            if line.ends_with(':') || (line.starts_with('.') && !line.ends_with(':')) {
                continue;
            }
            let pc = self.program.len();
            let instr = self.parse_instruction(line)?;
            self.program.push(instr);
            self.debug_info.insert(pc, line.to_string());
        }
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), String> {
        while self.running && (self.pc as usize) < self.program.len() {
            self.step()?;
            if self.step_mode { break; }
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        let pc = self.pc as usize;
        if pc >= self.program.len() {
            self.running = false;
            return Ok(());
        }
        let instr = self.program[pc].clone();
        self.execute_instruction(&instr)?;
        Ok(())
    }
    
    fn execute_instruction(&mut self, instr: &Instruction) -> Result<(), String> {
        use OpCode::*;
        let ops = &instr.operands;

        match instr.opcode {
            ADD => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_add(self.get_op(ops[2])?)),
            SUB => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_sub(self.get_op(ops[2])?)),
            MUL => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_mul(self.get_op(ops[2])?)),
            SDIV => {
                let v2 = self.get_op(ops[2])?;
                if v2 == 0 { return Err("Division by zero".to_string()); }
                self.set_reg_op(ops[0], self.get_op(ops[1])? / v2);
            }
            UDIV => {
                let v2 = self.get_op(ops[2])? as u64;
                if v2 == 0 { self.set_reg_op(ops[0], 0); }
                else { self.set_reg_op(ops[0], ((self.get_op(ops[1])? as u64) / v2) as i64); }
            }
            MOD => {
                let v2 = self.get_op(ops[2])?;
                if v2 == 0 { return Err("Modulo by zero".to_string()); }
                self.set_reg_op(ops[0], self.get_op(ops[1])? % v2);
            }
            MSUB => self.set_reg_op(ops[0], self.get_op(ops[3])?.wrapping_sub(self.get_op(ops[1])?.wrapping_mul(self.get_op(ops[2])?))),
            NEG => self.set_reg_op(ops[0], 0i64.wrapping_sub(self.get_op(ops[1])?)),
            UXTW => self.set_reg_op(ops[0], (self.get_op(ops[1])? as u64 & 0xFFFF_FFFF) as i64),
            FADD => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? + self.get_fp_op(ops[2])?),
            FSUB => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? - self.get_fp_op(ops[2])?),
            FMUL => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? * self.get_fp_op(ops[2])?),
            FDIV => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? / self.get_fp_op(ops[2])?),
            FMOV => {
                if let Operand::Reg(d) = ops[0] { // fmov d, x
                    self.set_fp_reg(d, f64::from_bits(self.get_op(ops[1])? as u64));
                } else { // fmov x, d
                    self.set_reg_op(ops[0], self.get_fp_op(ops[1])?.to_bits() as i64);
                }
            }
            AND => self.set_reg_op(ops[0], self.get_op(ops[1])? & self.get_op(ops[2])?),
            ORR => self.set_reg_op(ops[0], self.get_op(ops[1])? | self.get_op(ops[2])?),
            EOR => self.set_reg_op(ops[0], self.get_op(ops[1])? ^ self.get_op(ops[2])?),
            MVN => self.set_reg_op(ops[0], !self.get_op(ops[1])?),
            LSL => self.set_reg_op(ops[0], self.get_op(ops[1])? << self.get_op(ops[2])?),
            LSR => self.set_reg_op(ops[0], ((self.get_op(ops[1])? as u64) >> self.get_op(ops[2])?) as i64),
            MOV => self.set_reg_op(ops[0], self.get_op(ops[1])?),
            LDR => {
                let addr = self.get_addr(ops[1])?;
                let val = self.read_i64(addr)?;
                self.set_reg_op(ops[0], val);
            }
            LDRB => {
                let addr = self.get_addr(ops[1])?;
                self.set_reg_op(ops[0], self.read_u8(addr)? as i64);
            }
            STR => {
                let val = self.get_op(ops[0])?;
                let addr = self.get_addr(ops[1])?;
                self.write_i64(addr, val)?;
            }
            STRB => {
                let val = (self.get_op(ops[0])? & 0xFF) as u8;
                let addr = self.get_addr(ops[1])?;
                self.write_u8(addr, val)?;
            }
            STP => {
                let val1 = self.get_op(ops[0])?;
                let val2 = self.get_op(ops[1])?;
                let addr = self.get_addr(ops[2])?;
                self.write_i64(addr, val1)?;
                self.write_i64(addr + 8, val2)?;
                if let Operand::Mem { base, offset, .. } = ops[2] {  // Add .. to ignore writeback
                    if let Some(arg_str) = self.debug_info.get(&(self.pc as usize)) {
                        if arg_str.contains('!') || arg_str.contains("],") {
                            let writeback = if ops.len() > 3 {
                                match ops[3] {
                                    Operand::Imm(imm) => imm,
                                    _ => offset,
                                }
                            } else {
                                offset
                            };
                            self.set_reg(base, self.get_reg(base) + writeback);
                        }
                    }
                }
            }
            LDP => {
                let addr = self.get_addr(ops[2])?;
                let val1 = self.read_i64(addr)?;
                let val2 = self.read_i64(addr + 8)?;
                self.set_reg_op(ops[0], val1);
                self.set_reg_op(ops[1], val2);
                if let Operand::Mem { base, offset, .. } = ops[2] {  // Add .. to ignore writeback
                    if let Some(arg_str) = self.debug_info.get(&(self.pc as usize)) {
                        if arg_str.contains("],") || arg_str.contains('!') {
                            let writeback = if ops.len() > 3 {
                                match ops[3] {
                                    Operand::Imm(imm) => imm,
                                    _ => offset,
                                }
                            } else {
                                offset
                            };
                            self.set_reg(base, self.get_reg(base) + writeback);
                        }
                    }
                }
            }
            B => { self.pc = self.get_op(ops[0])? - 1; }
            BL => {
                self.registers[30] = self.pc + 1;
                match ops[0] {
                    Operand::Label(target) => {
                        self.pc = target as i64 - 1;
                    }
                    Operand::LabelName(name_id) => {
                        // Fixed borrow checker issue by cloning
                        if let Some(label_name) = self.label_names.get(&name_id).cloned() {
                            self.exec_bl_syscall(&label_name)?;
                        } else {
                            // Fallback to parsing from debug info
                            let label_name = self
                                .debug_info
                                .get(&(self.pc as usize))
                                .and_then(|line| line.split_whitespace().nth(1))
                                .ok_or_else(|| "Malformed BL instruction".to_string())?
                                .to_string();
                            self.exec_bl_syscall(&label_name)?;
                        }
                    }
                    _ => return Err("Invalid BL operand".to_string()),
                }
            }
            RET => { self.pc = self.get_reg(30) - 1; }
            CMP => {
                let val1 = self.get_op(ops[0])?;
                let val2 = self.get_op(ops[1])?;
                let (result, overflow) = val1.overflowing_sub(val2);
                self.flags.z = result == 0;
                self.flags.n = result < 0;
                self.flags.c = (val1 as u64) >= (val2 as u64);
                self.flags.v = overflow;
            }
            CSET => {
                if ops.len() < 2 {
                    return Err("CSET expects destination and condition".to_string());
                }
                let cond = self.decode_condition_operand(ops[1])?;
                let val = self.check_condition(cond)?;
                self.set_reg_op(ops[0], if val { 1 } else { 0 });
            }
            BCond(cond) => {
                if self.check_condition(cond)? { self.pc = self.get_op(ops[0])? - 1; }
            }
            CBZ => {
                if self.get_op(ops[0])? == 0 { self.pc = self.get_op(ops[1])? - 1; }
            }
            CBNZ => {
                if self.get_op(ops[0])? != 0 { self.pc = self.get_op(ops[1])? - 1; }
            }
            ADRP => {
                // Simplified ADRP: materialize page(base) for a label.
                // ADD with @PAGEOFF will add the low 12 bits.
                self.set_reg_op(ops[0], self.get_op(ops[1])? & !0xfff);
            }
            SVC => self.exec_svc()?,
            NOP => {},
        }
        self.pc += 1;
        Ok(())
    }

    fn get_op(&self, op: Operand) -> Result<i64, String> {
        match op {
            Operand::Reg(idx) => Ok(self.get_reg(idx)),
            Operand::Imm(val) => Ok(val),
            Operand::Label(addr) => Ok(addr as i64),
            _ => Err("Invalid operand type for integer operation".to_string()),
        }
    }

    fn get_fp_op(&self, op: Operand) -> Result<f64, String> {
        match op {
            Operand::Reg(idx) => Ok(self.get_fp_reg(idx)),
            _ => Err("Invalid operand type for float operation".to_string()),
        }
    }

    fn set_reg_op(&mut self, op: Operand, val: i64) {
        if let Operand::Reg(idx) = op { self.set_reg(idx, val); }
    }
    fn set_fp_reg_op(&mut self, op: Operand, val: f64) {
        if let Operand::Reg(idx) = op { self.set_fp_reg(idx, val); }
    }

    fn get_addr(&self, op: Operand) -> Result<usize, String> {
        match op {
            Operand::Mem { base, offset, .. } => {  // Add .. to ignore writeback
                let addr = self.get_reg(base) + offset;
                if addr < 0 {
                    return Err(format!("Memory address underflow: {}", addr));
                }
                Ok(addr as usize)
            }
            Operand::MemIndexed { base, index, lsl } => {
                let scaled = self.get_reg(index).wrapping_shl(lsl as u32);
                let addr = self.get_reg(base).wrapping_add(scaled);
                if addr < 0 {
                    return Err(format!("Memory address underflow: {}", addr));
                }
                Ok(addr as usize)
            }
            _ => Err("Operand is not a memory address".to_string()),
        }
    }

    fn check_mem_range(&self, addr: usize, len: usize) -> Result<(), String> {
        if addr
            .checked_add(len)
            .map(|end| end <= self.memory.len())
            .unwrap_or(false)
        {
            Ok(())
        } else {
            Err(format!(
                "Memory access out of bounds at 0x{:x} (len {})",
                addr, len
            ))
        }
    }

    fn read_u8(&self, addr: usize) -> Result<u8, String> {
        self.check_mem_range(addr, 1)?;
        Ok(self.memory[addr])
    }

    fn write_u8(&mut self, addr: usize, value: u8) -> Result<(), String> {
        self.check_mem_range(addr, 1)?;
        self.memory[addr] = value;
        Ok(())
    }

    fn read_i64(&self, addr: usize) -> Result<i64, String> {
        self.check_mem_range(addr, 8)?;
        Ok(i64::from_le_bytes(
            self.memory[addr..addr + 8]
                .try_into()
                .map_err(|_| "Invalid i64 read".to_string())?,
        ))
    }

    fn write_i64(&mut self, addr: usize, value: i64) -> Result<(), String> {
        self.check_mem_range(addr, 8)?;
        self.memory[addr..addr + 8].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    fn parse_reg(&self, s: &str) -> Result<usize, String> {
        let s = s.trim();
        if s == "xzr" || s == "wzr" {
            Ok(usize::MAX)
        } else if s == "sp" {
            Ok(31)
        } else if s.starts_with('x') || s.starts_with('w') || s.starts_with('d') || s.starts_with('s') {
            s[1..].parse::<usize>().map_err(|_| format!("Invalid register format: {}", s))
        } else {
            Err(format!("Invalid register: {}", s))
        }
    }

    fn parse_imm(&self, s: &str) -> Result<i64, String> {
        let s = s.trim().trim_start_matches('#');
        if let Some(hex) = s.strip_prefix("0x") {
            i64::from_str_radix(hex, 16).map_err(|_| format!("Invalid immediate: {}", s))
        } else if let Some(hex) = s.strip_prefix("-0x") {
            i64::from_str_radix(hex, 16).map(|v| -v).map_err(|_| format!("Invalid immediate: {}", s))
        } else {
            s.parse::<i64>().map_err(|_| format!("Invalid immediate: {}", s))
        }
    }

    fn parse_condition_token(&self, s: &str) -> Result<Condition, String> {
        let t = s.trim().trim_start_matches('.').to_ascii_lowercase();
        let c = match t.as_str() {
            "eq" => Condition::EQ,
            "ne" => Condition::NE,
            "lt" => Condition::LT,
            "gt" => Condition::GT,
            "le" => Condition::LE,
            "ge" => Condition::GE,
            "hi" => Condition::HI,
            "ls" => Condition::LS,
            "hs" | "cs" => Condition::HS,
            "lo" | "cc" => Condition::LO,
            "mi" => Condition::MI,
            "pl" => Condition::PL,
            "vs" => Condition::VS,
            "vc" => Condition::VC,
            _ => return Err(format!("Unknown condition code: {}", s)),
        };
        Ok(c)
    }

    fn condition_to_code(cond: Condition) -> i64 {
        match cond {
            Condition::EQ => 0,
            Condition::NE => 1,
            Condition::LT => 2,
            Condition::GT => 3,
            Condition::LE => 4,
            Condition::GE => 5,
            Condition::HI => 6,
            Condition::LS => 7,
            Condition::HS => 8,
            Condition::LO => 9,
            Condition::MI => 10,
            Condition::PL => 11,
            Condition::VS => 12,
            Condition::VC => 13,
        }
    }

    fn code_to_condition(code: i64) -> Result<Condition, String> {
        match code {
            0 => Ok(Condition::EQ),
            1 => Ok(Condition::NE),
            2 => Ok(Condition::LT),
            3 => Ok(Condition::GT),
            4 => Ok(Condition::LE),
            5 => Ok(Condition::GE),
            6 => Ok(Condition::HI),
            7 => Ok(Condition::LS),
            8 => Ok(Condition::HS),
            9 => Ok(Condition::LO),
            10 => Ok(Condition::MI),
            11 => Ok(Condition::PL),
            12 => Ok(Condition::VS),
            13 => Ok(Condition::VC),
            _ => Err(format!("Invalid condition code: {}", code)),
        }
    }

    fn decode_condition_operand(&self, op: Operand) -> Result<Condition, String> {
        match op {
            Operand::Imm(code) => Self::code_to_condition(code),
            _ => Err("Condition operand must be an immediate condition code".to_string()),
        }
    }

    fn get_reg(&self, idx: usize) -> i64 {
        if idx == usize::MAX { 0 } else if idx == 31 { self.sp } else { self.registers[idx] }
    }
    fn set_reg(&mut self, idx: usize, val: i64) {
        if idx == usize::MAX { return; }
        if idx == 31 { self.sp = val; } else { self.registers[idx] = val; }
    }
    fn get_fp_reg(&self, idx: usize) -> f64 { self.fp_registers[idx] }
    fn set_fp_reg(&mut self, idx: usize, val: f64) { self.fp_registers[idx] = val; }

    fn parse_mem_operand(&self, operand: &str) -> Result<Operand, String> {
        let operand = operand
            .trim()
            .trim_end_matches('!')
            .trim_start_matches('[')
            .trim_end_matches(']');
        let parts: Vec<&str> = operand.split(',').map(|s| s.trim()).collect();
        let base_reg_idx = self.parse_reg(parts[0])?;
        let mut offset = 0i64;
        if parts.len() > 1 {
            if parts[1].starts_with('#') {
                offset = self.parse_imm(parts[1])?;
            } else if parts.len() == 2 {
                let index_reg_idx = self.parse_reg(parts[1])?;
                return Ok(Operand::MemIndexed {
                    base: base_reg_idx,
                    index: index_reg_idx,
                    lsl: 0,
                });
            } else if parts.len() >= 3 {
                let index_reg_idx = self.parse_reg(parts[1])?;
                let lsl = parts[2]
                    .strip_prefix("lsl")
                    .map(str::trim)
                    .ok_or_else(|| "Only lsl scaling is supported for indexed memory operands".to_string())?;
                let shift = self.parse_imm(lsl)? as u8;
                return Ok(Operand::MemIndexed {
                    base: base_reg_idx,
                    index: index_reg_idx,
                    lsl: shift,
                });
            } else {
                // [base, index, lsl #3] not supported by this simplified parser, but could be added.
                return Err("Indexed memory operands not supported in this VM version".to_string());
            }
        }
        Ok(Operand::Mem { base: base_reg_idx, offset, writeback: false })  // Add writeback field
    }

    fn check_condition(&self, cond: Condition) -> Result<bool, String> {
        Ok(match cond {
            Condition::EQ => self.flags.z,
            Condition::NE => !self.flags.z,
            Condition::LT => self.flags.n != self.flags.v,
            Condition::GT => !self.flags.z && (self.flags.n == self.flags.v),
            Condition::LE => self.flags.z || (self.flags.n != self.flags.v),
            Condition::GE => self.flags.n == self.flags.v,
            Condition::HI => self.flags.c && !self.flags.z,
            Condition::LS => !self.flags.c || self.flags.z,
            Condition::HS => self.flags.c,
            Condition::LO => !self.flags.c,
            Condition::MI => self.flags.n,
            Condition::PL => !self.flags.n,
            Condition::VS => self.flags.v,
            Condition::VC => !self.flags.v,
        })
    }
    
    fn exec_bl_syscall(&mut self, label: &str) -> Result<(), String> {
        match label {
            "_malloc" => {
                let size = self.registers[0].max(0) as usize;
                let ptr = self.vm_malloc(size)?;
                self.registers[0] = ptr as i64;
            }
            "_free" => {
                let ptr = self.registers[0] as usize;
                self.vm_free(ptr);
                self.registers[0] = 0;
            }
            "_realloc" => {
                let old_ptr = self.registers[0] as usize;
                let new_size = self.registers[1].max(0) as usize;
                let new_ptr = self.vm_realloc(old_ptr, new_size)?;
                self.registers[0] = new_ptr as i64;
            }
            "_fflush" => { self.registers[0] = 0; }
            "_printf" => self.vm_printf()?,
            _ => return Err(format!("Unknown syscall label: {}", label)),
        }
        Ok(())
    }

    fn exec_svc(&mut self) -> Result<(), String> {
        match self.registers[16] {
            1 => { self.running = false; }
            3 => { // read
                let fd = self.registers[0];
                let buf_ptr = self.registers[1] as usize;
                let len = self.registers[2] as usize;
                if fd == 0 {
                    let mut buffer = vec![0; len];
                    let bytes_read = io::stdin().read(&mut buffer).map_err(|e| e.to_string())?;
                    if buf_ptr + bytes_read <= self.memory.len() {
                        self.memory[buf_ptr..buf_ptr + bytes_read].copy_from_slice(&buffer[..bytes_read]);
                        self.registers[0] = bytes_read as i64;
                    } else {
                        return Err(format!("Memory access out of bounds for SVC read at 0x{:x}", buf_ptr));
                    }
                }
            }
            4 => { // write
                let fd = self.registers[0];
                let buf_ptr = self.registers[1] as usize;
                let len = self.registers[2] as usize;
                if fd == 1 {
                    let end = (buf_ptr + len).min(self.memory.len());
                    print!("{}", String::from_utf8_lossy(&self.memory[buf_ptr..end]));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn vm_malloc(&mut self, size: usize) -> Result<usize, String> {
        let aligned = (size + 15) & !15;
        if aligned == 0 { return Ok(0); }

        // First-fit from free list.
        if let Some((idx, &(ptr, block_size))) = self
            .free_list
            .iter()
            .enumerate()
            .find(|(_, (_, block_size))| *block_size >= aligned)
        {
            self.free_list.swap_remove(idx);
            if block_size > aligned {
                self.free_list.push((ptr + aligned, block_size - aligned));
            }
            self.allocations.insert(ptr, aligned);
            self.coalesce_free_list();
            return Ok(ptr);
        }

        let ptr = (self.heap_ptr + 15) & !15;
        let end = ptr.saturating_add(aligned);
        if end > self.memory.len() { return Err(format!("Out of VM memory allocating {} bytes", aligned)); }
        self.heap_ptr = end;
        self.allocations.insert(ptr, aligned);
        Ok(ptr)
    }

    fn vm_free(&mut self, ptr: usize) {
        if let Some(size) = self.allocations.remove(&ptr) {
            self.free_list.push((ptr, size));
            self.coalesce_free_list();
        }
    }

    fn vm_realloc(&mut self, old_ptr: usize, new_size: usize) -> Result<usize, String> {
        if old_ptr == 0 { return self.vm_malloc(new_size); }
        let old_size = self.allocations.get(&old_ptr).copied().unwrap_or(0);
        let new_ptr = self.vm_malloc(new_size)?;
        let copy_len = old_size.min(new_size);
        if copy_len > 0 {
            if old_ptr + copy_len > self.memory.len() || new_ptr + copy_len > self.memory.len() {
                return Err("Realloc copy out of bounds".to_string());
            }
            let src = self.memory[old_ptr..old_ptr + copy_len].to_vec();
            self.memory[new_ptr..new_ptr + copy_len].copy_from_slice(&src);
        }
        self.vm_free(old_ptr);
        Ok(new_ptr)
    }

    fn coalesce_free_list(&mut self) {
        if self.free_list.len() <= 1 {
            return;
        }
        self.free_list.sort_by_key(|(ptr, _)| *ptr);
        let mut merged: Vec<(usize, usize)> = Vec::with_capacity(self.free_list.len());
        for (ptr, size) in self.free_list.drain(..) {
            if let Some((last_ptr, last_size)) = merged.last_mut() {
                let last_end = *last_ptr + *last_size;
                if last_end == ptr {
                    *last_size += size;
                    continue;
                }
            }
            merged.push((ptr, size));
        }
        self.free_list = merged;
    }

    fn read_c_string(&self, addr: usize) -> Result<String, String> {
        if addr >= self.memory.len() { return Err(format!("CString pointer out of bounds: 0x{:x}", addr)); }
        let mut bytes = Vec::new();
        let mut i = addr;
        while i < self.memory.len() && self.memory[i] != 0 {
            bytes.push(self.memory[i]);
            i += 1;
        }
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn vm_printf(&mut self) -> Result<(), String> {
        let fmt_ptr = self.registers[0] as usize;
        let fmt = self.read_c_string(fmt_ptr)?;
        let sp = self.sp as usize;
        if sp + 8 > self.memory.len() { return Err("Stack out of bounds in _printf".to_string()); }
        let arg = i64::from_le_bytes(self.memory[sp..sp + 8].try_into().unwrap());

        if fmt.contains("%ld") {
            let output = fmt.replace("%ld", &arg.to_string());
            print!("{}", output);
            io::stdout().flush().ok();
            self.registers[0] = 0;
            Ok(())
        } else {
            Err(format!("Unsupported _printf format: {}", fmt))
        }
    }

    pub fn print_registers(&self) {
        println!("\n=== Registers ===");
        for i in 0..31 {
            if i % 4 == 0 { println!(); }
            print!("x{:02}: {:<16}  ", i, self.registers[i]);
        }
        println!("\npc:  {:<16}  sp: {:<16}", self.pc, self.sp);
        println!("Flags: N:{} Z:{} C:{} V:{}", self.flags.n as u8, self.flags.z as u8, self.flags.c as u8, self.flags.v as u8);
        println!("================\n");
    }

    fn intern_label_name(&mut self, label: &str) -> usize {
        if let Some((id, _)) = self.label_names.iter().find(|(_, name)| name.as_str() == label) {
            return *id;
        }
        let next_id = self
            .label_names
            .keys()
            .max()
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        self.label_names.insert(next_id, label.to_string());
        next_id
    }

    fn parse_instruction(&mut self, line: &str) -> Result<Instruction, String> {
        let line = line.trim();
        if line.is_empty() {
            return Ok(Instruction { opcode: OpCode::NOP, operands: vec![] });
        }
        let (mnemonic, args_str) = match line.find(char::is_whitespace) {
            Some(i) => (&line[..i], line[i..].trim()),
            None => (line, ""),
        };
        let args: Vec<String> = if args_str.is_empty() {
            Vec::new()
        } else {
            split_asm_args(args_str)
        };

        let mut operands = Vec::new();
        for arg in &args {
            if arg.starts_with('[') {
                operands.push(self.parse_mem_operand(arg)?);
            } else if arg.starts_with('#') {
                operands.push(Operand::Imm(self.parse_imm(arg)?));
            } else if self.labels.contains_key(arg) {
                operands.push(Operand::Label(self.labels[arg]));
            } else if arg.starts_with('x') || arg.starts_with('w') || arg.starts_with('d') || arg.starts_with('s') || arg == "sp" {
                operands.push(Operand::Reg(self.parse_reg(arg)?));
            } else if let Ok(imm) = self.parse_imm(arg) {
                // For things like `svc #0x80` where the '#' is optional in some assemblers
                operands.push(Operand::Imm(imm));
            } else {
                // Symbolic operand (e.g. BL _printf)
                let name_id = self.intern_label_name(arg);
                operands.push(Operand::LabelName(name_id));
            }
        }

        let opcode = match mnemonic {
            "add" => {
                // Handle `add x0, x0, .L0@PAGEOFF` from ADRP lowering.
                if args.len() >= 3 && args[2].contains("@PAGEOFF") {
                    let dest = self.parse_reg(&args[0])?;
                    let base = self.parse_reg(&args[1])?;
                    let label_part = args[2].split('@').next().unwrap();
                    let label_addr = *self
                        .labels
                        .get(label_part)
                        .ok_or_else(|| format!("Label not found: {}", label_part))?;
                    operands = vec![
                        Operand::Reg(dest),
                        Operand::Reg(base),
                        Operand::Imm((label_addr as i64) & 0xfff),
                    ];
                }
                OpCode::ADD
            }
            "sub" => OpCode::SUB, "mul" => OpCode::MUL, "sdiv" => OpCode::SDIV, "udiv" => OpCode::UDIV,
            "mod" => OpCode::MOD,
            "msub" => OpCode::MSUB, "neg" => OpCode::NEG, "uxtw" => OpCode::UXTW,
            "fadd" => OpCode::FADD, "fsub" => OpCode::FSUB, "fmul" => OpCode::FMUL, "fdiv" => OpCode::FDIV,
            "fmov" => OpCode::FMOV,
            "and" => OpCode::AND, "orr" => OpCode::ORR, "eor" => OpCode::EOR, "mvn" => OpCode::MVN,
            "lsl" => OpCode::LSL, "lsr" => OpCode::LSR,
            "mov" => OpCode::MOV, "ldr" => OpCode::LDR, "ldrb" => OpCode::LDRB, "str" => OpCode::STR,
            "strb" => OpCode::STRB, "stp" => OpCode::STP, "ldp" => OpCode::LDP,
            "b" => OpCode::B, "bl" => OpCode::BL, "ret" => OpCode::RET,
            "cmp" => OpCode::CMP,
            "cset" => {
                if args.len() != 2 {
                    return Err("cset expects 2 operands: cset <reg>, <cond>".to_string());
                }
                let dest = self.parse_reg(&args[0])?;
                let cond = self.parse_condition_token(&args[1])?;
                operands = vec![
                    Operand::Reg(dest),
                    Operand::Imm(Self::condition_to_code(cond)),
                ];
                OpCode::CSET
            }
            "b.eq" => OpCode::BCond(Condition::EQ), "b.ne" => OpCode::BCond(Condition::NE),
            "b.lt" => OpCode::BCond(Condition::LT), "b.gt" => OpCode::BCond(Condition::GT),
            "b.le" => OpCode::BCond(Condition::LE), "b.ge" => OpCode::BCond(Condition::GE),
            "b.hi" => OpCode::BCond(Condition::HI), "b.ls" => OpCode::BCond(Condition::LS),
            "b.hs" | "b.cs" => OpCode::BCond(Condition::HS),
            "b.lo" | "b.cc" => OpCode::BCond(Condition::LO),
            "b.mi" => OpCode::BCond(Condition::MI), "b.pl" => OpCode::BCond(Condition::PL),
            "b.vs" => OpCode::BCond(Condition::VS), "b.vc" => OpCode::BCond(Condition::VC),
            "cbz" => OpCode::CBZ, "cbnz" => OpCode::CBNZ,
            "adrp" => {
                if args.len() >= 2 && args[1].contains("@PAGE") {
                    let dest = self.parse_reg(&args[0])?;
                    let label_part = args[1].split('@').next().unwrap();
                    let label_addr = *self
                        .labels
                        .get(label_part)
                        .ok_or_else(|| format!("Label not found: {}", label_part))?;
                    operands = vec![Operand::Reg(dest), Operand::Imm(label_addr as i64)];
                }
                OpCode::ADRP
            }
            "svc" => OpCode::SVC,
            "nop" => OpCode::NOP,
            _ => return Err(format!("Unknown instruction: {}", mnemonic)),
        };

        Ok(Instruction { opcode, operands })
    }

    pub fn append_instruction(&mut self, line: &str) -> Result<usize, String> {
        let instr = self.parse_instruction(line)?;
        let pc = self.program.len();
        self.program.push(instr);
        self.debug_info.insert(pc, line.to_string());
        Ok(pc)
    }
}

fn split_asm_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut bracket_depth: i32 = 0;

    for ch in s.chars() {
        match ch {
            '[' => { bracket_depth += 1; current.push(ch); }
            ']' => { bracket_depth = (bracket_depth - 1).max(0); current.push(ch); }
            ',' if bracket_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() { args.push(trimmed.to_string()); }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() { args.push(trimmed.to_string()); }
    args
}

fn write_data_directive(vm: &mut VM, line: &str, data_ptr: &mut usize) -> Result<(), String> {
    if line.starts_with(".asciz") {
        let raw = line.split('"').nth(1).unwrap_or("");
        let s = unescape_asciz(raw);
        let bytes = s.as_bytes();
        if *data_ptr + bytes.len() + 1 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + bytes.len()].copy_from_slice(bytes);
        vm.memory[*data_ptr + bytes.len()] = 0;
        *data_ptr += bytes.len() + 1;
        Ok(())
    } else if line.starts_with(".ascii") {
        let raw = line.split('"').nth(1).unwrap_or("");
        let s = unescape_asciz(raw);
        let bytes = s.as_bytes();
        if *data_ptr + bytes.len() > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + bytes.len()].copy_from_slice(bytes);
        *data_ptr += bytes.len();
        Ok(())
    } else if line.starts_with(".quad") {
        let val_str = line.split_whitespace().nth(1).unwrap_or("0");
        let val = vm.parse_imm(val_str)?;
        if *data_ptr + 8 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + 8].copy_from_slice(&val.to_le_bytes());
        *data_ptr += 8;
        Ok(())
    } else if line.starts_with(".word") {
        let val_str = line.split_whitespace().nth(1).unwrap_or("0");
        let val = vm.parse_imm(val_str)? as i32;
        if *data_ptr + 4 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + 4].copy_from_slice(&val.to_le_bytes());
        *data_ptr += 4;
        Ok(())
    } else if line.starts_with(".byte") {
        let val_str = line.split_whitespace().nth(1).unwrap_or("0");
        let val = vm.parse_imm(val_str)? as i16;
        if !(0..=255).contains(&val) {
            return Err(format!(".byte out of range: {}", val));
        }
        if *data_ptr + 1 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr] = val as u8;
        *data_ptr += 1;
        Ok(())
    } else if line.starts_with(".space") {
        let size_str = line.split_whitespace().nth(1).unwrap_or("0");
        let size = vm.parse_imm(size_str)?.max(0) as usize;
        if *data_ptr + size > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        for b in &mut vm.memory[*data_ptr..*data_ptr + size] {
            *b = 0;
        }
        *data_ptr += size;
        Ok(())
    } else {
        Ok(())
    }
}

fn unescape_asciz(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' { out.push(ch); continue; }
        match chars.next() {
            Some('n') => out.push('\n'), Some('t') => out.push('\t'), Some('r') => out.push('\r'),
            Some('0') => out.push('\0'), Some('\"') => out.push('\"'), Some('\\') => out.push('\\'),
            Some(other) => { out.push('\\'); out.push(other); }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_asm_args_preserves_bracket_commas() {
        let args = split_asm_args("x29, x30, [sp, #-16]!");
        assert_eq!(args, vec!["x29", "x30", "[sp, #-16]!"]);
        let args = split_asm_args("x0, [x21, x22, lsl #3]");
        assert_eq!(args, vec!["x0", "[x21, x22, lsl #3]"]);
    }

    #[test]
    fn test_load_program_data_and_bytecode() {
        let asm = r#"
.data
greet: .asciz "hi"
.text
_main:
    mov x0, #42
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        let addr = *vm.labels.get("greet").unwrap();
        assert_eq!(&vm.memory[addr..addr + 3], b"hi\0");
        assert_eq!(vm.program.len(), 2);
        assert!(matches!(vm.program[0].opcode, OpCode::MOV));
        assert!(matches!(vm.program[0].operands[1], Operand::Imm(42)));
        assert!(matches!(vm.program[1].opcode, OpCode::RET));
    }

    #[test]
    fn test_run_simple_bytecode() {
        let asm = r#"
.text
_main:
    mov x0, #10
    mov x1, #32
    add x2, x0, x1
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        vm.run().unwrap();
        assert_eq!(vm.get_reg(2), 42);
    }

    #[test]
    fn test_mod_and_data_directives() {
        let asm = r#"
.data
b: .byte 7
w: .word 1024
pad: .space 3
q: .quad 99
.text
_main:
    mov x0, #43
    mov x1, #10
    mod x2, x0, x1
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        let b = *vm.labels.get("b").unwrap();
        let w = *vm.labels.get("w").unwrap();
        let pad = *vm.labels.get("pad").unwrap();
        let q = *vm.labels.get("q").unwrap();
        assert_eq!(vm.memory[b], 7);
        assert_eq!(
            i32::from_le_bytes(vm.memory[w..w + 4].try_into().unwrap()),
            1024
        );
        assert_eq!(&vm.memory[pad..pad + 3], &[0, 0, 0]);
        assert_eq!(
            i64::from_le_bytes(vm.memory[q..q + 8].try_into().unwrap()),
            99
        );

        vm.run().unwrap();
        assert_eq!(vm.get_reg(2), 3);
    }

    #[test]
    fn test_memory_out_of_bounds_is_error() {
        let asm = r#"
.text
_main:
    mov x0, #1
    mov x1, #2097152
    str x0, [x1]
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        let err = vm.run().unwrap_err();
        assert!(err.contains("Memory access out of bounds"));
    }

    #[test]
    fn test_vm_malloc_reuses_freed_block() {
        let mut vm = VM::new();
        let p1 = vm.vm_malloc(24).unwrap();
        let p2 = vm.vm_malloc(24).unwrap();
        assert!(p2 > p1);
        vm.vm_free(p1);
        let p3 = vm.vm_malloc(16).unwrap();
        assert_eq!(p3, p1);
    }

    #[test]
    fn test_ascii_directive_no_null_terminator() {
        let asm = r#"
.data
msg: .ascii "abc"
tail: .byte 1
.text
_main:
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        let msg = *vm.labels.get("msg").unwrap();
        let tail = *vm.labels.get("tail").unwrap();
        assert_eq!(&vm.memory[msg..msg + 3], b"abc");
        assert_eq!(tail - msg, 3);
        assert_eq!(vm.memory[tail], 1);
    }

    #[test]
    fn test_cset_condition_execution() {
        let asm = r#"
.text
_main:
    mov x0, #5
    mov x1, #7
    cmp x0, x1
    cset x2, lt
    cset x3, gt
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        vm.run().unwrap();
        assert_eq!(vm.get_reg(2), 1);
        assert_eq!(vm.get_reg(3), 0);
    }
}
