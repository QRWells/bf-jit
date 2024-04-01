use std::{
    collections::HashMap,
    io::{Read, Write},
    path::Path,
    ptr,
};

use iced_x86::code_asm::{byte_ptr, r12, r13, r14, r15, rax, rcx, rdi, rdx, rsi, CodeAssembler};

use crate::{
    error::RuntimeError,
    ir::{compile, BrainfuckIR},
    jit::JitFunc,
};

pub struct VM<'io> {
    code: Vec<BrainfuckIR>,
    memory: Box<[u8]>,
    input: Box<dyn Read + 'io>,
    output: Box<dyn Write + 'io>,
}

const MEMORY_SIZE: usize = 4 * 1024 * 1024; // 4 MiB

impl<'io> VM<'io> {
    pub fn new(
        file_path: &Path,
        input: Box<dyn Read + 'io>,
        output: Box<dyn Write + 'io>,
    ) -> anyhow::Result<Self> {
        let src = std::fs::read_to_string(file_path)?;
        let code = compile(&src)?;

        let memory = vec![0; MEMORY_SIZE].into_boxed_slice();

        Ok(Self {
            code,
            memory,
            input,
            output,
        })
    }

    pub fn run(&mut self) -> anyhow::Result<()> {
        let map = self.mark_left_right();
        let mut ptr = 0;
        let mut pc = 0;
        loop {
            // dbg!(&self.code[pc]);
            match self.code[pc] {
                BrainfuckIR::AddVal(val) => self.memory[ptr] = self.memory[ptr].wrapping_add(val),
                BrainfuckIR::SubVal(val) => self.memory[ptr] = self.memory[ptr].wrapping_sub(val),
                BrainfuckIR::AddPtr(val) => {
                    if ptr == self.memory.len() {
                        return Err(RuntimeError::Overflow.into());
                    }
                    ptr += val as usize;
                }
                BrainfuckIR::SubPtr(val) => {
                    if ptr == 0 {
                        return Err(RuntimeError::Overflow.into());
                    }
                    ptr -= val as usize;
                }
                BrainfuckIR::PutByte => {
                    self.output.write_all(&self.memory[ptr..=ptr])?;
                }
                BrainfuckIR::GetByte => {
                    let mut byte: [u8; 1] = [0; 1];
                    self.input.read_exact(&mut byte)?;
                }
                BrainfuckIR::Jz => {
                    if self.memory[ptr] == 0 {
                        pc = map[&pc];
                        continue;
                    }
                }
                BrainfuckIR::Jnz => {
                    if self.memory[ptr] != 0 {
                        pc = map[&pc];
                        continue;
                    }
                }
            }

            pc += 1;

            if pc == self.code.len() {
                break;
            }
        }
        Ok(())
    }

    fn mark_left_right(&self) -> HashMap<usize, usize> {
        let mut i = 0;
        let mut stack = vec![];
        let mut map = HashMap::new();
        while i < self.code.len() {
            match self.code[i] {
                BrainfuckIR::Jz => {
                    stack.push(i);
                }
                BrainfuckIR::Jnz => {
                    if let Some(left) = stack.pop() {
                        map.insert(left, i);
                        map.insert(i, left);
                    } else {
                        unreachable!()
                    }
                }
                _ => {}
            }
            i += 1;
        }
        map
    }
}

impl<'io> VM<'io> {
    pub fn run_jit(&mut self) -> anyhow::Result<()> {
        let mut asm = CodeAssembler::new(64)?;
        let mut stack = Vec::new();
        let mut overflow = asm.create_label();
        let mut exit = asm.create_label();

        let put_byte = VM::put_byte as *const () as u64;
        let get_byte = VM::get_byte as *const () as u64;
        let overflow_error = VM::overflow_error as *const () as u64;

        asm.push(rax)?;
        asm.mov(r12, rdi)?; // *this
        asm.mov(r13, rsi)?; // mem_start
        asm.mov(r14, rdx)?; // mem_end
        asm.mov(rcx, rsi)?; // ptr

        for instr in &self.code {
            match instr {
                BrainfuckIR::AddVal(x) => {
                    asm.add(byte_ptr(rcx), *x as i32)?;
                }
                BrainfuckIR::SubVal(x) => {
                    asm.sub(byte_ptr(rcx), *x as i32)?;
                }
                BrainfuckIR::AddPtr(x) => {
                    asm.add(rcx, *x as i32)?;
                    asm.jc(overflow)?;
                    asm.cmp(rcx, r14)?;
                    asm.jnb(overflow)?;
                }
                BrainfuckIR::SubPtr(x) => {
                    asm.sub(rcx, *x as i32)?;
                    asm.jc(overflow)?;
                    asm.cmp(rcx, r13)?;
                    asm.jb(overflow)?;
                }
                BrainfuckIR::PutByte => {
                    asm.mov(r15, rcx)?;
                    asm.mov(rdi, r12)?;
                    asm.mov(rsi, rcx)?;
                    asm.mov(rax, put_byte)?;
                    asm.call(rax)?;
                    asm.test(rax, rax)?;
                    asm.jnz(exit)?;
                    asm.mov(rcx, r15)?;
                }
                BrainfuckIR::GetByte => {
                    asm.mov(r15, rcx)?;
                    asm.mov(rdi, r12)?;
                    asm.mov(rsi, rcx)?;
                    asm.mov(rax, get_byte)?;
                    asm.call(rax)?;
                    asm.test(rax, rax)?;
                    asm.jnz(exit)?;
                    asm.mov(rcx, r15)?;
                }
                BrainfuckIR::Jz => {
                    let mut left = asm.create_label();
                    let right = asm.create_label();
                    stack.push((left, right));
                    asm.cmp(byte_ptr(rcx), 0)?;
                    asm.jz(right)?;
                    asm.set_label(&mut left)?;
                }
                BrainfuckIR::Jnz => {
                    let (left, mut right) = stack.pop().unwrap();
                    asm.cmp(byte_ptr(rcx), 0)?;
                    asm.jnz(left)?;
                    asm.set_label(&mut right)?;
                }
            }
        }

        asm.xor(rax, rax)?;
        asm.jmp(exit)?;

        asm.set_label(&mut overflow)?;
        asm.mov(rax, overflow_error)?;
        asm.call(rax)?;

        asm.set_label(&mut exit)?;
        asm.pop(rdx)?;
        asm.ret()?;

        let code = asm.assemble(0x1000_0000)?;

        type RawFn = unsafe extern "sysv64" fn(
            this: *mut VM<'_>,
            memory_start: *mut u8,
            memory_end: *const u8,
        ) -> *mut RuntimeError;

        let exec = JitFunc::new(&code);

        drop(code); // early drop to avoid strange pointer error

        let func: RawFn = unsafe { std::mem::transmute(exec.as_ptr()) };

        let this: *mut Self = self;
        let memory_start = self.memory.as_mut_ptr();
        let memory_end = unsafe { memory_start.add(MEMORY_SIZE) };

        let ret = unsafe { func(this, memory_start, memory_end) };

        if ret.is_null() {
            Ok(())
        } else {
            Err((*unsafe { Box::from_raw(ret) }).into())
        }
    }

    unsafe extern "sysv64" fn get_byte(this: *mut Self, ptr: *mut u8) -> *mut RuntimeError {
        let mut buf = [0_u8];
        let this = &mut *this;
        match this.input.read(&mut buf) {
            Ok(0) => {}
            Ok(1) => *ptr = buf[0],
            Err(e) => return vm_error(RuntimeError::IO(e)),
            _ => unreachable!(),
        }
        ptr::null_mut()
    }

    unsafe extern "sysv64" fn put_byte(this: *mut Self, ptr: *const u8) -> *mut RuntimeError {
        let buf = std::slice::from_ref(&*ptr);
        let this = &mut *this;
        match this.output.write_all(buf) {
            Ok(()) => ptr::null_mut(),
            Err(e) => vm_error(RuntimeError::IO(e)),
        }
    }

    unsafe extern "sysv64" fn overflow_error() -> *mut RuntimeError {
        vm_error(RuntimeError::Overflow)
    }
}

#[inline(always)]
fn vm_error(re: RuntimeError) -> *mut RuntimeError {
    let e = Box::new(re);
    Box::into_raw(e)
}
