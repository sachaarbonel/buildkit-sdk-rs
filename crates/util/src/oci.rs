use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OciBackend {
    #[default]
    Docker,
    Podman,
}

impl OciBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            OciBackend::Docker => "docker",
            OciBackend::Podman => "podman",
        }
    }
}

impl fmt::Display for OciBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug)]
pub struct OciBackendFromStrError(String);

impl std::error::Error for OciBackendFromStrError {}

impl fmt::Display for OciBackendFromStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown OCI backend: {}", self.0)
    }
}

impl FromStr for OciBackend {
    type Err = OciBackendFromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "docker" => Ok(OciBackend::Docker),
            "podman" => Ok(OciBackend::Podman),
            _ => Err(OciBackendFromStrError(s.to_owned())),
        }
    }
}

impl AsRef<str> for OciBackend {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_docker() {
        let backend = OciBackend::default();
        assert_eq!(backend, OciBackend::Docker);
    }

    #[test]
    fn test_debug_formatting() {
        assert_eq!(format!("{:?}", OciBackend::Docker), "Docker");
        assert_eq!(format!("{:?}", OciBackend::Podman), "Podman");
    }

    #[test]
    fn test_clone_and_partial_eq() {
        let backend = OciBackend::Docker;
        let cloned = backend;
        assert_eq!(backend, cloned);

        let podman = OciBackend::Podman;
        assert_ne!(backend, podman);
    }

    #[test]
    fn test_display() {
        assert_eq!(OciBackend::Docker.to_string(), "docker");
        assert_eq!(OciBackend::Podman.to_string(), "podman");
    }

    #[test]
    fn test_from_str() {
        assert_eq!("docker".parse::<OciBackend>().unwrap(), OciBackend::Docker);
        assert_eq!("podman".parse::<OciBackend>().unwrap(), OciBackend::Podman);
        assert!("unknown".parse::<OciBackend>().is_err());
    }

    #[test]
    fn test_as_ref() {
        let backend: &str = OciBackend::Docker.as_ref();
        assert_eq!(backend, "docker");
    }
}
