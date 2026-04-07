// Allow dead code for WIP/incomplete functionality (file ops, sourcemap, etc.)
#![allow(dead_code)]

mod ops;
mod platform;
mod serialize;
mod sourcemap;
pub mod state;
pub mod utils;

pub use ops::build::Build;
pub use ops::diff::Diff;
pub use ops::exec::Exec;
pub use ops::exec::mount::CacheSharingMode;
pub use ops::exec::mount::Mount;
pub use ops::file::FileActions;
pub use ops::file::chown::ChownOpt;
pub use ops::file::copy::Copy;
pub use ops::file::mkdir::Mkdir;
pub use ops::file::mkfile::MkFile;
pub use ops::file::rm::Rm;
pub use ops::file::symlink::Symlink;
pub use ops::merge::Merge;
pub use ops::metadata::OpMetadataBuilder;
pub use ops::output::{
    MultiBorrowedLastOutput, MultiBorrowedOutput, MultiOwnedLastOutput, MultiOwnedOutput,
    SingleBorrowedOutput, SingleOwnedOutput,
};
pub use ops::source::git::Git;
pub use ops::source::http::Http;
pub use ops::source::image::Image;
pub use ops::source::image::ResolveMode;
pub use ops::source::local::Local;
pub use platform::Platform;
pub use serialize::Definition;
