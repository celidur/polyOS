#![allow(dead_code)]
use alloc::vec::Vec;

use crate::{constant::PROGRAM_VIRTUAL_ADDRESS, kernel::KERNEL, memory::Page};

pub const PF_X: u32 = 0x1;
pub const PF_W: u32 = 0x2;
pub const PF_R: u32 = 0x4;

pub const PT_NULL: u8 = 0x0;
pub const PT_LOAD: u8 = 0x1;
pub const PT_DYNAMIC: u8 = 0x2;
pub const PT_INTERP: u8 = 0x3;
pub const PT_NOTE: u8 = 0x4;
pub const PT_SHLIB: u8 = 0x5;
pub const PT_PHDR: u8 = 0x6;
pub const SHT_NULL: u8 = 0x0;
pub const SHT_PROGBITS: u8 = 0x1;
pub const SHT_SYMTAB: u8 = 0x2;
pub const SHT_STRTAB: u8 = 0x3;
pub const SHT_RELA: u8 = 0x4;
pub const SHT_HASH: u8 = 0x5;
pub const SHT_DYNAMIC: u8 = 0x6;
pub const SHT_NOTE: u8 = 0x7;
pub const SHT_NOBITS: u8 = 0x8;
pub const SHT_REL: u8 = 0x9;
pub const SHT_SHLIB: u8 = 0xA;
pub const SHT_DYNSYM: u8 = 0xB;
pub const SHT_LOPROC: u8 = 0xC;
pub const SHT_HIPROC: u8 = 0xD;
pub const SHT_LOUSER: u8 = 0xE;
pub const SHT_HIUSER: u8 = 0xF;
pub const ET_NONE: u8 = 0x0;
pub const ET_REL: u8 = 0x1;
pub const ET_EXEC: u8 = 0x2;
pub const ET_DYN: u8 = 0x3;
pub const ET_CORE: u8 = 0x4;
pub const EI_NIDENT: u8 = 16;
pub const EI_CLASS: u8 = 4;
pub const EI_DATA: u8 = 5;
pub const ELFCLASSNONE: u8 = 0x0;
pub const ELFCLASS32: u8 = 0x1;
pub const ELFCLASS64: u8 = 0x2;
pub const ELFDATANONE: u8 = 0x0;
pub const ELFDATA2LSB: u8 = 0x1;
pub const ELFDATA2MSB: u8 = 0x2;
pub const SHN_UNDEF: u8 = 0x0;

#[derive(Debug)]
pub enum ElfError {
    Io,
    InvalidFormat,
    Unknown,
}

const ELF_SIGNATURE: [u8; 4] = [0x7F, b'E', b'L', b'F'];

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf32Phdr {
    p_type: u32,
    p_offset: u32,
    pub p_vaddr: u32,
    pub p_paddr: u32,
    p_filesz: u32,
    pub p_memsz: u32,
    pub p_flags: u32,
    p_align: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Elf32Shdr {
    sh_name: u32,
    sh_type: u32,
    sh_flags: u32,
    sh_addr: u32,
    sh_offset: u32,
    sh_size: u32,
    sh_link: u32,
    sh_info: u32,
    sh_addralign: u32,
    sh_entsize: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    pub e_entry: u32,
    e_phoff: u32,
    e_shoff: u32,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

impl ElfHeader {
    fn is_valid(&self) -> bool {
        self.e_ident[0..4] == ELF_SIGNATURE
            && matches!(self.e_ident[4], 0 | 1)
            && matches!(self.e_ident[5], 0 | 1)
            && self.e_type == 2
            && self.e_entry >= PROGRAM_VIRTUAL_ADDRESS as u32
            && self.e_phoff != 0
    }

    pub fn validate(&self) -> Result<(), ElfError> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(ElfError::InvalidFormat)
        }
    }

    pub fn program_headers(&self) -> &[Elf32Phdr] {
        let base = (self as *const _ as usize + self.e_phoff as usize) as *const Elf32Phdr;
        unsafe { core::slice::from_raw_parts(base, self.e_phnum as usize) }
    }

    pub fn str_table(&self) -> &[u8] {
        let base = (self as *const _ as usize + self.e_shoff as usize) as *const Elf32Shdr;
        // SAFETY: e_shstrndx must be valid.
        let shdr = unsafe { &*base.add(self.e_shstrndx as usize) };
        let str_ptr = (self as *const _ as usize + shdr.sh_offset as usize) as *const u8;
        unsafe { core::slice::from_raw_parts(str_ptr, shdr.sh_size as usize) }
    }
}

#[derive(Debug, Clone)]
pub struct ElfSegment {
    virtual_address: u32,
    flags: u32,
    page_offset: usize,
    file_size: usize,
    memory_size: usize,
    memory: Page<u8>,
}

impl ElfSegment {
    pub fn virtual_address(&self) -> u32 {
        self.virtual_address
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn page_offset(&self) -> usize {
        self.page_offset
    }

    pub fn file_size(&self) -> usize {
        self.file_size
    }

    pub fn memory_size(&self) -> usize {
        self.memory_size
    }

    pub fn memory(&self) -> &Page<u8> {
        &self.memory
    }
}

#[derive(Debug, Clone)]
pub struct ElfFile {
    memory: Page<u8>,
    segments: Vec<ElfSegment>,
}

unsafe impl Send for ElfFile {}
unsafe impl Sync for ElfFile {}

impl ElfFile {
    pub fn load(filename: &str) -> Result<Self, ElfError> {
        let mut file = KERNEL.vfs.read().open(filename).map_err(|_| ElfError::Io)?;
        let stat = file.ops.stat().map_err(|_| ElfError::Io)?;

        let memory = Page::new(stat.size as usize).ok_or(ElfError::Io)?;

        file.ops
            .read(memory.as_mut_slice())
            .map_err(|_| ElfError::Io)?;

        let header = unsafe { &*(memory.as_ptr() as *const ElfHeader) };
        header.validate()?;

        let mut elf = Self {
            memory,
            segments: Vec::new(),
        };

        elf.load_segments(header)?;
        Ok(elf)
    }

    fn load_segments(&mut self, header: &ElfHeader) -> Result<(), ElfError> {
        for phdr in header.program_headers() {
            if phdr.p_type == 1 {
                self.process_load_segment(phdr);
            }
        }
        Ok(())
    }

    fn process_load_segment(&mut self, phdr: &Elf32Phdr) {
        let virtual_address = phdr.p_vaddr & 0xFFFFF000;
        let page_offset = (phdr.p_vaddr - virtual_address) as usize;
        let size = page_offset + phdr.p_memsz as usize;
        let Some(segment_memory) = Page::<u8>::new(size) else {
            return;
        };

        let source = unsafe {
            core::slice::from_raw_parts(
                self.memory.as_ptr().add(phdr.p_offset as usize),
                phdr.p_filesz as usize,
            )
        };
        let destination =
            &mut segment_memory.as_mut_slice()[page_offset..page_offset + phdr.p_filesz as usize];
        destination.copy_from_slice(source);

        self.segments.push(ElfSegment {
            virtual_address,
            flags: phdr.p_flags,
            page_offset,
            file_size: phdr.p_filesz as usize,
            memory_size: phdr.p_memsz as usize,
            memory: segment_memory,
        });
    }

    pub fn header(&self) -> &ElfHeader {
        unsafe { &*(self.memory.as_ptr() as *const ElfHeader) }
    }

    pub fn segments(&self) -> &[ElfSegment] {
        &self.segments
    }
}
