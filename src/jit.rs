const PAGE_SIZE: usize = 4096;

pub struct JitFunc {
    addr: *mut u8,
    size: usize,
}

struct JitMemory {
    addr: *mut u8,
    size: usize,
    /// current position for writing the next byte
    offset: usize,
}

impl JitMemory {
    /// Allocates read-write memory aligned on a 16 byte boundary.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn new(num_pages: usize) -> JitMemory {
        use std::mem;

        let size: usize = num_pages * PAGE_SIZE;
        let addr: *mut u8;

        unsafe {
            let mut raw_addr = mem::MaybeUninit::<*mut libc::c_void>::uninit();

            // Allocate aligned to page size
            libc::posix_memalign(raw_addr.as_mut_ptr(), PAGE_SIZE, size);

            // Make the memory readable and writable
            libc::mprotect(
                *raw_addr.as_mut_ptr(),
                size,
                libc::PROT_READ | libc::PROT_WRITE,
            );

            // Fill with 'ret' (0xc3)
            libc::memset(*raw_addr.as_mut_ptr(), 0xc3, size);

            // Transmute the c_void pointer to u8 pointer
            addr = mem::transmute(raw_addr);
        }

        JitMemory {
            addr,
            size,
            offset: 0,
        }
    }

    pub fn fill(&mut self, bytes: &[u8]) {
        unsafe {
            let addr = self.addr.add(self.offset) as *mut libc::c_void;
            libc::memcpy(addr, bytes.as_ptr() as *const libc::c_void, bytes.len());
        }
        self.offset += bytes.len();
    }

    pub fn into_func(self) -> JitFunc {
        unsafe {
            // Make the memory executable
            libc::mprotect(self.addr as *mut _, self.size, libc::PROT_READ | libc::PROT_EXEC);
        }
        JitFunc {
            addr: self.addr,
            size: self.size,
        }
    }
}

impl JitFunc {
    pub fn as_ptr(&self) -> *const u8 {
        self.addr
    }

    pub fn new(code: &[u8]) -> JitFunc {
        let num_pages = (code.len() + PAGE_SIZE - 1) / PAGE_SIZE;
        let mut mem = JitMemory::new(num_pages);
        mem.fill(code);
        mem.into_func()
    }
}

impl Drop for JitFunc {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.addr as *mut _, self.size);
        }
    }
}
