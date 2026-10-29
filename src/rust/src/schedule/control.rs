//! Process identity, namespace and job-control policy.
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
#[derive(Clone, Default)]
pub struct Credentials {
    pub uid: u32,
    pub euid: u32,
    pub saved_uid: u32,
    pub gid: u32,
    pub egid: u32,
    pub saved_gid: u32,
    pub groups: Vec<u32>,
}
#[derive(Clone)]
pub struct Control {
    pub credentials: Credentials,
    pub root: String,
    pub pgid: u32,
    pub sid: u32,
    pub execed: bool,
    pub auto_reap: bool,
    pub no_new_privs: bool,
}
impl Default for Control {
    fn default() -> Self {
        Self {
            credentials: Default::default(),
            root: "/".to_string(),
            pgid: 0,
            sid: 0,
            execed: false,
            auto_reap: false,
            no_new_privs: false,
        }
    }
}
impl Credentials {
    pub fn set_uid(&mut self, id: u32) -> Result<(), i32> {
        if self.euid == 0 {
            self.uid = id;
            self.euid = id;
            self.saved_uid = id;
            Ok(())
        } else if id == self.uid || id == self.saved_uid {
            self.euid = id;
            Ok(())
        } else {
            Err(1)
        }
    }
    pub fn set_gid(&mut self, id: u32) -> Result<(), i32> {
        if self.euid == 0 {
            self.gid = id;
            self.egid = id;
            self.saved_gid = id;
            Ok(())
        } else if id == self.gid || id == self.saved_gid {
            self.egid = id;
            Ok(())
        } else {
            Err(1)
        }
    }
    pub fn permission(&self, meta: &crate::fs::FileMetadata, bits: u16, real: bool) -> bool {
        let uid = if real { self.uid } else { self.euid };
        let gid = if real { self.gid } else { self.egid };
        let shift = if uid == meta.uid {
            6
        } else if gid == meta.gid || self.groups.contains(&meta.gid) {
            3
        } else {
            0
        };
        ((meta.mode >> shift) & bits) == bits
    }
}
