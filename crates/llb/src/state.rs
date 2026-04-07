//! High-level state-based API for constructing LLB operation graphs.
//!
//! This module provides a [`State`]-centric API that mirrors the Go BuildKit SDK's
//! `llb.State` patterns. Instead of constructing low-level operations and wiring
//! outputs manually, you work with `State` objects that represent filesystem states
//! and chain operations on them.
//!
//! # Examples
//!
//! ```no_run
//! use buildkit_rs_llb::state::*;
//!
//! // Simple hello world (equivalent to Go's llb.Image("alpine").Run(llb.Shlex("echo hello")).Root())
//! let st = image("alpine:latest")
//!     .run(shlex("echo 'hello world'"))
//!     .root();
//! write_to(&st.marshal(), &mut std::io::stdout());
//! ```

use std::io::Write;
use std::sync::{Arc, OnceLock};

use crate::ops::diff::Diff as DiffOp;
use crate::ops::exec::Exec;
use crate::ops::exec::mount::Mount;
use crate::ops::file::FileActions;
use crate::ops::file::copy::Copy;
use crate::ops::file::mkdir::Mkdir;
use crate::ops::file::mkfile::MkFile;
use crate::ops::file::rm::Rm;
use crate::ops::file::symlink::Symlink;
use crate::ops::merge::Merge as MergeOp;
use crate::ops::output::{MultiOwnedOutput, SingleOwnedOutput};
use crate::serialize::Definition;
use crate::utils::{OperationOutput, OutputIdx};

// Re-export types so examples can use them without extra imports.
pub use crate::ops::exec::mount::CacheSharingMode;
pub use crate::ops::metadata::OpMetadataBuilder;
pub use crate::ops::source::git::Git;
pub use crate::ops::source::http::Http;
pub use crate::ops::source::image::Image;
pub use crate::ops::source::local::Local;

// ============================================================
// State
// ============================================================

/// Represents a filesystem state at a point in a build graph.
///
/// This is the central type for constructing LLB operations, mirroring
/// Go BuildKit's `llb.State`. Operations are chained on states to build
/// a directed acyclic graph (DAG) of build steps.
///
/// `State` is cheaply cloneable (backed by `Arc`) and can be reused as
/// input to multiple operations.
#[derive(Clone, Default)]
pub struct State {
    output: Option<OperationOutput<'static>>,
    constraints: Constraints,
}

#[derive(Clone, Default)]
struct Constraints {
    env: Vec<String>,
    cwd: String,
}

impl State {
    /// Create an empty scratch state (no filesystem content).
    ///
    /// Equivalent to Go's `llb.Scratch()`.
    pub fn scratch() -> Self {
        Self::default()
    }

    fn from_output(output: OperationOutput<'static>) -> Self {
        Self {
            output: Some(output),
            constraints: Constraints::default(),
        }
    }

    /// Get the underlying [`OperationOutput`], if any.
    ///
    /// Returns `None` for scratch states.
    pub fn output(&self) -> Option<&OperationOutput<'static>> {
        self.output.as_ref()
    }

    /// Set the working directory for subsequent [`run()`](Self::run) calls.
    ///
    /// Equivalent to Go's `state.Dir(path)`.
    pub fn dir(mut self, path: impl Into<String>) -> Self {
        self.constraints.cwd = path.into();
        self
    }

    /// Add an environment variable (as `KEY=VALUE`) for subsequent
    /// [`run()`](Self::run) calls.
    ///
    /// Equivalent to Go's `state.AddEnv(key, value)`.
    pub fn add_env(mut self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.constraints
            .env
            .push(format!("{}={}", key.as_ref(), value.as_ref()));
        self
    }

    /// Run a command on this state's filesystem.
    ///
    /// Returns an [`ExecState`] builder that can be further configured with
    /// mounts. Call [`.root()`](ExecState::root) to get the resulting state.
    ///
    /// Equivalent to Go's `state.Run(opts...)`.
    pub fn run(self, option: RunOption) -> ExecState {
        ExecState {
            env: self.constraints.env.clone(),
            cwd: self.constraints.cwd.clone(),
            base: self,
            args: option.args,
            custom_name: None,
            extra_mounts: Vec::new(),
            built: OnceLock::new(),
        }
    }

    /// Apply a file operation to this state's filesystem.
    ///
    /// Equivalent to Go's `state.File(action)`.
    pub fn file(self, op: impl Into<FileOp>) -> Self {
        let op = op.into();
        let file_actions = op.build(self.output.clone());
        let arc: Arc<FileActions<'static>> = Arc::new(file_actions);
        let output = MultiOwnedOutput::output(&arc, 0);
        Self {
            output: Some(output),
            constraints: self.constraints,
        }
    }

    /// Apply a state transformation function.
    ///
    /// Equivalent to Go's `state.With(stateOption)`.
    pub fn with(self, f: impl FnOnce(State) -> State) -> Self {
        f(self)
    }

    /// Serialize this state to a protobuf Definition (as bytes).
    ///
    /// Equivalent to Go's `state.Marshal(ctx)`.
    pub fn marshal(&self) -> Vec<u8> {
        match self.output {
            Some(ref output) => Definition::new(output.clone()).into_bytes(),
            None => Vec::new(),
        }
    }
}

// From implementations for converting low-level source operations into State.

impl From<Image> for State {
    fn from(img: Image) -> Self {
        let arc = Arc::new(img);
        Self::from_output(SingleOwnedOutput::output(&arc))
    }
}

impl From<Git> for State {
    fn from(git: Git) -> Self {
        let arc = Arc::new(git);
        Self::from_output(SingleOwnedOutput::output(&arc))
    }
}

impl From<Http> for State {
    fn from(http: Http) -> Self {
        let arc = Arc::new(http);
        Self::from_output(SingleOwnedOutput::output(&arc))
    }
}

impl From<Local> for State {
    fn from(local: Local) -> Self {
        let arc = Arc::new(local);
        Self::from_output(SingleOwnedOutput::output(&arc))
    }
}

// ============================================================
// ExecState
// ============================================================

struct BuiltExec {
    exec: Arc<Exec<'static>>,
    mount_outputs: Vec<(String, u32)>,
    root_output: u32,
}

/// Represents a command execution step in the build graph.
///
/// Created by [`State::run()`]. Use [`.root()`](Self::root) to get the root
/// filesystem state after execution, or [`.get_mount()`](Self::get_mount) to
/// get the output of a specific mount point.
///
/// Additional mounts can be added with methods like
/// [`add_mount()`](Self::add_mount), [`add_mount_scratch()`](Self::add_mount_scratch),
/// and [`add_mount_cache()`](Self::add_mount_cache).
pub struct ExecState {
    base: State,
    args: Vec<String>,
    env: Vec<String>,
    cwd: String,
    custom_name: Option<String>,
    extra_mounts: Vec<ExtraMountSpec>,
    built: OnceLock<BuiltExec>,
}

struct ExtraMountSpec {
    dest: String,
    kind: ExtraMountKind,
}

enum ExtraMountKind {
    Readonly(State),
    Scratch,
    Cache {
        id: String,
        sharing: CacheSharingMode,
    },
}

impl ExecState {
    fn do_build(&self) -> BuiltExec {
        let mut mounts: Vec<Mount<'static>> = Vec::new();
        let mut mount_outputs: Vec<(String, u32)> = Vec::new();
        let mut next_output: u32 = 0;

        // Root mount at "/"
        let root_output = next_output;
        if let Some(ref out) = self.base.output {
            mounts.push(Mount::layer(out.clone(), "/", root_output));
        } else {
            mounts.push(Mount::scratch("/", root_output));
        }
        next_output += 1;

        // Extra mounts
        for m in &self.extra_mounts {
            match &m.kind {
                ExtraMountKind::Readonly(state) => {
                    if let Some(ref out) = state.output {
                        mounts.push(Mount::layer_readonly(out.clone(), &*m.dest));
                    }
                }
                ExtraMountKind::Scratch => {
                    let idx = next_output;
                    mounts.push(Mount::scratch(&*m.dest, idx));
                    mount_outputs.push((m.dest.clone(), idx));
                    next_output += 1;
                }
                ExtraMountKind::Cache { id, sharing } => {
                    mounts.push(Mount::cache(&*m.dest, id, *sharing));
                }
            }
        }

        // Build the Exec operation
        let mut exec = Exec::new(self.args.iter().map(|s| s.as_str()));
        for mount in mounts {
            exec = exec.with_mount(mount);
        }
        if !self.env.is_empty() {
            exec = exec.with_env(self.env.clone());
        }
        if !self.cwd.is_empty() {
            exec = exec.with_cwd(self.cwd.clone());
        }
        if let Some(ref name) = self.custom_name {
            exec = exec.with_custom_name(name);
        }

        BuiltExec {
            exec: Arc::new(exec),
            mount_outputs,
            root_output,
        }
    }

    fn get_built(&self) -> &BuiltExec {
        self.built.get_or_init(|| self.do_build())
    }

    /// Get the root filesystem state after running the command.
    ///
    /// Equivalent to Go's `execState.Root()`.
    pub fn root(&self) -> State {
        let built = self.get_built();
        State {
            output: Some(OperationOutput::owned(
                built.exec.clone(),
                OutputIdx(built.root_output),
            )),
            constraints: Constraints {
                env: self.env.clone(),
                cwd: self.cwd.clone(),
            },
        }
    }

    /// Get the output state of a specific mount point after execution.
    ///
    /// Only works for writable mounts (scratch mounts added via
    /// [`add_mount_scratch()`](Self::add_mount_scratch)).
    ///
    /// Equivalent to Go's `execState.AddMount(path, llb.Scratch())` (return value).
    pub fn get_mount(&self, dest: &str) -> State {
        let built = self.get_built();
        let idx = built
            .mount_outputs
            .iter()
            .find(|(d, _)| d == dest)
            .map(|(_, idx)| *idx)
            .unwrap_or_else(|| panic!("no writable mount at '{dest}'"));
        State::from_output(OperationOutput::owned(built.exec.clone(), OutputIdx(idx)))
    }

    /// Add a read-only mount from another state.
    ///
    /// Equivalent to Go's `execState.AddMount(path, src, llb.Readonly)`.
    pub fn add_mount(mut self, dest: impl Into<String>, src: State) -> Self {
        self.extra_mounts.push(ExtraMountSpec {
            dest: dest.into(),
            kind: ExtraMountKind::Readonly(src),
        });
        // Safety: `mut self` guarantees exclusive ownership, so no aliased
        // references to the old OnceLock value can exist.
        self.built = OnceLock::new();
        self
    }

    /// Add a scratch (empty) writable mount.
    ///
    /// Use [`.get_mount()`](Self::get_mount) to retrieve the output of
    /// this mount after execution.
    ///
    /// Equivalent to Go's `execState.AddMount(path, llb.Scratch())`.
    pub fn add_mount_scratch(mut self, dest: impl Into<String>) -> Self {
        self.extra_mounts.push(ExtraMountSpec {
            dest: dest.into(),
            kind: ExtraMountKind::Scratch,
        });
        // Safety: `mut self` guarantees exclusive ownership.
        self.built = OnceLock::new();
        self
    }

    /// Add a cache mount for persisting data between builds.
    pub fn add_mount_cache(
        mut self,
        dest: impl Into<String>,
        id: impl Into<String>,
        sharing: CacheSharingMode,
    ) -> Self {
        self.extra_mounts.push(ExtraMountSpec {
            dest: dest.into(),
            kind: ExtraMountKind::Cache {
                id: id.into(),
                sharing,
            },
        });
        // Safety: `mut self` guarantees exclusive ownership.
        self.built = OnceLock::new();
        self
    }

    /// Set a custom display name for this exec operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        // Safety: `mut self` guarantees exclusive ownership.
        self.built = OnceLock::new();
        self
    }

    /// Chain another run command (implicitly calls [`.root()`](Self::root) first).
    pub fn run(self, option: RunOption) -> ExecState {
        self.root().run(option)
    }

    /// Set working directory (implicitly calls [`.root()`](Self::root) first).
    pub fn dir(self, path: impl Into<String>) -> State {
        self.root().dir(path)
    }

    /// Serialize to bytes (implicitly calls [`.root()`](Self::root) first).
    pub fn marshal(&self) -> Vec<u8> {
        self.root().marshal()
    }
}

// ============================================================
// RunOption
// ============================================================

/// Specifies the command to run in an exec operation.
///
/// Created via the convenience function [`shlex()`] or the associated methods.
pub struct RunOption {
    args: Vec<String>,
}

impl RunOption {
    /// Parse a command string using shell-like syntax.
    ///
    /// Equivalent to Go's `llb.Shlex(cmd)`.
    pub fn shlex(cmd: impl AsRef<str>) -> Self {
        let args = shlex::Shlex::new(cmd.as_ref()).collect();
        Self { args }
    }

    /// Run a command through a shell (e.g., `/bin/sh -c "cmd"`).
    pub fn shell(shell: impl AsRef<str>, cmd: impl AsRef<str>) -> Self {
        Self {
            args: vec![shell.as_ref().into(), "-c".into(), cmd.as_ref().into()],
        }
    }

    /// Create a run option from an explicit argument list.
    pub fn args<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            args: args.into_iter().map(|s| s.into()).collect(),
        }
    }
}

// ============================================================
// File Operations
// ============================================================

/// A file operation to apply to a [`State`] via [`State::file()`].
pub enum FileOp {
    Copy(CopyOp),
    Mkdir(MkdirOp),
    MkFile(MkFileOp),
    Rm(RmOp),
    Symlink(SymlinkOp),
}

impl FileOp {
    fn build(self, dest_output: Option<OperationOutput<'static>>) -> FileActions<'static> {
        let dest = dest_output.expect("file operation requires a non-scratch destination state");
        match self {
            FileOp::Copy(c) => {
                let mut cp = Copy::new(c.src_path, c.src, c.dest_path, dest);
                if c.create_dest_path {
                    cp = cp.with_create_dest_path(true);
                }
                if c.allow_wildcard {
                    cp = cp.with_allow_wildcard(true);
                }
                let mut fa = FileActions::new().with_action(cp);
                if let Some(name) = c.custom_name {
                    fa = fa.with_custom_name(name);
                }
                fa
            }
            FileOp::Mkdir(m) => {
                let mut md = Mkdir::new(m.path, dest).with_mode(m.mode);
                if m.make_parents {
                    md = md.with_make_parents(true);
                }
                let mut fa = FileActions::new().with_action(md);
                if let Some(name) = m.custom_name {
                    fa = fa.with_custom_name(name);
                }
                fa
            }
            FileOp::MkFile(m) => {
                let mf = MkFile::new(m.path, dest, m.data).with_mode(m.mode);
                let mut fa = FileActions::new().with_action(mf);
                if let Some(name) = m.custom_name {
                    fa = fa.with_custom_name(name);
                }
                fa
            }
            FileOp::Rm(r) => {
                let rm = Rm::new(r.path, dest);
                let mut fa = FileActions::new().with_action(rm);
                if let Some(name) = r.custom_name {
                    fa = fa.with_custom_name(name);
                }
                fa
            }
            FileOp::Symlink(s) => {
                let sl = Symlink::new(s.oldpath, s.newpath, dest);
                let mut fa = FileActions::new().with_action(sl);
                if let Some(name) = s.custom_name {
                    fa = fa.with_custom_name(name);
                }
                fa
            }
        }
    }
}

/// A copy file operation builder. Created by [`copy()`].
pub struct CopyOp {
    src: OperationOutput<'static>,
    src_path: String,
    dest_path: String,
    create_dest_path: bool,
    allow_wildcard: bool,
    custom_name: Option<String>,
}

impl CopyOp {
    /// Create the destination path if it doesn't exist.
    pub fn with_create_dest_path(mut self, create: bool) -> Self {
        self.create_dest_path = create;
        self
    }

    /// Allow wildcard patterns in the source path.
    pub fn with_allow_wildcard(mut self, allow: bool) -> Self {
        self.allow_wildcard = allow;
        self
    }

    /// Set a custom display name for this operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        self
    }
}

impl From<CopyOp> for FileOp {
    fn from(c: CopyOp) -> Self {
        FileOp::Copy(c)
    }
}

/// A mkdir file operation builder. Created by [`mkdir()`].
pub struct MkdirOp {
    path: String,
    mode: i32,
    make_parents: bool,
    custom_name: Option<String>,
}

impl MkdirOp {
    /// Create parent directories as needed.
    pub fn with_make_parents(mut self, make: bool) -> Self {
        self.make_parents = make;
        self
    }

    /// Set a custom display name for this operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        self
    }
}

impl From<MkdirOp> for FileOp {
    fn from(m: MkdirOp) -> Self {
        FileOp::Mkdir(m)
    }
}

/// A mkfile file operation builder. Created by [`mkfile()`].
pub struct MkFileOp {
    path: String,
    mode: i32,
    data: Vec<u8>,
    custom_name: Option<String>,
}

impl MkFileOp {
    /// Set a custom display name for this operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        self
    }
}

impl From<MkFileOp> for FileOp {
    fn from(m: MkFileOp) -> Self {
        FileOp::MkFile(m)
    }
}

/// A remove file operation builder. Created by [`rm()`].
pub struct RmOp {
    path: String,
    custom_name: Option<String>,
}

impl RmOp {
    /// Set a custom display name for this operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        self
    }
}

impl From<RmOp> for FileOp {
    fn from(r: RmOp) -> Self {
        FileOp::Rm(r)
    }
}

/// A symlink file operation builder. Created by [`symlink()`].
pub struct SymlinkOp {
    oldpath: String,
    newpath: String,
    custom_name: Option<String>,
}

impl SymlinkOp {
    /// Set a custom display name for this operation.
    pub fn with_custom_name(mut self, name: impl AsRef<str>) -> Self {
        self.custom_name = Some(name.as_ref().to_string());
        self
    }
}

impl From<SymlinkOp> for FileOp {
    fn from(s: SymlinkOp) -> Self {
        FileOp::Symlink(s)
    }
}

// ============================================================
// Convenience functions
// ============================================================

/// Create a state from a container image.
///
/// Equivalent to Go's `llb.Image(name)`.
///
/// For advanced options (resolve mode, platform), use [`Image`] directly
/// and convert with `State::from(image)`.
pub fn image(name: impl AsRef<str>) -> State {
    State::from(Image::new(name))
}

/// Create an empty scratch state.
///
/// Equivalent to Go's `llb.Scratch()`.
pub fn scratch() -> State {
    State::scratch()
}

/// Create a state from a Git repository.
///
/// Equivalent to Go's `llb.Git(remote, ref)`.
///
/// For advanced options (keep git dir, auth), use [`Git`] directly
/// and convert with `State::from(git)`.
pub fn git(remote: impl Into<String>, git_ref: impl Into<String>) -> State {
    State::from(Git::new(remote, git_ref))
}

/// Create a state from an HTTP URL.
///
/// Equivalent to Go's `llb.HTTP(url)`.
///
/// For advanced options (filename, checksum), use [`Http`] directly
/// and convert with `State::from(http)`.
pub fn http(url: impl Into<String>) -> State {
    State::from(Http::new(url))
}

/// Create a state from a local build context.
///
/// Equivalent to Go's `llb.Local(name)`.
pub fn local(name: impl Into<String>) -> State {
    State::from(Local::new(name.into()))
}

/// Parse a shell command string into a [`RunOption`].
///
/// Equivalent to Go's `llb.Shlex(cmd)`.
pub fn shlex(cmd: impl AsRef<str>) -> RunOption {
    RunOption::shlex(cmd)
}

/// Merge multiple states into one by overlaying their filesystems.
///
/// Equivalent to Go's `llb.Merge(states)`.
pub fn merge(states: Vec<State>) -> State {
    let inputs: Vec<_> = states.into_iter().filter_map(|s| s.output).collect();
    let m = Arc::new(MergeOp::new(inputs));
    State::from_output(SingleOwnedOutput::output(&m))
}

/// Compute the diff between two states.
///
/// Returns a state containing only the changes (additions, modifications,
/// deletions) between `lower` (base) and `upper` (changed).
///
/// Equivalent to Go's `llb.Diff(lower, upper)`.
pub fn diff(lower: &State, upper: &State) -> State {
    let d = Arc::new(DiffOp::new(lower.output.clone(), upper.output.clone()));
    State::from_output(SingleOwnedOutput::output(&d))
}

/// Create a copy file operation.
///
/// Copies files from `src` state at `src_path` to the destination state
/// at `dest_path`. The destination state is provided by [`State::file()`].
///
/// Equivalent to Go's `llb.Copy(src, srcPath, destPath)`.
pub fn copy(src: &State, src_path: impl Into<String>, dest_path: impl Into<String>) -> CopyOp {
    CopyOp {
        src: src.output.clone().expect("copy source must not be scratch"),
        src_path: src_path.into(),
        dest_path: dest_path.into(),
        create_dest_path: false,
        allow_wildcard: false,
        custom_name: None,
    }
}

/// Create a mkdir file operation.
///
/// Equivalent to Go's `llb.Mkdir(path, mode)`.
pub fn mkdir(path: impl Into<String>, mode: i32) -> MkdirOp {
    MkdirOp {
        path: path.into(),
        mode,
        make_parents: false,
        custom_name: None,
    }
}

/// Create a mkfile file operation.
///
/// Equivalent to Go's `llb.Mkfile(path, mode, data)`.
pub fn mkfile(path: impl Into<String>, mode: i32, data: impl Into<Vec<u8>>) -> MkFileOp {
    MkFileOp {
        path: path.into(),
        mode,
        data: data.into(),
        custom_name: None,
    }
}

/// Create a remove file operation.
///
/// Equivalent to Go's `llb.Rm(path)`.
pub fn rm(path: impl Into<String>) -> RmOp {
    RmOp {
        path: path.into(),
        custom_name: None,
    }
}

/// Create a symlink file operation.
///
/// Equivalent to Go's `llb.Symlink(oldpath, newpath)`.
pub fn symlink(oldpath: impl Into<String>, newpath: impl Into<String>) -> SymlinkOp {
    SymlinkOp {
        oldpath: oldpath.into(),
        newpath: newpath.into(),
        custom_name: None,
    }
}

/// Write bytes to a writer.
///
/// Convenience wrapper for writing serialized LLB definitions to stdout
/// or other writers.
///
/// Equivalent to Go's `llb.WriteTo(def, os.Stdout)`.
///
/// # Panics
///
/// Panics if writing to the writer fails.
pub fn write_to(data: &[u8], w: &mut impl Write) {
    w.write_all(data).expect("failed to write LLB definition");
}
