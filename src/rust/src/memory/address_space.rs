//! Owned mmap pages, permissions and address selection, independent of syscall ABI.
use super::{COW, PRESENT, Page, PageDirectory, USER_ACCESS, WRITABLE};
use crate::{fs::descriptor::ProcessDescriptor, schedule::process::Process};
use alloc::{collections::BTreeMap, vec::Vec};
const START: u32 = 0x10000000;
const END: u32 = 0x70000000;
const PAGE: u32 = 4096;
#[derive(Clone)]
struct Mapping {
    _page: Page<u8>,
    prot: u32,
    offset: u32,
    file: bool,
}
#[derive(Clone, Default)]
pub struct AddressSpace {
    pages: BTreeMap<u32, Mapping>,
}
#[derive(Clone, Copy)]
pub enum VmError {
    Invalid,
    NoMemory,
    Fault,
    Unsupported,
    BadFd,
    Io,
    Access,
}
fn length(len: u32) -> Result<u32, VmError> {
    if len == 0 {
        return Err(VmError::Invalid);
    }
    len.checked_add(PAGE - 1)
        .map(|n| n & !(PAGE - 1))
        .ok_or(VmError::Invalid)
}
fn range(addr: u32, len: u32) -> Result<u32, VmError> {
    let end = addr.checked_add(length(len)?).ok_or(VmError::Invalid)?;
    if addr & (PAGE - 1) != 0 || addr < START || end > END {
        return Err(VmError::Invalid);
    }
    Ok(end)
}
fn flags(prot: u32, cow: bool) -> u32 {
    if prot == 0 {
        0
    } else {
        PRESENT
            | USER_ACCESS
            | if prot & 2 != 0 {
                if cow { COW } else { WRITABLE }
            } else {
                0
            }
    }
}
impl AddressSpace {
    pub fn regions(&self) -> Vec<(u32, u32, u32, u32, &'static str)> {
        let mut regions: Vec<(u32, u32, u32, u32, &'static str)> = Vec::new();
        for (&address, mapping) in &self.pages {
            let label = if mapping.file { "[file-snapshot]" } else { "[anon]" };
            if let Some(last) = regions.last_mut() {
                if last.1 == address && last.2 == mapping.prot && last.4 == label
                    && (!mapping.file || last.3 + (last.1 - last.0) == mapping.offset) {
                    last.1 += PAGE; continue;
                }
            }
            regions.push((address, address + PAGE, mapping.prot, mapping.offset, label));
        }
        regions
    }
    fn free(&self, addr: u32, len: u32) -> bool {
        range(addr, len).is_ok_and(|end| self.pages.range(addr..end).next().is_none())
    }
    pub fn map(
        &mut self,
        process: &Process,
        hint: u32,
        len: u32,
        prot: u32,
        options: u32,
        file: Option<ProcessDescriptor>,
        offset: u32,
    ) -> Result<u32, VmError> {
        if prot & !7 != 0 || options & !0x32 != 0 || options & 2 == 0 || offset & (PAGE - 1) != 0 {
            return Err(VmError::Invalid);
        }
        let len = length(len)?;
        if len > 32 * 1024 * 1024 {
            return Err(VmError::NoMemory);
        }
        let addr = if options & 0x10 != 0 {
            range(hint, len)?;
            hint
        } else if hint != 0 && self.free(hint & !(PAGE - 1), len) {
            hint & !(PAGE - 1)
        } else {
            let mut candidate = START;
            for &address in self.pages.keys() {
                if address >= candidate.saturating_add(len) {
                    break;
                }
                candidate = address + PAGE;
            }
            if !self.free(candidate, len) {
                return Err(VmError::NoMemory);
            }
            candidate
        };
        let mut pending = Vec::new();
        pending
            .try_reserve((len / PAGE) as usize)
            .map_err(|_| VmError::NoMemory)?;
        for _ in 0..len / PAGE {
            pending.push(Page::new(PAGE as usize).ok_or(VmError::NoMemory)?);
        }
        let file_backed = file.is_some();
        if let Some(descriptor) = file {
            let ProcessDescriptor::File(handle) = descriptor else {
                return Err(VmError::BadFd);
            };
            let mut handle = handle.lock();
            let size = handle.ops.stat().map_err(|_| VmError::Io)?.size;
            if offset as u64 >= size || len as u64 > (size - offset as u64 + 4095) & !4095 {
                return Err(VmError::Invalid);
            }
            let old = handle.ops.position().map_err(|_| VmError::Unsupported)?;
            let result = (|| {
                handle.ops.seek(offset as usize).map_err(|_| VmError::Io)?;
                for page in &pending {
                    let mut done = 0;
                    while done < 4096 {
                        let n = handle
                            .ops
                            .read(&mut page.as_mut_slice()[done..])
                            .map_err(|_| VmError::Io)?;
                        if n == 0 {
                            break;
                        }
                        done += n;
                    }
                }
                Ok(())
            })();
            let restored = handle.ops.seek(old).map_err(|_| VmError::Io);
            result?;
            restored?;
        }
        self.unmap(process, addr, len)?;
        for (index, page) in pending.into_iter().enumerate() {
            let address = addr + index as u32 * PAGE;
            process
                .page_directory
                .map(address, page.as_ptr() as u32, flags(prot, false))
                .map_err(|_| VmError::Fault)?;
            self.pages.insert(address, Mapping { _page: page, prot, file: file_backed,
                offset: if file_backed { offset + index as u32 * PAGE } else { 0 } });
        }
        Ok(addr)
    }
    pub fn unmap(&mut self, process: &Process, addr: u32, len: u32) -> Result<(), VmError> {
        let end = range(addr, len)?;
        for address in (addr..end).step_by(PAGE as usize) {
            if self.pages.contains_key(&address) {
                process
                    .page_directory
                    .set(address, 0)
                    .map_err(|_| VmError::Fault)?;
                self.pages.remove(&address);
            }
        }
        process.remove_cow_pages_in_range(addr, end - addr);
        Ok(())
    }
    pub fn protect(
        &mut self,
        directory: &PageDirectory,
        addr: u32,
        len: u32,
        prot: u32,
    ) -> Result<(), VmError> {
        if prot & !7 != 0 {
            return Err(VmError::Invalid);
        }
        if len == 0 {
            return if addr & 4095 == 0 {
                Ok(())
            } else {
                Err(VmError::Invalid)
            };
        }
        let end = range(addr, len)?;
        // Validate the complete range before changing any page.
        if (addr..end)
            .step_by(PAGE as usize)
            .any(|a| !self.pages.contains_key(&a))
        {
            return Err(VmError::NoMemory);
        }
        for address in (addr..end).step_by(PAGE as usize) {
            let physical = directory.get(address).map_err(|_| VmError::Fault)? & !4095;
            directory
                .set(address, physical | flags(prot, true))
                .map_err(|_| VmError::Fault)?;
            self.pages.get_mut(&address).unwrap().prot = prot;
        }
        Ok(())
    }
}
