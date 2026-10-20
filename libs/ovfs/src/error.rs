use miette::Diagnostic;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum OVFSError {
    #[error(transparent)]
    #[diagnostic(code(ovfs::syscall_error))]
    SyscallError {
        #[from]
        source: rustix::io::Errno,
    },

    #[error(transparent)]
    #[diagnostic(code(ovfs::io_error))]
    IOError {
        #[from]
        source: std::io::Error,
    },
}
