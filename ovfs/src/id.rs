use rustix::process;
use rustix::thread::{Gid, RawGid, RawUid, Uid};

#[derive(Debug, Clone, Copy)]
pub struct Id {
    pub(crate) uid: Uid,
    pub(crate) gid: Gid,
}

impl Id {
    pub fn me() -> Self {
        Self {
            uid: process::geteuid(),
            gid: process::getegid(),
        }
    }
    pub fn root() -> Self {
        Self {
            uid: Uid::ROOT,
            gid: Gid::ROOT,
        }
    }

    pub fn raw_uid(&self) -> RawUid {
        self.uid.as_raw()
    }
    pub fn raw_gid(&self) -> RawGid {
        self.gid.as_raw()
    }
}
