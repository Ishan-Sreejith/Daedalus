//! Phase 1: The Executable Memory Allocator
//!
//! This module provides `JitMemory`, a structure to allocate memory pages,
//! write machine code into them, and then make those pages executable.
//! It handles macOS-specific requirements like W^X protection and cache coherency.
#![allow(dead_code)]

#[cfg(all(feature = "libc", target_os = "macos"))]
use libc::MAP_JIT;
#[cfg(feature = "libc")]
use libc::{
    c_void, mmap, mprotect, munmap, size_t, MAP_ANON, MAP_FAILED, MAP_PRIVATE, PROT_EXEC,
    PROT_READ, PROT_WRITE,
};
use std::{io, ptr};

// WebAssembly constants (not used but needed for compilation)
#[cfg(not(feature = "libc"))]
const PROT_READ: i32 = 1;
#[cfg(not(feature = "libc"))]
const PROT_WRITE: i32 = 2;
#[cfg(not(feature = "libc"))]
const PROT_EXEC: i32 = 4;

// On macOS ARM64, sys_icache_invalidate is the function to flush instruction cache.
// It's not directly in libc, but available via a C function.
// We'll declare it as an external function.
#[cfg(all(feature = "libc", target_os = "macos"))]
extern "C" {
    fn sys_icache_invalidate(start: *mut c_void, size: size_t);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
extern "C" {
    fn pthread_jit_write_protect_np(enabled: i32);
}

// For WebAssembly builds, we need a different struct since we can't use raw pointers
#[cfg(not(feature = "libc"))]
pub struct JitMemory {
    data: Vec<u8>,
    requested_size: usize,
    page_size: usize,
}

// For native builds with libc
#[cfg(feature = "libc")]
pub struct JitMemory {
    ptr: *mut u8,
    size: usize,
    requested_size: usize,
    /// The actual page size of the system.
    /// Used for mmap alignment and mprotect granularity.
    page_size: usize,
}

impl JitMemory {
    /// Allocates a new block of memory suitable for JIT code.
    /// The memory is initially writable but not executable (W^X).
    ///
    /// # Arguments
    /// * `size_in_bytes` - The desired size of the memory block.
    ///
    /// # Returns
    /// A `Result` containing `JitMemory` on success, or an `io::Error` on failure.
    #[cfg(feature = "libc")]
    pub fn new(size_in_bytes: usize) -> io::Result<Self> {
        let page_size = Self::get_page_size();
        // Round up size to the nearest page size
        let aligned_size = (size_in_bytes + page_size - 1) & !(page_size - 1);

        let mut flags = MAP_PRIVATE | MAP_ANON;
        #[cfg(all(feature = "libc", target_os = "macos"))]
        {
            // MAP_JIT is required on macOS to enable JIT write/execute transitions.
            flags |= MAP_JIT;
        }

        let ptr = unsafe {
            mmap(
                ptr::null_mut(), // Let the system choose the address
                aligned_size,
                PROT_READ | PROT_WRITE, // Initially readable and writable
                flags,
                -1, // No file descriptor
                0,  // No offset
            )
        };

        if ptr == MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(JitMemory {
            ptr: ptr as *mut u8,
            size: aligned_size,
            requested_size: size_in_bytes,
            page_size,
        })
    }

    #[cfg(not(feature = "libc"))]
    pub fn new(size_in_bytes: usize) -> io::Result<Self> {
        // For WebAssembly, we just allocate a regular Vec
        let page_size = Self::get_page_size();
        let aligned_size = (size_in_bytes + page_size - 1) & !(page_size - 1);

        let data = vec![0u8; aligned_size];

        Ok(JitMemory {
            data,
            requested_size: size_in_bytes,
            page_size,
        })
    }

    /// Returns the system's memory page size.
    /// This is crucial for `mmap` and `mprotect` calls.
    #[cfg(feature = "libc")]
    fn get_page_size() -> usize {
        let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if size <= 0 {
            4096
        } else {
            size as usize
        }
    }

    #[cfg(not(feature = "libc"))]
    fn get_page_size() -> usize {
        4096 // Default page size for WebAssembly
    }

    #[cfg(feature = "libc")]
    fn set_protection(&self, prot: i32) -> io::Result<()> {
        let result = unsafe { mprotect(self.ptr as *mut c_void, self.size, prot) };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    #[cfg(not(feature = "libc"))]
    fn set_protection(&self, _prot: i32) -> io::Result<()> {
        // WebAssembly doesn't support memory protection
        Ok(())
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn set_write_protect(enabled: bool) {
        let value = if enabled { 1 } else { 0 };
        unsafe {
            pthread_jit_write_protect_np(value);
        }
    }

    /// Enables writing to JIT memory in a W^X-safe way.
    #[cfg(feature = "libc")]
    fn begin_write(&self) -> io::Result<()> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::set_write_protect(false);
        }
        self.set_protection(PROT_READ | PROT_WRITE)?;
        Ok(())
    }

    /// Enables writing to JIT memory in a W^X-safe way.
    #[cfg(not(feature = "libc"))]
    fn begin_write(&self) -> io::Result<()> {
        // WebAssembly doesn't need write protection
        Ok(())
    }

    /// Ends a write phase for JIT memory.
    #[cfg(feature = "libc")]
    fn end_write(&self) -> io::Result<()> {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::set_write_protect(true);
        }
        Ok(())
    }

    /// Ends a write phase for JIT memory.
    #[cfg(not(feature = "libc"))]
    fn end_write(&self) -> io::Result<()> {
        // WebAssembly doesn't need write protection
        Ok(())
    }

    /// Writes a slice of bytes into the allocated memory.
    ///
    /// # Arguments
    /// * `offset` - The offset within the allocated memory to start writing.
    /// * `code` - The slice of bytes (machine code) to write.
    ///
    /// # Returns
    /// An `io::Result` indicating success or failure.
    ///
    /// # Panics
    /// Panics if the write would go out of bounds.
    #[cfg(feature = "libc")]
    pub fn write_code(&mut self, offset: usize, code: &[u8]) -> io::Result<()> {
        if offset + code.len() > self.requested_size {
            panic!("Attempted to write code out of bounds of JitMemory buffer.");
        }

        self.begin_write()?;
        unsafe {
            ptr::copy_nonoverlapping(code.as_ptr(), self.ptr.add(offset), code.len());
        }
        self.end_write()?;

        Ok(())
    }

    /// Writes a slice of bytes into the allocated memory.
    /// WebAssembly version that uses the Vec directly.
    #[cfg(not(feature = "libc"))]
    pub fn write_code(&mut self, offset: usize, code: &[u8]) -> io::Result<()> {
        if offset + code.len() > self.requested_size {
            panic!("Attempted to write code out of bounds of JitMemory buffer.");
        }

        // Ensure the Vec is large enough
        let needed_size = offset + code.len();
        if needed_size > self.data.len() {
            self.data.resize(needed_size, 0);
        }

        self.data[offset..offset + code.len()].copy_from_slice(code);
        Ok(())
    }

    /// Changes the memory protection to Read-Execute (RX).
    /// This makes the code executable but no longer writable (W^X).
    /// It also flushes the instruction cache on ARM64 macOS.
    ///
    /// # Returns
    /// An `io::Result` indicating success or failure.
    #[cfg(feature = "libc")]
    pub fn make_executable(&mut self) -> io::Result<()> {
        self.set_protection(PROT_READ | PROT_EXEC)?;

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::set_write_protect(true);
        }

        // On ARM64, after writing new code and changing permissions,
        // the instruction cache must be flushed to ensure the CPU sees the new code.
        #[cfg(all(feature = "libc", target_os = "macos"))]
        unsafe {
            sys_icache_invalidate(self.ptr as *mut c_void, self.size);
        }

        Ok(())
    }

    /// Changes the memory protection to Read-Execute (RX).
    /// For WebAssembly, this is a no-op since we can't execute JIT code anyway.
    #[cfg(not(feature = "libc"))]
    pub fn make_executable(&mut self) -> io::Result<()> {
        // WebAssembly doesn't support executable memory
        Ok(())
    }

    /// Returns a raw pointer to the allocated memory.
    /// This pointer can be transmuted to a function pointer for execution.
    #[cfg(feature = "libc")]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Returns a raw pointer to the allocated memory.
    /// For WebAssembly, this returns a pointer to the Vec's data.
    #[cfg(not(feature = "libc"))]
    pub fn as_ptr(&self) -> *const u8 {
        self.data.as_ptr()
    }

    /// Returns the size of the allocated memory block.
    #[cfg(feature = "libc")]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the size of the allocated memory block.
    #[cfg(not(feature = "libc"))]
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl Drop for JitMemory {
    /// Frees the allocated memory when `JitMemory` goes out of scope.
    fn drop(&mut self) {
        #[cfg(feature = "libc")]
        unsafe {
            munmap(self.ptr as *mut c_void, self.size);
        }
        // For WebAssembly, the memory will be cleaned up by the garbage collector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_memory_allocation() {
        let size = 4096; // One page
        let jit_mem = JitMemory::new(size).unwrap();
        assert!(jit_mem.as_ptr() as usize != 0);
        assert!(jit_mem.size() >= size); // Should be page-aligned or exact
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn test_jit_memory_write_and_execute() {
        // This test requires executing generated code, which is unsafe.
        // We'll use a simple ARM64 instruction sequence:
        // mov x0, #42  (0xD2800540)
        // ret          (0xD65F03C0)
        // This function will return 42.

        let code_bytes: [u8; 8] = [
            0x40, 0x05, 0x80, 0xD2, // mov x0, #42
            0xC0, 0x03, 0x5F, 0xD6, // ret
        ];

        let mut jit_mem = JitMemory::new(code_bytes.len()).unwrap();
        jit_mem.write_code(0, &code_bytes).unwrap();
        jit_mem.make_executable().unwrap();

        // Transmute the memory pointer to a function pointer
        let func: extern "C" fn() -> i64 = unsafe { std::mem::transmute(jit_mem.as_ptr()) };

        // Call the generated function
        let result = func();
        assert_eq!(result, 42);
    }

    #[test]
    #[should_panic(expected = "Attempted to write code out of bounds")]
    fn test_jit_memory_write_out_of_bounds() {
        let mut jit_mem = JitMemory::new(16).unwrap();
        let large_code = vec![0x90; 32]; // 32 bytes
        jit_mem.write_code(0, &large_code).unwrap();
    }
}
