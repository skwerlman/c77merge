use miette::{Diagnostic, Report};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum OVFSError {
    #[error(transparent)]
    #[diagnostic(code(ovfs::syscall_error))]
    SyscallError(#[from] rustix::io::Errno),

    #[error(transparent)]
    #[diagnostic(code(ovfs::io_error))]
    IOError(#[from] std::io::Error),

    #[error("{0}")]
    Report(Report),
}

impl From<Report> for OVFSError {
    fn from(report: Report) -> Self {
        OVFSError::Report(report)
    }
}
