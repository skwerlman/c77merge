use std::{fs::File, io::Write, os::fd::AsFd};

use miette::{IntoDiagnostic, WrapErr};
use rustix::fs;
use rustix::mount::{FsMountFlags, FsOpenFlags, MountAttrFlags, MoveMountFlags, fsconfig_set_flag};
use rustix::mount::{fsconfig_create_exclusive, fsconfig_set_string, fsmount, fsopen, move_mount};
use rustix::thread::{self, UnshareFlags};
use tracing::{debug, instrument, trace};

pub use error::OVFSError;
pub use id::Id;
use ovfs::OVFSTree;

mod error;
mod id;
pub mod ovfs;

pub(crate) type Result<T> = miette::Result<T>;

// sets up a user namespace and returns who we used to be
#[instrument(level = "trace")]
pub fn unshare(id: Id, with_mounts: bool) -> Result<Id> {
    debug!("creating new namespace");
    let me = Id::me();

    // create a new namespace allowing user mapping and maybe mounts
    let flags = UnshareFlags::NEWUSER
        | if with_mounts {
            debug!("enabling mounting for namespace");
            UnshareFlags::NEWNS
        } else {
            UnshareFlags::empty()
        };

    // SAFETY: we don't use UnshareFlags::FILES, so the concern about unsafe file descriptors does not apply here
    unsafe {
        thread::unshare_unsafe(flags)
            .map_err(|e| OVFSError::SyscallError { source: e })
            .wrap_err(format!("flags: {:?}", flags))?;
    }

    // map our user to the provided Id
    trace!("mapping user {} to {}", me.raw_uid(), id.raw_uid());
    let mut uid_map = File::options()
        .write(true)
        .open("/proc/self/uid_map")
        .into_diagnostic()
        .wrap_err("Failed to open uid_map")?;
    let uid_mapping = format!("{} {} {}", id.raw_uid(), me.raw_uid(), 1);
    uid_map
        .write_all(uid_mapping.as_bytes())
        .into_diagnostic()
        .wrap_err("Failed to write uid mapping")?;
    drop(uid_map);
    drop(uid_mapping);

    let mut setgroup_policy = File::options()
        .write(true)
        .open("/proc/self/setgroups")
        .into_diagnostic()
        .wrap_err("Failed to open setgroups")?;
    setgroup_policy
        .write_all("deny".as_bytes())
        .into_diagnostic()
        .wrap_err("Failed to write setgroups policy")?;
    drop(setgroup_policy);

    // map our group to the provided Id
    trace!("mapping group {} to {}", me.raw_gid(), id.raw_gid());
    let mut gid_map = File::options()
        .write(true)
        .open("/proc/self/gid_map")
        .into_diagnostic()
        .wrap_err("Failed to open gid_map")?;
    let gid_mapping = format!("{} {} {}", id.raw_gid(), me.raw_gid(), 1);
    gid_map
        .write_all(gid_mapping.as_bytes())
        .into_diagnostic()
        .wrap_err("Failed to write gid mapping")?;
    drop(gid_map);
    drop(gid_mapping);

    Ok(me)
}

#[instrument(level = "trace", skip(tree))]
pub fn mount(tree: OVFSTree) -> Result<()> {
    debug!("mounting vfs");

    let (upper, lowers, work, mount) = tree.overlayfs_spec();

    trace!("upper: {upper:?}");
    trace!("lower: {lowers:?}");
    trace!("work: {work:?}");
    trace!("mount: {mount:?}");

    trace!("fsopen");
    let fd = fsopen("overlay", FsOpenFlags::FSOPEN_CLOEXEC)
        .into_diagnostic()
        .wrap_err("Failed to open overlayfs")?;

    // configure and create the overlay
    trace!("fsconfig_set_string index off");
    fsconfig_set_string(fd.as_fd(), "index", "off")
        .into_diagnostic()
        .wrap_err("Failed to set index")?;
    trace!("fsconfig_set_string xino off");
    fsconfig_set_string(fd.as_fd(), "xino", "off")
        .into_diagnostic()
        .wrap_err("Failed to set xino")?;
    trace!("fsconfig_set_string metacopy off");
    fsconfig_set_string(fd.as_fd(), "metacopy", "off")
        .into_diagnostic()
        .wrap_err("Failed to set metacopy")?;
    trace!("fsconfig_set_string redirect_dir off");
    fsconfig_set_string(fd.as_fd(), "redirect_dir", "off")
        .into_diagnostic()
        .wrap_err("Failed to set redirect_dir")?;
    trace!("fsconfig_set_string uuid null");
    fsconfig_set_string(fd.as_fd(), "uuid", "null")
        .into_diagnostic()
        .wrap_err("Failed to set uuid")?;
    trace!("fsconfig_set_flag userxattr");
    fsconfig_set_flag(fd.as_fd(), "userxattr")
        .into_diagnostic()
        .wrap_err("Failed to set userxattr")?;
    trace!("fsconfig_set_string source overlay");
    fsconfig_set_string(fd.as_fd(), "source", "overlay")
        .into_diagnostic()
        .wrap_err("Failed to set source")?;
    for lower in lowers {
        let path = lower.canonicalize().into_diagnostic().wrap_err(format!(
            "Failed to canonicalize lowerdir: `{}`",
            lower.display()
        ))?;
        trace!("fsconfig_set_string lowerdir+ {path:?}");
        fsconfig_set_string(fd.as_fd(), "lowerdir+", &path)
            .into_diagnostic()
            .wrap_err(format!("Failed to append lowerdir `{}`. This can be caused by an incompatible lowerdir, such as a case-sensitive filesystem.", path.display()))?;
    }

    trace!("fsconfig_set_string upperdir {upper:?}");
    fsconfig_set_string(fd.as_fd(), "upperdir", upper)
        .into_diagnostic()
        .wrap_err("Failed to set upperdir")?;
    trace!("fsconfig_set_string workdir {work:?}");
    fsconfig_set_string(fd.as_fd(), "workdir", work)
        .into_diagnostic()
        .wrap_err("Failed to set workdir")?;

    // if rustix::
    trace!("fsconfig_create_exclusive");
    fsconfig_create_exclusive(fd.as_fd())
        .into_diagnostic()
        .wrap_err("Failed to create overlayfs")?;

    // mount it
    trace!("fsmount");
    let mount_fd = fsmount(
        fd.as_fd(),
        FsMountFlags::FSMOUNT_CLOEXEC,
        MountAttrFlags::empty(),
    )
    .into_diagnostic()
    .wrap_err("Failed to mount overlayfs")?;

    // move it to shadow the mount directory
    trace!("move_mount");
    move_mount(
        mount_fd.as_fd(),
        "",
        fs::CWD,
        mount,
        MoveMountFlags::MOVE_MOUNT_F_EMPTY_PATH,
    )
    .into_diagnostic()
    .wrap_err("Failed to move overlayfs")?;

    Ok(())
}
