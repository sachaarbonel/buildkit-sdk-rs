pub enum OsFamily {
    Windows,
    Unix,
}

pub const DEFAULT_PATH_ENV_UNIX: &str =
    "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

/// Windows style list of directories to search for executables. Each
/// directory is separated from the next by a colon `;` character.
pub const DEFAULT_PATH_ENV_WINDOWS: &str = "c:\\Windows\\System32;c:\\Windows";

pub const fn default_path_env(family: OsFamily) -> &'static str {
    match family {
        OsFamily::Windows => DEFAULT_PATH_ENV_WINDOWS,
        OsFamily::Unix => DEFAULT_PATH_ENV_UNIX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_path_env_unix() {
        let path = default_path_env(OsFamily::Unix);
        assert_eq!(path, "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
        assert!(path.contains("/usr/local/bin"));
        assert!(path.contains("/usr/bin"));
    }

    #[test]
    fn test_default_path_env_windows() {
        let path = default_path_env(OsFamily::Windows);
        assert_eq!(path, "c:\\Windows\\System32;c:\\Windows");
        assert!(path.contains("System32"));
    }

    #[test]
    fn test_default_path_env_unix_constant() {
        assert_eq!(
            DEFAULT_PATH_ENV_UNIX,
            "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
        );
        // Unix paths are colon-separated
        let parts: Vec<&str> = DEFAULT_PATH_ENV_UNIX.split(':').collect();
        assert_eq!(parts.len(), 6);
    }

    #[test]
    fn test_default_path_env_windows_constant() {
        assert_eq!(DEFAULT_PATH_ENV_WINDOWS, "c:\\Windows\\System32;c:\\Windows");
        // Windows paths are semicolon-separated
        let parts: Vec<&str> = DEFAULT_PATH_ENV_WINDOWS.split(';').collect();
        assert_eq!(parts.len(), 2);
    }
}
