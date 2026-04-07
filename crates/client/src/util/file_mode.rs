use std::fs::Metadata;

use bitflags::bitflags;

bitflags! {
    /// A Rust version of <https://pkg.go.dev/io/fs#FileMode>
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct FileMode: u32 {
        const MODE_DIR        = 1 << (32 - 1);
        const MODE_APPEND     = 1 << (32 - 1 - 1);
        const MODE_EXCLUSIVE  = 1 << (32 - 1 - 2);
        const MODE_TEMPORARY  = 1 << (32 - 1 - 3);
        const MODE_SYMLINK    = 1 << (32 - 1 - 4);
        const MODE_DEVICE     = 1 << (32 - 1 - 5);
        const MODE_NAMED_PIPE = 1 << (32 - 1 - 6);
        const MODE_SOCKET     = 1 << (32 - 1 - 7);
        const MODE_SETUID     = 1 << (32 - 1 - 8);
        const MODE_SETGID     = 1 << (32 - 1 - 9);
        const MODE_CHAR_DEVICE = 1 << (32 - 1 - 10);
        const MODE_STICKY     = 1 << (32 - 1 - 11);
        const MODE_IRREGULAR  = 1 << (32 - 1 - 12);

        const OTHER_EXE = 1;
        const OTHER_WRITE = 1 << 1;
        const OTHER_READ = 1 << 2;
        const GROUP_EXE = 1 << 3;
        const GROUP_WRITE = 1 << 4;
        const GROUP_READ = 1 << 5;
        const USER_EXE = 1 << 6;
        const USER_WRITE = 1 << 7;
        const USER_READ = 1 << 8;

        const MODE_PERM_MASK = Self::OTHER_EXE.bits()
            | Self::OTHER_WRITE.bits()
            | Self::OTHER_READ.bits()
            | Self::GROUP_EXE.bits()
            | Self::GROUP_WRITE.bits()
            | Self::GROUP_READ.bits()
            | Self::USER_EXE.bits()
            | Self::USER_WRITE.bits()
            | Self::USER_READ.bits();


        const MODE_TYPE_MASK = Self::MODE_DIR.bits()
            | Self::MODE_SYMLINK.bits()
            | Self::MODE_NAMED_PIPE.bits()
            | Self::MODE_SOCKET.bits()
            | Self::MODE_DEVICE.bits()
            | Self::MODE_CHAR_DEVICE.bits()
            | Self::MODE_IRREGULAR.bits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitflags_combination() {
        let mode = FileMode::USER_READ | FileMode::USER_WRITE | FileMode::USER_EXE;
        assert!(mode.contains(FileMode::USER_READ));
        assert!(mode.contains(FileMode::USER_WRITE));
        assert!(mode.contains(FileMode::USER_EXE));
        assert!(!mode.contains(FileMode::GROUP_READ));
    }

    #[test]
    fn test_permission_mask() {
        let all_perms = FileMode::MODE_PERM_MASK;
        // 0o777 = 0x1ff = 511
        assert_eq!(all_perms.bits(), 0o777);
        assert!(all_perms.contains(FileMode::USER_READ));
        assert!(all_perms.contains(FileMode::USER_WRITE));
        assert!(all_perms.contains(FileMode::USER_EXE));
        assert!(all_perms.contains(FileMode::GROUP_READ));
        assert!(all_perms.contains(FileMode::GROUP_WRITE));
        assert!(all_perms.contains(FileMode::GROUP_EXE));
        assert!(all_perms.contains(FileMode::OTHER_READ));
        assert!(all_perms.contains(FileMode::OTHER_WRITE));
        assert!(all_perms.contains(FileMode::OTHER_EXE));
        // Type bits should not be in perm mask
        assert!(!all_perms.contains(FileMode::MODE_DIR));
    }

    #[test]
    fn test_type_mask() {
        let type_mask = FileMode::MODE_TYPE_MASK;
        assert!(type_mask.contains(FileMode::MODE_DIR));
        assert!(type_mask.contains(FileMode::MODE_SYMLINK));
        assert!(type_mask.contains(FileMode::MODE_NAMED_PIPE));
        assert!(type_mask.contains(FileMode::MODE_SOCKET));
        assert!(type_mask.contains(FileMode::MODE_DEVICE));
        assert!(type_mask.contains(FileMode::MODE_CHAR_DEVICE));
        assert!(type_mask.contains(FileMode::MODE_IRREGULAR));
        // Permission bits should not be in type mask
        assert!(!type_mask.contains(FileMode::USER_READ));
    }
}

impl FileMode {
    pub(crate) fn from_metadata(metadata: &Metadata) -> Self {
        let mut mode = Self::empty();

        #[cfg(unix)]
        let unix_mode = {
            use std::os::unix::fs::MetadataExt;
            metadata.mode()
        };

        // Trying to emulate the behavior of go on windows where it makes fake mode bits
        #[cfg(windows)]
        let unix_mode = {
            let readonly = metadata.permissions().readonly();
            let is_dir = metadata.is_dir();
            match (readonly, is_dir) {
                (true, false) => 0o444,
                (true, true) => 0o555,
                (false, false) => 0o666,
                (false, true) => 0o777,
            }
        };

        mode |= FileMode::from_bits_truncate(unix_mode) & Self::MODE_PERM_MASK;

        if metadata.is_dir() {
            mode |= Self::MODE_DIR;
        }

        if metadata.is_symlink() {
            mode |= Self::MODE_SYMLINK;
        }

        mode
    }
}
