use anyhow::{Context, Result, bail};
use semver::Version;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const BUILDKIT_LATEST_RELEASE_URL: &str =
    "https://api.github.com/repos/moby/buildkit/releases/latest";
const BUILDKIT_RAW_BASE_URL: &str = "https://raw.githubusercontent.com/moby/buildkit";
const PLANETSCALE_VTPROTO_MODULE: &str = "github.com/planetscale/vtprotobuf";
const USER_AGENT: &str = "buildkit-sdk-rs-xtask";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandStatus {
    Success,
    Outdated,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

fn main() -> ExitCode {
    match try_main() {
        Ok(CommandStatus::Success) => ExitCode::SUCCESS,
        Ok(CommandStatus::Outdated) => ExitCode::from(1),
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}

fn try_main() -> Result<CommandStatus> {
    let repo_root = workspace_root();
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        bail!("missing subcommand");
    };

    match command.as_str() {
        "check-protos" => {
            ensure_no_extra_args(args)?;
            check_protos(&repo_root)
        }
        "update-protos" => {
            let requested_version = parse_update_version_flag(args)?;
            update_protos(&repo_root, requested_version.as_deref())?;
            Ok(CommandStatus::Success)
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(CommandStatus::Success)
        }
        other => {
            print_usage();
            bail!("unknown subcommand `{other}`");
        }
    }
}

fn ensure_no_extra_args(args: impl Iterator<Item = String>) -> Result<()> {
    let extras = args.collect::<Vec<_>>();
    if extras.is_empty() {
        return Ok(());
    }

    bail!("unexpected arguments: {}", extras.join(" "));
}

fn parse_update_version_flag(mut args: impl Iterator<Item = String>) -> Result<Option<String>> {
    let Some(flag) = args.next() else {
        return Ok(None);
    };

    if flag != "--version" {
        bail!("expected `--version <tag>`, got `{flag}`");
    }

    let version = args.next().context("missing value for `--version`")?;
    ensure_no_extra_args(args)?;
    Ok(Some(version))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate should live in the workspace root")
        .to_path_buf()
}

fn proto_dir(repo_root: &Path) -> PathBuf {
    repo_root.join("crates").join("proto")
}

fn version_file(repo_root: &Path) -> PathBuf {
    proto_dir(repo_root).join("BUILDKIT_VERSION")
}

fn vendor_dir(repo_root: &Path) -> PathBuf {
    proto_dir(repo_root).join("vendor")
}

fn github_vendor_dir(repo_root: &Path) -> PathBuf {
    vendor_dir(repo_root).join("github.com")
}

fn buildkit_vendor_dir(repo_root: &Path) -> PathBuf {
    github_vendor_dir(repo_root).join("moby").join("buildkit")
}

fn read_pinned_version(repo_root: &Path) -> Result<String> {
    let version = fs::read_to_string(version_file(repo_root))
        .with_context(|| format!("failed to read {}", version_file(repo_root).display()))?;
    normalize_version(version.trim())
}

fn write_pinned_version(repo_root: &Path, version: &str) -> Result<()> {
    fs::write(version_file(repo_root), format!("{version}\n"))
        .with_context(|| format!("failed to write {}", version_file(repo_root).display()))
}

fn normalize_version(version: &str) -> Result<String> {
    let normalized = version.trim().trim_start_matches('v');
    let parsed = Version::parse(normalized)
        .with_context(|| format!("invalid BuildKit version tag `{version}`"))?;
    Ok(format!("v{parsed}"))
}

fn parse_semver_tag(version: &str) -> Result<Version> {
    let normalized = normalize_version(version)?;
    let raw = normalized
        .strip_prefix('v')
        .expect("normalized version should start with v");
    Version::parse(raw).with_context(|| format!("invalid BuildKit version tag `{version}`"))
}

fn fetch_latest_buildkit_release_tag() -> Result<String> {
    let response = github_request(BUILDKIT_LATEST_RELEASE_URL)
        .call()
        .context("failed to fetch the latest moby/buildkit release")?;
    let release: GitHubRelease = response
        .into_json()
        .context("failed to decode the latest moby/buildkit release response")?;
    normalize_version(&release.tag_name)
}

fn github_request(url: &str) -> ureq::Request {
    let mut request = ureq::get(url)
        .set("Accept", "application/vnd.github+json")
        .set("User-Agent", USER_AGENT);

    if let Some(token) = github_token() {
        request = request.set("Authorization", &format!("Bearer {token}"));
    }

    request
}

fn github_token() -> Option<String> {
    env::var("GITHUB_TOKEN")
        .ok()
        .or_else(|| env::var("GH_TOKEN").ok())
}

fn check_protos(repo_root: &Path) -> Result<CommandStatus> {
    let pinned_version = read_pinned_version(repo_root)?;
    let latest_version = fetch_latest_buildkit_release_tag()?;
    let update_available = parse_semver_tag(&latest_version)? > parse_semver_tag(&pinned_version)?;

    eprintln!("Pinned BuildKit version: {pinned_version}");
    eprintln!("Latest upstream BuildKit release: {latest_version}");

    // Stdout is reserved for machine-readable output so GitHub Actions can
    // append it directly to GITHUB_OUTPUT.
    println!("pinned_version={pinned_version}");
    println!("latest_version={latest_version}");
    println!("update_available={update_available}");

    if update_available {
        return Ok(CommandStatus::Outdated);
    }

    Ok(CommandStatus::Success)
}

fn update_protos(repo_root: &Path, requested_version: Option<&str>) -> Result<()> {
    let current_version = read_pinned_version(repo_root)?;
    let target_version = match requested_version {
        Some(version) => normalize_version(version)?,
        None => current_version.clone(),
    };
    let buildkit_module_versions = fetch_buildkit_module_versions(&target_version)?;

    eprintln!("Vendoring BuildKit protos for {target_version}");

    update_buildkit_files(repo_root, &target_version)?;

    for repo in list_github_vendor_repos(repo_root)? {
        eprintln!("Vendoring github.com/{repo} from BuildKit {target_version}");
        update_other_repo_files(repo_root, &target_version, &repo)?;
    }

    update_planetscale_vtproto_file(repo_root, &buildkit_module_versions)?;
    write_pinned_version(repo_root, &target_version)?;

    if target_version == current_version {
        eprintln!("Pinned BuildKit version remains {target_version}");
    } else {
        eprintln!("Updated pinned BuildKit version {current_version} -> {target_version}");
    }

    Ok(())
}

fn list_github_vendor_repos(repo_root: &Path) -> Result<Vec<String>> {
    let mut repos = Vec::new();

    for org_dir in sorted_dir_entries(&github_vendor_dir(repo_root))? {
        if !org_dir.is_dir() {
            continue;
        }

        for repo_dir in sorted_dir_entries(&org_dir)? {
            if !repo_dir.is_dir() {
                continue;
            }

            let org_name = org_dir
                .file_name()
                .and_then(|name| name.to_str())
                .context("invalid organization directory name")?;
            let repo_name = repo_dir
                .file_name()
                .and_then(|name| name.to_str())
                .context("invalid repository directory name")?;
            let repo = format!("{org_name}/{repo_name}");

            if repo != "moby/buildkit" && repo != "planetscale/vtprotobuf" {
                repos.push(repo);
            }
        }
    }

    repos.sort();
    Ok(repos)
}

fn sorted_dir_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("failed to read directory {}", dir.display()))?;
    entries.sort();
    Ok(entries)
}

fn update_buildkit_files(repo_root: &Path, version: &str) -> Result<()> {
    let root = buildkit_vendor_dir(repo_root);
    update_files_under(&root, &mut |path| {
        let relative_path = path
            .strip_prefix(&root)
            .with_context(|| format!("failed to relativize {}", path.display()))?;
        let url = format!(
            "{BUILDKIT_RAW_BASE_URL}/{version}/{}",
            path_to_url_path(relative_path)
        );
        download_to_path(path, &url)
    })
}

fn update_other_repo_files(repo_root: &Path, version: &str, repo: &str) -> Result<()> {
    let root = github_vendor_dir(repo_root).join(repo);
    update_files_under(&root, &mut |path| {
        let relative_path = path
            .strip_prefix(&root)
            .with_context(|| format!("failed to relativize {}", path.display()))?;
        let url = format!(
            "{BUILDKIT_RAW_BASE_URL}/{version}/vendor/github.com/{repo}/{}",
            path_to_url_path(relative_path)
        );
        download_to_path(path, &url)
    })
}

fn update_files_under<F>(dir: &Path, visit_file: &mut F) -> Result<()>
where
    F: FnMut(&Path) -> Result<()>,
{
    for path in sorted_dir_entries(dir)? {
        if path.is_dir() {
            update_files_under(&path, visit_file)?;
        } else if path.is_file() {
            visit_file(&path)?;
        }
    }

    Ok(())
}

fn download_to_path(path: &Path, url: &str) -> Result<()> {
    eprintln!("Updating {}", path.display());
    let body = github_request(url)
        .call()
        .with_context(|| format!("failed to download {url}"))?
        .into_string()
        .with_context(|| format!("failed to read {url}"))?;
    fs::write(path, body).with_context(|| format!("failed to write {}", path.display()))
}

fn path_to_url_path(path: &Path) -> String {
    path.iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn fetch_buildkit_module_versions(version: &str) -> Result<HashMap<String, String>> {
    let url = format!("{BUILDKIT_RAW_BASE_URL}/{version}/vendor/modules.txt");
    let body = github_request(&url)
        .call()
        .with_context(|| format!("failed to download {url}"))?
        .into_string()
        .with_context(|| format!("failed to read {url}"))?;

    let mut modules = HashMap::new();

    for line in body.lines() {
        let Some(module_entry) = line.strip_prefix("# ") else {
            continue;
        };
        let mut parts = module_entry.split_whitespace();
        let Some(module_name) = parts.next() else {
            continue;
        };
        let Some(module_version) = parts.next() else {
            continue;
        };

        if module_version.starts_with('v') {
            modules.insert(module_name.to_owned(), module_version.to_owned());
        }
    }

    Ok(modules)
}

fn update_planetscale_vtproto_file(
    repo_root: &Path,
    buildkit_module_versions: &HashMap<String, String>,
) -> Result<()> {
    let module_version = buildkit_module_versions
        .get(PLANETSCALE_VTPROTO_MODULE)
        .with_context(|| {
            format!("missing {PLANETSCALE_VTPROTO_MODULE} in BuildKit vendor/modules.txt")
        })?;
    let git_ref = go_module_version_git_ref(module_version);
    let path = github_vendor_dir(repo_root)
        .join("planetscale")
        .join("vtprotobuf")
        .join("vtproto")
        .join("ext.proto");
    let url = format!(
        "https://raw.githubusercontent.com/planetscale/vtprotobuf/{git_ref}/include/github.com/planetscale/vtprotobuf/vtproto/ext.proto"
    );

    eprintln!("Vendoring github.com/planetscale/vtprotobuf from upstream ref {git_ref}");
    download_to_path(&path, &url)
}

fn go_module_version_git_ref(version: &str) -> &str {
    version
        .rsplit_once('-')
        .filter(|(_, suffix)| {
            suffix.len() >= 12 && suffix.chars().all(|char| char.is_ascii_hexdigit())
        })
        .map(|(_, suffix)| suffix)
        .unwrap_or(version)
}

fn print_usage() {
    eprintln!(
        "Usage:
  cargo xtask check-protos
  cargo xtask update-protos [--version vX.Y.Z]"
    );
}

#[cfg(test)]
mod tests {
    use super::{go_module_version_git_ref, normalize_version, parse_semver_tag, path_to_url_path};
    use std::path::Path;

    #[test]
    fn normalize_version_accepts_missing_prefix() {
        assert_eq!(normalize_version("0.29.0").unwrap(), "v0.29.0");
        assert_eq!(normalize_version("v0.29.0").unwrap(), "v0.29.0");
    }

    #[test]
    fn normalize_version_rejects_invalid_tags() {
        assert!(normalize_version("latest").is_err());
    }

    #[test]
    fn parse_semver_tag_compares_versions() {
        assert!(parse_semver_tag("v0.30.0").unwrap() > parse_semver_tag("v0.29.0").unwrap());
    }

    #[test]
    fn path_to_url_path_uses_forward_slashes() {
        assert_eq!(
            path_to_url_path(Path::new("vendor/github.com/moby/buildkit")),
            "vendor/github.com/moby/buildkit"
        );
    }

    #[test]
    fn go_module_pseudo_versions_use_commit_refs() {
        assert_eq!(
            go_module_version_git_ref("v0.6.1-0.20240319094008-0393e58bdf10"),
            "0393e58bdf10"
        );
        assert_eq!(go_module_version_git_ref("v0.29.0"), "v0.29.0");
    }
}
