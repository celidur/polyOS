//! System V semaphore ownership and atomic operation transactions.
use super::{task::WaitReason, task_manager::TaskManager};
use alloc::{collections::BTreeMap, vec::Vec};
use lazy_static::lazy_static;
use spin::Mutex;
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Operation {
    pub number: u16,
    pub delta: i16,
    pub flags: u16,
}
struct Set {
    key: i32,
    uid: u32,
    mode: u16,
    values: Vec<u16>,
}
#[derive(Default)]
pub struct SemaphoreSets {
    next: u32,
    sets: BTreeMap<u32, Set>,
}
lazy_static! {
    pub static ref SEMAPHORES: Mutex<SemaphoreSets> = Mutex::new(SemaphoreSets::default());
}
impl SemaphoreSets {
    pub fn get(&mut self, key: i32, count: u32, flags: u32, uid: u32) -> Result<u32, i32> {
        if flags & !(0o777 | 0o1000 | 0o2000) != 0 || count > 64 {
            return Err(22);
        }
        if key != 0 {
            if let Some((&id, set)) = self.sets.iter().find(|(_, s)| s.key == key) {
                if flags & 0o3000 == 0o3000 {
                    return Err(17);
                }
                if count as usize > set.values.len() {
                    return Err(22);
                }
                if uid != 0 && uid != set.uid && set.mode & 0o004 == 0 {
                    return Err(13);
                }
                return Ok(id);
            }
            if flags & 0o1000 == 0 {
                return Err(2);
            }
        }
        if count == 0 {
            return Err(22);
        }
        if self.sets.len() >= 128 {
            return Err(28);
        }
        self.next = self.next.checked_add(1).ok_or(28)?;
        let id = self.next;
        self.sets.insert(
            id,
            Set {
                key,
                uid,
                mode: flags as u16 & 0o777,
                values: vec![0; count as usize],
            },
        );
        Ok(id)
    }
    fn set(&mut self, id: u32, uid: u32, alter: bool) -> Result<&mut Set, i32> {
        let set = self
            .sets
            .get_mut(&id)
            .ok_or(if id > 0 && id <= self.next { 43 } else { 22 })?;
        let permission = if alter { 2 } else { 4 };
        let shift = if uid == set.uid { 6 } else { 0 };
        if uid != 0 && (set.mode >> shift) & permission == 0 {
            return Err(13);
        }
        Ok(set)
    }
    pub fn values(&mut self, id: u32, uid: u32, alter: bool) -> Result<Vec<u16>, i32> {
        Ok(self.set(id, uid, alter)?.values.clone())
    }
    pub fn replace(
        &mut self,
        id: u32,
        uid: u32,
        values: &[u16],
        tm: &mut TaskManager,
    ) -> Result<(), i32> {
        let set = self.set(id, uid, true)?;
        if values.len() != set.values.len() {
            return Err(22);
        }
        if values.iter().any(|v| *v > 32767) {
            return Err(34);
        }
        set.values.copy_from_slice(values);
        tm.wake_preserving(WaitReason::Semaphore(id as usize));
        Ok(())
    }
    pub fn remove(&mut self, id: u32, uid: u32, tm: &mut TaskManager) -> Result<(), i32> {
        let set = self.sets.get(&id).ok_or(22)?;
        if uid != 0 && uid != set.uid {
            return Err(1);
        }
        self.sets.remove(&id);
        tm.wake_preserving(WaitReason::Semaphore(id as usize));
        Ok(())
    }
    pub fn operate(
        &mut self,
        id: u32,
        uid: u32,
        ops: &[Operation],
        tm: &mut TaskManager,
    ) -> Result<bool, i32> {
        let set = self.set(id, uid, ops.iter().any(|o| o.delta != 0))?;
        let mut next = set.values.clone();
        // Validate every operation even if an earlier operation would block.
        for op in ops {
            if op.number as usize >= next.len() || op.flags & !0o14000 != 0 {
                return Err(22);
            }
            if op.flags & 0o10000 != 0 {
                return Err(95);
            }
        }
        for op in ops {
            let value = next[op.number as usize] as i32;
            let updated = value + op.delta as i32;
            if updated < 0 || (op.delta == 0 && value != 0) {
                if op.flags & 0o4000 != 0 {
                    return Err(11);
                }
                tm.block_current_and_restart_syscall(WaitReason::Semaphore(id as usize))
                    .map_err(|_| 3)?;
                return Ok(false);
            }
            if updated > 32767 {
                return Err(34);
            }
            next[op.number as usize] = updated as u16;
        }
        set.values = next;
        tm.wake_preserving(WaitReason::Semaphore(id as usize));
        Ok(true)
    }
}
