use std::{fs::File, io::Write, os::fd::AsFd};

use rustix::fs;
use rustix::mount::{FsMountFlags, FsOpenFlags, MountAttrFlags, MoveMountFlags};
use rustix::mount::{fsconfig_create, fsconfig_set_string, fsmount, fsopen, move_mount};
use rustix::thread::{self, UnshareFlags};
use tracing::{debug, instrument, trace};

pub use error::OVFSError;
pub use id::Id;
use ovfs::OVFSTree;

mod error;
mod id;
pub mod ovfs;

type Result<T> = std::result::Result<T, OVFSError>;

// sets up a user namespace and returns who we used to be
#[instrument(level = "trace")]
pub fn unshare(id: Id, with_mounts: bool) -> Result<Id> {
    let me = Id::me();

    // create a new namespace allowing user mapping and maybe mounts
    debug!("creating new namespace");
    let flags = UnshareFlags::NEWUSER
        | if with_mounts {
            UnshareFlags::NEWNS
        } else {
            UnshareFlags::empty()
        };
    thread::unshare(flags)?;

    // map our user to the provided Id
    trace!("mapping user {} to {}", me.raw_uid(), id.raw_uid());
    let mut uid_map = File::options().write(true).open("/proc/self/uid_map")?;
    let uid_mapping = format!("{} {} {}", id.raw_uid(), me.raw_uid(), 1);
    uid_map.write_all(uid_mapping.as_bytes())?;
    drop(uid_map);
    drop(uid_mapping);

    let mut setgroup_policy = File::options().write(true).open("/proc/self/setgroups")?;
    setgroup_policy.write_all("deny".as_bytes())?;
    drop(setgroup_policy);

    // map our group to the provided Id
    trace!("mapping group {} to {}", me.raw_gid(), id.raw_gid());
    let mut gid_map = File::options().write(true).open("/proc/self/gid_map")?;
    let gid_mapping = format!("{} {} {}", id.raw_gid(), me.raw_gid(), 1);
    gid_map.write_all(gid_mapping.as_bytes())?;
    drop(gid_map);
    drop(gid_mapping);

    Ok(me)
}

#[instrument(level = "trace", skip(tree))]
pub fn mount(tree: OVFSTree, readonly: bool) -> Result<()> {
    debug!("mounting vfs");
    let (upper, lowers, work, mount) = tree.overlayfs_spec();

    trace!("upper: {upper}");
    trace!("lower: {lowers:?}");
    trace!("work: {work}");
    trace!("mount: {mount}");

    trace!("fsopen");
    let fd = fsopen("overlay", FsOpenFlags::FSOPEN_CLOEXEC)?;

    // configure and create the overlay
    trace!("fsconfig_set_string source");
    fsconfig_set_string(fd.as_fd(), "source", "overlay")?;
    for lower in lowers {
        trace!("fsconfig_set_string lowerdir+ {lower}");
        fsconfig_set_string(fd.as_fd(), "lowerdir+", lower)?;
    }

    if !readonly {
        trace!("fsconfig_set_string upperdir {upper}");
        fsconfig_set_string(fd.as_fd(), "upperdir", upper)?;
        trace!("fsconfig_set_string workdir {work}");
        fsconfig_set_string(fd.as_fd(), "workdir", work)?;
    }

    trace!("fsconfig_create");
    fsconfig_create(fd.as_fd())?;

    // mount it
    trace!("fsmount");
    let mount_fd = fsmount(
        fd.as_fd(),
        FsMountFlags::FSMOUNT_CLOEXEC,
        MountAttrFlags::empty(),
    )?;

    // move it to shadow the mount directory
    trace!("move_mount");
    move_mount(
        mount_fd.as_fd(),
        "",
        fs::CWD,
        mount.as_bytes(),
        MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH,
    )?;

    Ok(())
}
