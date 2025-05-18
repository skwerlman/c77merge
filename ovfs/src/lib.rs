use std::{fs::File, io::Write};

use ovfs::OVFSTree;
use rustix::thread::{self, UnshareFlags};

pub use error::OVFSError;
use id::Id;
use tracing::{instrument, trace};

mod error;
mod id;
pub mod ovfs;

type Result<T> = std::result::Result<T, OVFSError>;

// sets up a user namespace and returns who we used to be
#[instrument(level = "trace")]
pub fn unshare(id: Id, with_mounts: bool) -> Result<Id> {
    let me = Id::me();

    // create a new namespace allowing user mapping and maybe mounts
    let flags = UnshareFlags::NEWUSER
        | if with_mounts {
            UnshareFlags::NEWNS
        } else {
            UnshareFlags::empty()
        };
    thread::unshare(flags)?;

    // map our user to the provided Id
    let mut uid_map = File::options().write(true).open("/proc/self/uid_map")?;
    let uid_mapping = format!("{} {} {}", id.raw_uid(), me.raw_uid(), 1);
    uid_map.write_all(uid_mapping.as_bytes())?;
    drop(uid_map);
    drop(uid_mapping);

    let mut setgroup_policy = File::options().write(true).open("/proc/self/setgroups")?;
    setgroup_policy.write_all("deny".as_bytes())?;
    drop(setgroup_policy);

    // map our group to the provided Id
    let mut gid_map = File::options().write(true).open("/proc/self/gid_map")?;
    let gid_mapping = format!("{} {} {}", id.raw_gid(), me.raw_gid(), 1);
    gid_map.write_all(gid_mapping.as_bytes())?;
    drop(gid_map);
    drop(gid_mapping);

    Ok(me)
}

#[instrument(level = "trace", skip(tree))]
pub fn mount(tree: OVFSTree) -> Result<()> {
    let (upper, lower, work, mount) = tree.overlayfs_spec();
    trace!("upper: {upper}");
    trace!("lower: {lower}");
    trace!("work: {work}");
    trace!("mount: {mount}");

    Ok(())
}
