use std::{ffi::NulError, io};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct PosixError(i32);

impl PosixError {
    pub const PERMISSION_DENIED: PosixError = PosixError(libc::EPERM);
    pub const FILE_NOT_FOUND: PosixError = PosixError(libc::ENOENT);
    pub const NO_SUCH_PROCESS: PosixError = PosixError(libc::ESRCH);
    pub const INTERRUPTED_SYSTEM_CALL: PosixError = PosixError(libc::EINTR);
    pub const INPUT_OUTPUT_ERROR: PosixError = PosixError(libc::EIO);
    pub const NO_SUCH_DEVICE_OR_ADDRESS: PosixError = PosixError(libc::ENXIO);
    pub const ARGUMENT_LIST_TOO_LONG: PosixError = PosixError(libc::E2BIG);
    pub const EXEC_FORMAT_ERROR: PosixError = PosixError(libc::ENOEXEC);
    pub const BAD_FILE_DESCRIPTOR: PosixError = PosixError(libc::EBADF);
    pub const NO_CHILD_PROCESSES: PosixError = PosixError(libc::ECHILD);
    pub const RESOURCE_DEADLOCK_AVOIDED: PosixError = PosixError(libc::EDEADLK);
    pub const OUT_OF_MEMORY: PosixError = PosixError(libc::ENOMEM);
    pub const PERMISSION_DENIED_ACCESS: PosixError = PosixError(libc::EACCES);
    pub const BAD_ADDRESS: PosixError = PosixError(libc::EFAULT);
    pub const BLOCK_DEVICE_REQUIRED: PosixError = PosixError(libc::ENOTBLK);
    pub const DEVICE_OR_RESOURCE_BUSY: PosixError = PosixError(libc::EBUSY);
    pub const FILE_EXISTS: PosixError = PosixError(libc::EEXIST);
    pub const INVALID_CROSS_DEVICE_LINK: PosixError = PosixError(libc::EXDEV);
    pub const NO_SUCH_DEVICE: PosixError = PosixError(libc::ENODEV);
    pub const NOT_A_DIRECTORY: PosixError = PosixError(libc::ENOTDIR);
    pub const IS_A_DIRECTORY: PosixError = PosixError(libc::EISDIR);
    pub const INVALID_ARGUMENT: PosixError = PosixError(libc::EINVAL);
    pub const TOO_MANY_OPEN_FILES: PosixError = PosixError(libc::EMFILE);
    pub const TOO_MANY_FILES_IN_SYSTEM: PosixError = PosixError(libc::ENFILE);
    pub const INAPPROPRIATE_IOCTL_FOR_DEVICE: PosixError = PosixError(libc::ENOTTY);
    pub const TEXT_FILE_BUSY: PosixError = PosixError(libc::ETXTBSY);
    pub const FILE_TOO_LARGE: PosixError = PosixError(libc::EFBIG);
    pub const NO_SPACE_LEFT_ON_DEVICE: PosixError = PosixError(libc::ENOSPC);
    pub const ILLEGAL_SEEK: PosixError = PosixError(libc::ESPIPE);
    pub const READ_ONLY_FILE_SYSTEM: PosixError = PosixError(libc::EROFS);
    pub const TOO_MANY_LINKS: PosixError = PosixError(libc::EMLINK);
    pub const BROKEN_PIPE: PosixError = PosixError(libc::EPIPE);
    pub const DOMAIN_ERROR: PosixError = PosixError(libc::EDOM);
    pub const RESULT_TOO_LARGE: PosixError = PosixError(libc::ERANGE);
    pub const RESOURCE_UNAVAILABLE_TRY_AGAIN: PosixError = PosixError(libc::EAGAIN);
    pub const OPERATION_WOULD_BLOCK: PosixError = PosixError(libc::EWOULDBLOCK);
    pub const OPERATION_IN_PROGRESS: PosixError = PosixError(libc::EINPROGRESS);
    pub const OPERATION_ALREADY_IN_PROGRESS: PosixError = PosixError(libc::EALREADY);
    pub const NOT_A_SOCKET: PosixError = PosixError(libc::ENOTSOCK);
    pub const MESSAGE_SIZE: PosixError = PosixError(libc::EMSGSIZE);
    pub const PROTOCOL_WRONG_TYPE: PosixError = PosixError(libc::EPROTOTYPE);
    pub const PROTOCOL_NOT_AVAILABLE: PosixError = PosixError(libc::ENOPROTOOPT);
    pub const PROTOCOL_NOT_SUPPORTED: PosixError = PosixError(libc::EPROTONOSUPPORT);
    pub const SOCKET_TYPE_NOT_SUPPORTED: PosixError = PosixError(libc::ESOCKTNOSUPPORT);
    pub const OPERATION_NOT_SUPPORTED: PosixError = PosixError(libc::EOPNOTSUPP);
    pub const PROTOCOL_FAMILY_NOT_SUPPORTED: PosixError = PosixError(libc::EPFNOSUPPORT);
    pub const ADDRESS_FAMILY_NOT_SUPPORTED: PosixError = PosixError(libc::EAFNOSUPPORT);
    pub const ADDRESS_IN_USE: PosixError = PosixError(libc::EADDRINUSE);
    pub const ADDRESS_NOT_AVAILABLE: PosixError = PosixError(libc::EADDRNOTAVAIL);
    pub const NETWORK_DOWN: PosixError = PosixError(libc::ENETDOWN);
    pub const NETWORK_UNREACHABLE: PosixError = PosixError(libc::ENETUNREACH);
    pub const NETWORK_RESET: PosixError = PosixError(libc::ENETRESET);
    pub const CONNECTION_ABORTED: PosixError = PosixError(libc::ECONNABORTED);
    pub const CONNECTION_RESET: PosixError = PosixError(libc::ECONNRESET);
    pub const NO_BUFFER_SPACE_AVAILABLE: PosixError = PosixError(libc::ENOBUFS);
    pub const ALREADY_CONNECTED: PosixError = PosixError(libc::EISCONN);
    pub const NOT_CONNECTED: PosixError = PosixError(libc::ENOTCONN);
    pub const DESTINATION_ADDRESS_REQUIRED: PosixError = PosixError(libc::EDESTADDRREQ);
    pub const SHUTDOWN: PosixError = PosixError(libc::ESHUTDOWN);
    pub const TOO_MANY_REFERENCES: PosixError = PosixError(libc::ETOOMANYREFS);
    pub const TIMED_OUT: PosixError = PosixError(libc::ETIMEDOUT);
    pub const CONNECTION_REFUSED: PosixError = PosixError(libc::ECONNREFUSED);
    pub const TOO_MANY_SYMBOLIC_LINKS: PosixError = PosixError(libc::ELOOP);
    pub const FILE_NAME_TOO_LONG: PosixError = PosixError(libc::ENAMETOOLONG);
    pub const HOST_IS_DOWN: PosixError = PosixError(libc::EHOSTDOWN);
    pub const NO_ROUTE_TO_HOST: PosixError = PosixError(libc::EHOSTUNREACH);
    pub const DIRECTORY_NOT_EMPTY: PosixError = PosixError(libc::ENOTEMPTY);
    pub const TOO_MANY_USERS: PosixError = PosixError(libc::EUSERS);
    pub const QUOTA_EXCEEDED: PosixError = PosixError(libc::EDQUOT);
    pub const STALE_FILE_HANDLE: PosixError = PosixError(libc::ESTALE);
    pub const OBJECT_IS_REMOTE: PosixError = PosixError(libc::EREMOTE);
    pub const NO_LOCKS_AVAILABLE: PosixError = PosixError(libc::ENOLCK);
    pub const FUNCTION_NOT_IMPLEMENTED: PosixError = PosixError(libc::ENOSYS);
    pub const LIBRARY_ERROR: PosixError = PosixError(libc::ELIBEXEC);
    pub const NOT_SUPPORTED: PosixError = PosixError(libc::ENOTSUP);
    pub const ILLEGAL_BYTE_SEQUENCE: PosixError = PosixError(libc::EILSEQ);
    pub const BAD_MESSAGE: PosixError = PosixError(libc::EBADMSG);
    pub const IDENTIFIER_REMOVED: PosixError = PosixError(libc::EIDRM);
    pub const MULTIHOP_ATTEMPTED: PosixError = PosixError(libc::EMULTIHOP);
    pub const NO_DATA_AVAILABLE: PosixError = PosixError(libc::ENODATA);
    pub const LINK_HAS_BEEN_SEVERED: PosixError = PosixError(libc::ENOLINK);
    pub const NO_MESSAGE: PosixError = PosixError(libc::ENOMSG);
    pub const OUT_OF_STREAMS: PosixError = PosixError(libc::ENOSR);
}

impl From<PosixError> for io::Error {
    fn from(value: PosixError) -> Self {
        Self::from_raw_os_error(value.0)
    }
}

impl From<PosixError> for i32 {
    fn from(value: PosixError) -> Self {
        value.0
    }
}

impl From<NulError> for PosixError {
    fn from(_value: NulError) -> Self {
        PosixError::INVALID_ARGUMENT
    }
}

pub fn from_last_errno() -> io::Error {
    std::io::Error::last_os_error()
}
