use buildkit_rs_proto::pb;
use std::{borrow::Cow, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Platform {
    /// The architecture of the platform
    pub architecture: Cow<'static, str>,
    /// The name of the operating system
    pub os: Cow<'static, str>,
    /// The variant of the architecture
    pub variant: Option<Cow<'static, str>>,
}

impl Platform {
    pub const fn new(os: &'static str, arch: &'static str, variant: Option<&'static str>) -> Self {
        Self {
            architecture: Cow::Borrowed(arch),
            os: Cow::Borrowed(os),
            // We have to manually map since `map` is not const yet
            // TODO: Replace with `map` when it is const
            variant: match variant {
                Some(variant) => Some(Cow::Borrowed(variant)),
                None => None,
            },
        }
    }

    pub const LINUX_AMD64: Platform = Platform::new("linux", "amd64", None);
    pub const LINUX_ARMHF: Platform = Platform::new("linux", "arm", Some("v7"));
    pub const LINUX_ARM: Platform = Platform::new("linux", "arm", Some("v7"));
    pub const LINUX_ARMEL: Platform = Platform::new("linux", "arm", Some("v6"));
    pub const LINUX_ARM64: Platform = Platform::new("linux", "arm64", None);
    pub const LINUX_S390X: Platform = Platform::new("linux", "s390x", None);
    pub const LINUX_PPC64: Platform = Platform::new("linux", "ppc64", None);
    pub const LINUX_PPC64LE: Platform = Platform::new("linux", "ppc64le", None);
    pub const DARWIN: Platform = Platform::new("darwin", "amd64", None);
    pub const WINDOWS: Platform = Platform::new("windows", "amd64", None);

    pub(crate) fn to_pb(&self) -> pb::Platform {
        pb::Platform {
            architecture: self.architecture.clone().into_owned(),
            os: self.os.clone().into_owned(),
            variant: self
                .variant
                .as_ref()
                .map(|v| v.clone().into_owned())
                .unwrap_or_default(),
            ..Default::default()
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Platform {
            architecture,
            os,
            variant,
        } = self;

        write!(f, "{os}/{architecture}")?;
        if let Some(variant) = variant {
            write!(f, "-{variant}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.splitn(2, '/');
        let os = parts.next().unwrap();
        let mut parts = parts.next().unwrap().splitn(2, '-');
        let architecture = parts.next().unwrap();
        let variant = parts.next();

        Ok(Self {
            architecture: Cow::Owned(architecture.to_owned()),
            os: Cow::Owned(os.to_owned()),
            variant: variant.map(|v| Cow::Owned(v.to_owned())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_display() {
        assert_eq!(Platform::LINUX_AMD64.to_string(), "linux/amd64");
        assert_eq!(Platform::LINUX_ARMHF.to_string(), "linux/arm-v7");
        assert_eq!(Platform::LINUX_ARM.to_string(), "linux/arm-v7");
        assert_eq!(Platform::LINUX_ARMEL.to_string(), "linux/arm-v6");
        assert_eq!(Platform::LINUX_ARM64.to_string(), "linux/arm64");
        assert_eq!(Platform::LINUX_S390X.to_string(), "linux/s390x");
        assert_eq!(Platform::LINUX_PPC64.to_string(), "linux/ppc64");
        assert_eq!(Platform::LINUX_PPC64LE.to_string(), "linux/ppc64le");
        assert_eq!(Platform::DARWIN.to_string(), "darwin/amd64");
        assert_eq!(Platform::WINDOWS.to_string(), "windows/amd64");
    }

    #[test]
    fn test_platform_from_str() {
        let p: Platform = "linux/amd64".parse().unwrap();
        assert_eq!(p.os, "linux");
        assert_eq!(p.architecture, "amd64");
        assert!(p.variant.is_none());

        let p: Platform = "linux/arm-v7".parse().unwrap();
        assert_eq!(p.os, "linux");
        assert_eq!(p.architecture, "arm");
        assert_eq!(p.variant.as_deref(), Some("v7"));
    }

    #[test]
    fn test_platform_to_pb() {
        let pb = Platform::LINUX_AMD64.to_pb();
        assert_eq!(pb.os, "linux");
        assert_eq!(pb.architecture, "amd64");
        assert_eq!(pb.variant, "");

        let pb = Platform::LINUX_ARM.to_pb();
        assert_eq!(pb.os, "linux");
        assert_eq!(pb.architecture, "arm");
        assert_eq!(pb.variant, "v7");
    }

    #[test]
    fn test_platform_clone_eq_hash() {
        use std::collections::HashSet;

        let p = Platform::LINUX_AMD64;
        let p2 = p.clone();
        assert_eq!(p, p2);

        let mut set = HashSet::new();
        set.insert(p.clone());
        assert!(set.contains(&p2));

        assert_ne!(Platform::LINUX_AMD64, Platform::LINUX_ARM64);
    }

    #[test]
    fn test_platform_new() {
        let p = Platform::new("linux", "riscv64", None);
        assert_eq!(p.os, "linux");
        assert_eq!(p.architecture, "riscv64");
        assert!(p.variant.is_none());

        let p = Platform::new("linux", "arm", Some("v5"));
        assert_eq!(p.variant.as_deref(), Some("v5"));
    }
}
