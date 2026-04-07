use buildkit_rs_proto::pb;
use camino::Utf8PathBuf;

use crate::utils::OperationOutput;

use super::FileAction;
use super::chown::ChownOpt;

/// Copy files from one input to another.
#[derive(Debug)]
pub struct Copy<'a> {
    pub(crate) src_path: Utf8PathBuf,
    pub(crate) src_input: OperationOutput<'a>,
    pub(crate) dst_path: Utf8PathBuf,
    pub(crate) dest_input: OperationOutput<'a>,
    owner: Option<ChownOpt>,
    mode: i32,
    follow_symlink: bool,
    dir_copy_contents: bool,
    attempt_unpack_docker_compatibility: bool,
    create_dest_path: bool,
    allow_wildcard: bool,
    allow_empty_wildcard: bool,
    timestamp: i64,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
    always_replace_existing_dest_paths: bool,
}

impl<'a> Copy<'a> {
    pub fn new(
        src_path: impl Into<Utf8PathBuf>,
        src_input: OperationOutput<'a>,
        dst_path: impl Into<Utf8PathBuf>,
        dest_input: OperationOutput<'a>,
    ) -> Self {
        Self {
            src_path: src_path.into(),
            src_input,
            dst_path: dst_path.into(),
            dest_input,
            owner: None,
            mode: -1,
            follow_symlink: false,
            dir_copy_contents: false,
            attempt_unpack_docker_compatibility: false,
            create_dest_path: false,
            allow_wildcard: false,
            allow_empty_wildcard: false,
            timestamp: -1,
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            always_replace_existing_dest_paths: false,
        }
    }

    pub fn with_owner(mut self, owner: ChownOpt) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_mode(mut self, mode: i32) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_follow_symlink(mut self, follow: bool) -> Self {
        self.follow_symlink = follow;
        self
    }

    pub fn with_dir_copy_contents(mut self, dir_copy: bool) -> Self {
        self.dir_copy_contents = dir_copy;
        self
    }

    pub fn with_create_dest_path(mut self, create: bool) -> Self {
        self.create_dest_path = create;
        self
    }

    pub fn with_allow_wildcard(mut self, allow: bool) -> Self {
        self.allow_wildcard = allow;
        self
    }

    pub fn with_allow_empty_wildcard(mut self, allow: bool) -> Self {
        self.allow_empty_wildcard = allow;
        self
    }

    pub fn with_timestamp(mut self, timestamp: i64) -> Self {
        self.timestamp = timestamp;
        self
    }

    pub fn with_include_patterns(mut self, patterns: Vec<String>) -> Self {
        self.include_patterns = patterns;
        self
    }

    pub fn with_exclude_patterns(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    pub fn with_always_replace_existing_dest_paths(mut self, replace: bool) -> Self {
        self.always_replace_existing_dest_paths = replace;
        self
    }

    pub(crate) fn to_pb(&self) -> pb::FileActionCopy {
        pb::FileActionCopy {
            src: self.src_path.to_string(),
            dest: self.dst_path.to_string(),
            owner: self.owner.as_ref().map(|o| o.to_pb()),
            mode: self.mode,
            follow_symlink: self.follow_symlink,
            dir_copy_contents: self.dir_copy_contents,
            attempt_unpack_docker_compatibility: self.attempt_unpack_docker_compatibility,
            create_dest_path: self.create_dest_path,
            allow_wildcard: self.allow_wildcard,
            allow_empty_wildcard: self.allow_empty_wildcard,
            timestamp: self.timestamp,
            include_patterns: self.include_patterns.clone(),
            exclude_patterns: self.exclude_patterns.clone(),
            always_replace_existing_dest_paths: self.always_replace_existing_dest_paths,
            mode_str: String::new(),
            required_paths: Vec::new(),
        }
    }
}

impl<'a> From<Copy<'a>> for FileAction<'a> {
    fn from(copy: Copy<'a>) -> Self {
        Self::Copy(copy)
    }
}
