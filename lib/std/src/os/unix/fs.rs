//! `std::os::unix::fs`: permission bits and raw stat fields.

use crate::fs::{self, DirBuilder, DirEntry, FileType, Metadata, OpenOptions, Permissions};
use crate::io;
use crate::path::Path;

pub trait PermissionsExt {
    fn mode(&self) -> u32;
    fn set_mode(&mut self, mode: u32);
    fn from_mode(mode: u32) -> Self;
}

impl PermissionsExt for Permissions {
    fn mode(&self) -> u32 {
        self.mode
    }
    fn set_mode(&mut self, mode: u32) {
        self.mode = mode;
    }
    fn from_mode(mode: u32) -> Permissions {
        Permissions { mode }
    }
}

pub trait OpenOptionsExt {
    fn mode(&mut self, mode: u32) -> &mut Self;
    fn custom_flags(&mut self, flags: i32) -> &mut Self;
}

impl OpenOptionsExt for OpenOptions {
    fn mode(&mut self, mode: u32) -> &mut OpenOptions {
        self.mode = mode;
        self
    }
    fn custom_flags(&mut self, flags: i32) -> &mut OpenOptions {
        self.custom_flags = flags;
        self
    }
}

pub trait DirBuilderExt {
    fn mode(&mut self, mode: u32) -> &mut Self;
}

impl DirBuilderExt for DirBuilder {
    fn mode(&mut self, mode: u32) -> &mut DirBuilder {
        self.mode = mode;
        self
    }
}

pub trait MetadataExt {
    fn dev(&self) -> u64;
    fn ino(&self) -> u64;
    fn mode(&self) -> u32;
    fn nlink(&self) -> u64;
    fn uid(&self) -> u32;
    fn gid(&self) -> u32;
    fn rdev(&self) -> u64;
    fn size(&self) -> u64;
    fn atime(&self) -> i64;
    fn atime_nsec(&self) -> i64;
    fn mtime(&self) -> i64;
    fn mtime_nsec(&self) -> i64;
    fn ctime(&self) -> i64;
    fn ctime_nsec(&self) -> i64;
    fn blksize(&self) -> u64;
    fn blocks(&self) -> u64;
}

impl MetadataExt for Metadata {
    fn dev(&self) -> u64 {
        self.st[12]
    }
    fn ino(&self) -> u64 {
        self.st[13]
    }
    fn mode(&self) -> u32 {
        self.st[1] as u32
    }
    fn nlink(&self) -> u64 {
        self.st[14]
    }
    fn uid(&self) -> u32 {
        self.st[15] as u32
    }
    fn gid(&self) -> u32 {
        self.st[16] as u32
    }
    fn rdev(&self) -> u64 {
        self.st[17]
    }
    fn size(&self) -> u64 {
        self.st[2]
    }
    fn atime(&self) -> i64 {
        self.st[5] as i64
    }
    fn atime_nsec(&self) -> i64 {
        self.st[6] as i64
    }
    fn mtime(&self) -> i64 {
        self.st[3] as i64
    }
    fn mtime_nsec(&self) -> i64 {
        self.st[4] as i64
    }
    fn ctime(&self) -> i64 {
        self.st[7] as i64
    }
    fn ctime_nsec(&self) -> i64 {
        self.st[8] as i64
    }
    fn blksize(&self) -> u64 {
        self.st[18]
    }
    fn blocks(&self) -> u64 {
        self.st[19]
    }
}

pub trait FileTypeExt {
    fn is_block_device(&self) -> bool;
    fn is_char_device(&self) -> bool;
    fn is_fifo(&self) -> bool;
    fn is_socket(&self) -> bool;
}

impl FileTypeExt for FileType {
    fn is_block_device(&self) -> bool {
        FileType::is_block_device(self)
    }
    fn is_char_device(&self) -> bool {
        FileType::is_char_device(self)
    }
    fn is_fifo(&self) -> bool {
        FileType::is_fifo(self)
    }
    fn is_socket(&self) -> bool {
        FileType::is_socket(self)
    }
}

pub trait DirEntryExt {
    fn ino(&self) -> u64;
}

impl DirEntryExt for DirEntry {
    fn ino(&self) -> u64 {
        DirEntry::ino(self)
    }
}

pub fn symlink<P: AsRef<Path>, Q: AsRef<Path>>(original: P, link: Q) -> io::Result<()> {
    fs::symlink_impl(original.as_ref(), link.as_ref())
}
