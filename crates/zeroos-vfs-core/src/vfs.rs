use crate::{DeviceFactory, Fd, FdEntry, VfsResult};
use foundation::utils::GlobalCell;

const MAX_FDS: usize = 256;

pub struct Vfs {
    fd_table: [Option<FdEntry>; MAX_FDS],
    next_fd: Fd,
    devices: [(Option<&'static str>, Option<DeviceFactory>); 32],
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs {
    /// Create a new VFS instance
    pub const fn new() -> Self {
        const NONE: (Option<&'static str>, Option<DeviceFactory>) = (None, None);
        Self {
            fd_table: [None; MAX_FDS],
            next_fd: 3,
            devices: [NONE; 32],
        }
    }

    pub fn register_fd(&mut self, fd: Fd, entry: FdEntry) -> VfsResult<()> {
        if fd < 0 || fd as usize >= MAX_FDS {
            return Err(-(libc::EINVAL as isize));
        }
        self.fd_table[fd as usize] = Some(entry);
        Ok(())
    }

    pub fn register_device(&mut self, path: &'static str, factory: DeviceFactory) -> VfsResult<()> {
        for entry in &mut self.devices {
            if entry.0.is_none() {
                *entry = (Some(path), Some(factory));
                return Ok(());
            }
        }
        Err(-(libc::ENOMEM as isize))
    }

    pub fn open(&mut self, path: &str, _flags: i32, _mode: u32) -> VfsResult<Fd> {
        let factory = self
            .devices
            .iter()
            .find(|(p, _)| p.is_some_and(|device_path| device_path == path))
            .and_then(|(_, f)| *f)
            .ok_or(-(libc::ENOENT as isize))?;

        let mut found: Option<Fd> = None;
        let start = self.next_fd.max(3) as usize;
        for idx in start..MAX_FDS {
            if self.fd_table[idx].is_none() {
                found = Some(idx as Fd);
                break;
            }
        }
        if found.is_none() {
            for idx in 3..start.min(MAX_FDS) {
                if self.fd_table[idx].is_none() {
                    found = Some(idx as Fd);
                    break;
                }
            }
        }
        let fd = match found {
            Some(fd) => fd,
            None => return Err(-(libc::EMFILE as isize)),
        };
        self.next_fd = if (fd as usize) + 1 < MAX_FDS {
            fd + 1
        } else {
            3
        };

        let entry = factory();
        self.fd_table[fd as usize] = Some(entry);

        Ok(fd)
    }

    pub fn read(&self, fd: Fd, buf: *mut u8, count: usize) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }
        if count != 0 && buf.is_null() {
            return -(libc::EFAULT as isize);
        }

        match self.fd_table[fd as usize] {
            Some(entry) => (entry.ops.read)(entry.private_data, buf, count),
            None => -(libc::EBADF as isize),
        }
    }

    pub fn write(&self, fd: Fd, buf: *const u8, count: usize) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }
        if count != 0 && buf.is_null() {
            return -(libc::EFAULT as isize);
        }

        match self.fd_table[fd as usize] {
            Some(entry) => (entry.ops.write)(entry.private_data, buf, count),
            None => -(libc::EBADF as isize),
        }
    }

    pub fn lseek(&self, fd: Fd, offset: isize, whence: i32) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }

        match self.fd_table[fd as usize] {
            Some(entry) => (entry.ops.llseek)(entry.private_data, offset, whence),
            None => -(libc::EBADF as isize),
        }
    }

    pub fn ioctl(&self, fd: Fd, request: usize, arg: usize) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }

        match self.fd_table[fd as usize] {
            Some(entry) => (entry.ops.ioctl)(entry.private_data, request, arg),
            None => -(libc::EBADF as isize),
        }
    }

    pub fn close(&mut self, fd: Fd) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }

        match self.fd_table[fd as usize].take() {
            Some(entry) => (entry.ops.release)(entry.private_data),
            None => -(libc::EBADF as isize),
        }
    }

    pub fn fstat(&self, fd: Fd, statbuf: *mut libc::stat) -> isize {
        if fd < 0 || fd as usize >= MAX_FDS {
            return -(libc::EBADF as isize);
        }

        if statbuf.is_null() {
            return -(libc::EFAULT as isize);
        }

        -(libc::ENOSYS as isize)
    }
}

static VFS: GlobalCell<Vfs> = GlobalCell::new(Vfs::new());

pub fn register_fd(fd: Fd, entry: FdEntry) -> VfsResult<()> {
    VFS.with_mut(|vfs| vfs.register_fd(fd, entry))
}

pub fn register_device(path: &'static str, factory: DeviceFactory) -> VfsResult<()> {
    VFS.with_mut(|vfs| vfs.register_device(path, factory))
}

pub fn read(fd: Fd, buf: *mut u8, count: usize) -> isize {
    VFS.with(|vfs| vfs.read(fd, buf, count))
}

pub fn write(fd: Fd, buf: *const u8, count: usize) -> isize {
    VFS.with(|vfs| vfs.write(fd, buf, count))
}

pub fn lseek(fd: Fd, offset: isize, whence: i32) -> isize {
    VFS.with(|vfs| vfs.lseek(fd, offset, whence))
}

pub fn ioctl(fd: Fd, request: usize, arg: usize) -> isize {
    VFS.with(|vfs| vfs.ioctl(fd, request, arg))
}

pub fn close(fd: Fd) -> isize {
    VFS.with_mut(|vfs| vfs.close(fd))
}

pub fn fstat(fd: Fd, statbuf: *mut libc::stat) -> isize {
    VFS.with(|vfs| vfs.fstat(fd, statbuf))
}

pub(crate) fn fstat_raw(fd: Fd, statbuf: *mut u8) -> isize {
    fstat(fd, statbuf as *mut libc::stat)
}

pub const VFS_OPS: crate::VfsOps = crate::VfsOps {
    init: || {},
    read,
    write,
    open: open_cstr,
    close,
    lseek,
    ioctl,
    fstat: fstat_raw,
};

/// # Safety
/// `path` must be a valid NUL-terminated string.
pub unsafe fn open_cstr(path: *const u8, flags: i32, mode: u32) -> isize {
    if path.is_null() {
        return -(libc::EFAULT as isize);
    }

    let mut len = 0;
    while *path.add(len) != 0 {
        len += 1;
        if len > 4096 {
            return -(libc::ENAMETOOLONG as isize);
        }
    }
    let slice = core::slice::from_raw_parts(path, len);
    match core::str::from_utf8(slice) {
        Ok(s) => VFS.with_mut(|vfs| match vfs.open(s, flags, mode) {
            Ok(fd) => fd as isize,
            Err(e) => e,
        }),
        Err(_) => -(libc::EINVAL as isize),
    }
}
