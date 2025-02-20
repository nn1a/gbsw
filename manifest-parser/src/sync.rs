use crate::{Manifest, Project};
use log::{debug, error};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use threadpool::ThreadPool;

/// Trait for running git commands, used for mocking in tests.
pub trait GitCommandRunner {
    fn run_git_command(
        &self,
        project_path: &Path,
        args: &[&str],
    ) -> Result<ExitStatus, Box<dyn Error>>;
}

/// Default implementation of GitCommandRunner.
pub struct DefaultGitCommandRunner;

impl GitCommandRunner for DefaultGitCommandRunner {
    fn run_git_command(
        &self,
        project_path: &Path,
        args: &[&str],
    ) -> Result<ExitStatus, Box<dyn Error>> {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(project_path);
        for arg in args {
            cmd.arg(arg);
        }
        let status = cmd.status()?;
        if !status.success() {
            return Err(
                std::io::Error::new(std::io::ErrorKind::Other, "Git command failed").into(),
            );
        }
        Ok(status)
    }
}

/// Syncs the repositories defined in the manifest.
///
/// # Arguments
///
/// * `manifest_path` - A string slice that holds the path to the manifest XML file.
/// * `project_list` - An optional list of project names to sync. If None, all projects are synced.
/// * `options` - A struct containing options for the sync operation.
/// * `target_dir` - A string slice that holds the path to the target directory where repositories will be cloned.
///
/// # Example
///
/// ```ignore
/// use manifest_parser::sync::{sync_repos, SyncOptions};
///
/// let options = SyncOptions {
///     current_branch_only: false,
///     detach: false,
///     force: false,
///     jobs: None,
///     quiet: false,
///     smart_sync: false,
///     keep: true,
/// };
/// sync_repos("path/to/manifest.xml", None, options, "path/to/target/dir").unwrap();
/// ```
pub fn sync_repos(
    manifest_path: &str,
    project_list: Option<Vec<&str>>,
    options: SyncOptions,
    target_dir: &str,
) -> Result<(), Box<dyn Error>> {
    debug!("sync_repos called with:");
    debug!("  manifest_path: {}", manifest_path);
    debug!("  project_list: {:#?}", project_list);
    debug!("  target_dir: {}", target_dir);
    debug!("  options: {:?}", options);

    let manifest = load_and_merge_manifests(manifest_path, None)?;

    let projects_to_sync: Vec<_> = match project_list {
        Some(list) => manifest
            .projects
            .clone()
            .into_iter()
            .filter(|p| list.contains(&p.name.as_str()))
            .collect(),
        None => manifest.projects.clone(), // Sync all projects if project_list is None
    };
    debug!("Projects to sync: {:#?}", projects_to_sync);

    let target_path = Path::new(target_dir);

    // Create the target directory if it does not exist
    if !target_path.exists() {
        fs::create_dir_all(target_path)?;
    }

    // Determine the number of jobs to use
    let jobs = determine_jobs(&manifest, &options);
    debug!("Number of jobs: {}", jobs);

    let errors = Arc::new(Mutex::new(Vec::new()));
    let pool = ThreadPool::new(jobs);
    let stop_flag = Arc::new(AtomicBool::new(false));

    for project in projects_to_sync.clone() {
        let stop_flag = Arc::clone(&stop_flag);
        if !options.keep && stop_flag.load(Ordering::Relaxed) {
            break;
        }
        let errors = Arc::clone(&errors);
        let manifest = manifest.clone();
        let target_path = target_path.to_path_buf();
        let options = options.clone();

        pool.execute(move || {
            if !options.keep && stop_flag.load(Ordering::Relaxed) {
                return;
            }
            if let Err(e) = process_project(&project, &manifest, &target_path, &options) {
                let mut errors = errors.lock().unwrap();
                errors.push((project.name.clone(), e.to_string()));
                stop_flag.store(true, Ordering::Relaxed);
            }
        });
    }

    pool.join();

    handle_errors(errors, options.keep)?;

    for project in projects_to_sync {
        debug!("Processing project: {:?}", project.name);
        let project_path_str = project.path.clone().unwrap_or_else(|| project.name.clone());
        let project_path = target_path.join(&project_path_str);
        for copyfile in project.copyfiles {
            handle_copyfiles_and_linkfiles(
                &project_path.join(&copyfile.src),
                &target_path.join(&copyfile.dest),
                target_path,
                false,
            )?;
        }
        for linkfile in project.linkfiles {
            handle_copyfiles_and_linkfiles(
                &project_path.join(&linkfile.src),
                &target_path.join(&linkfile.dest),
                target_path,
                true,
            )?;
        }
    }

    Ok(())
}

/// Handles the copying and linking of files as specified in the manifest.
///
/// # Arguments
///
/// * `src` - absolute path to the source file path, under git repository (project) directory.
/// * `dest` - absolute path to the dest file path., under projects root directory.
/// * `target_path` - absolute path to the target directory. projects root directory.
/// * `is_symlink` - boolean flag to indicate if the file should be symlinked.
fn handle_copyfiles_and_linkfiles(
    src: &Path,
    dest: &Path,
    target_path: &Path,
    is_symlink: bool,
) -> Result<(), Box<dyn Error>> {
    // Ensure src and dest do not go above target_path
    if !src.starts_with(target_path) || !dest.starts_with(target_path) {
        return Err("Source or destination path is outside the target directory".into());
    }

    // Validate that src exists and dest is not a directory
    if !src.exists() {
        return Err(format!("Source '{}' does not exist", src.display()).into());
    }

    if dest.exists() && dest.is_dir() {
        return Err(format!("Destination '{}' is a directory", dest.display()).into());
    }

    // Create parent directories of dest if missing
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if is_symlink {
        std::os::unix::fs::symlink(src, dest)?;
    } else {
        if !src.is_file() {
            return Err(format!("Source '{}' is not a file", src.display()).into());
        }

        if dest.exists() && !dest.is_file() {
            return Err(format!("Destination '{}' is not a file", dest.display()).into());
        }

        std::fs::copy(src, dest)?;
    }

    Ok(())
}

/// Loads and merges the main manifest and local manifests.
///
/// # Arguments
///
/// * `manifest_path` - A string slice that holds the path to the main manifest XML file.
/// * `local_manifests_dir` - An optional path to the directory containing local manifests.
///
/// # Returns
///
/// A merged `Manifest` struct.
pub fn load_and_merge_manifests(
    manifest_path: &str,
    local_manifests_dir: Option<&str>,
) -> Result<Manifest, Box<dyn Error>> {
    let default_remote = Some("origin");
    let default_revision = Some("main");

    let mut manifest = Manifest::from_file(manifest_path, default_remote, default_revision)?;

    // Determine the local manifests directory
    let local_manifests_dir = local_manifests_dir.map(PathBuf::from).unwrap_or_else(|| {
        let manifest_dir = Path::new(manifest_path).parent().unwrap();
        manifest_dir.join(".repo/local_manifests")
    });

    // Load and merge local manifests
    if local_manifests_dir.exists() {
        for entry in fs::read_dir(local_manifests_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("xml") {
                let local_manifest =
                    Manifest::from_file(path.to_str().unwrap(), default_remote, default_revision)?;
                merge_manifests(&mut manifest, local_manifest);
            }
        }
    }

    Ok(manifest)
}

fn merge_manifests(base: &mut Manifest, local: Manifest) {
    // Remove projects specified in remove_projects
    for remove_project in &local.remove_projects {
        debug!("Processing remove-project: {:?}", remove_project);
        base.projects.retain(|project| {
            let mut should_remove = false;
            if let Some(name) = &remove_project.name {
                if project.name == *name {
                    if let Some(path) = &remove_project.path {
                        should_remove = project.path.as_deref() == Some(path);
                    } else {
                        should_remove = true;
                    }
                }
            } else if let Some(path) = &remove_project.path {
                should_remove = project.path.as_deref() == Some(path);
            }

            if should_remove {
                if let Some(base_rev) = &remove_project.base_rev {
                    if project.revision.as_deref() != Some(base_rev) {
                        debug!(
                            "Revision mismatch for project '{}': expected '{}', found '{}'",
                            project.name,
                            base_rev,
                            project.revision.as_deref().unwrap_or("none")
                        );
                        return true;
                    }
                }
                debug!("Removing project: {:?}", project);
                return false;
            }
            true
        });

        if remove_project.optional.as_deref() == Some("true")
            && !base.projects.iter().any(|p| {
                if let Some(name) = &remove_project.name {
                    if p.name == *name {
                        if let Some(path) = &remove_project.path {
                            return p.path.as_deref() == Some(path);
                        }
                        return true;
                    }
                } else if let Some(path) = &remove_project.path {
                    return p.path.as_deref() == Some(path);
                }
                false
            })
        {
            debug!(
                "Optional remove-project element did not match any project: {:?}",
                remove_project
            );
        }
    }

    // Apply extend-project modifications
    for extend_project in &local.extend_projects {
        for project in &mut base.projects {
            if project.name == extend_project.name {
                if let Some(path) = &extend_project.path {
                    if project.path.as_deref() != Some(path) {
                        continue;
                    }
                }
                if let Some(dest_path) = &extend_project.dest_path {
                    project.path = Some(dest_path.clone());
                }
                if let Some(groups) = &extend_project.groups {
                    project.groups = Some(groups.clone());
                }
                if let Some(revision) = &extend_project.revision {
                    project.revision = Some(revision.clone());
                }
                if let Some(remote) = &extend_project.remote {
                    project.remote = Some(remote.clone());
                }
                if let Some(dest_branch) = &extend_project.dest_branch {
                    project.dest_branch = Some(dest_branch.clone());
                }
                if let Some(upstream) = &extend_project.upstream {
                    project.upstream = Some(upstream.clone());
                }
                if let Some(_base_rev) = &extend_project.base_rev {
                    // Add logic to handle base_rev if needed
                }
                debug!("Extended project: {:?}", project);
            }
        }
    }

    base.remotes.extend(local.remotes);
    base.default = local.default.or(base.default.take());
    base.manifest_server = local.manifest_server.or(base.manifest_server.take());
    base.submanifests.extend(local.submanifests);
    base.remove_projects.extend(local.remove_projects);
    base.projects.extend(local.projects);
    base.extend_projects.extend(local.extend_projects);
    base.repo_hooks = local.repo_hooks.or(base.repo_hooks.take());
    base.superproject = local.superproject.or(base.superproject.take());
    base.contactinfo = local.contactinfo.or(base.contactinfo.take());
    base.includes.extend(local.includes);
}

fn determine_jobs(manifest: &Manifest, options: &SyncOptions) -> usize {
    options
        .jobs
        .or_else(|| {
            manifest
                .default
                .as_ref()
                .and_then(|d| d.sync_j.as_ref().map(|s| s.parse::<usize>().unwrap_or(1)))
        })
        .unwrap_or(1)
        .clamp(1, 4)
}

fn process_project(
    project: &Project,
    manifest: &Manifest,
    target_path: &Path,
    options: &SyncOptions,
) -> Result<(), Box<dyn Error>> {
    debug!("Processing project: {:?}", project.name);

    let project_path_str = project.path.clone().unwrap_or_else(|| project.name.clone());
    let project_path = target_path.join(&project_path_str);

    // Find the corresponding remote fetch URL
    let remote_name = project
        .remote
        .clone()
        .or_else(|| manifest.default.as_ref().and_then(|d| d.remote.clone()))
        .unwrap_or_else(|| "origin".to_string());
    debug!("Searching for remote: {}", remote_name);

    let remote = manifest
        .remotes
        .iter()
        .find(|r| r.name == remote_name)
        .ok_or_else(|| {
            let error_message = format!("Remote '{}' not found in manifest", remote_name);
            error!("{}", error_message);
            error_message
        })?;
    let repo_url = format!("{}/{}.git", remote.fetch, project.name);

    debug!("Repo URL: {}", repo_url);

    // Determine the revision to use
    let revision = project
        .revision
        .clone()
        .or_else(|| manifest.default.as_ref().and_then(|d| d.revision.clone()))
        .ok_or_else(|| {
            if manifest.default.is_none() {
                "Default element is missing and project does not specify a revision".to_string()
            } else {
                "Default element does not specify a revision and project does not specify a revision".to_string()
            }
        })?;

    debug!("Revision: {}", revision);

    if project_path.exists() {
        debug!("Project path exists, fetching and rebasing...");
        fetch_and_rebase(&project_path, &revision, options)?;
    } else {
        debug!("Project path does not exist, cloning repository...");
        clone_repository(&project_path, &repo_url, &revision)?;
    }

    if options.detach {
        debug!("Detaching to revision: {}", revision);
        checkout_revision(&project_path, &revision)?;
    }

    Ok(())
}

fn fetch_and_rebase(
    project_path: &Path,
    revision: &str,
    _options: &SyncOptions,
) -> Result<(), Box<dyn Error>> {
    debug!(
        "Fetching and rebasing project at: {}",
        project_path.display()
    );
    debug!("Revision: {}", revision);

    // Fetch the latest changes with depth 1
    let fetch_args = vec!["fetch", "origin", "--prune", "--depth", "1", revision];

    debug!("Running git fetch with args: {:?}", fetch_args);
    if let Err(e) = run_git_command(project_path, &fetch_args) {
        error!("Failed to fetch: {}", e);
        return Err(e);
    }

    // Reset the repository to the fetched revision
    debug!("Resetting repository to fetched revision");
    if let Err(e) = run_git_command(project_path, &["reset", "--hard", "FETCH_HEAD"]) {
        error!("Failed to reset repository: {}", e);
        return Err(e);
    }

    Ok(())
}

fn clone_repository(
    project_path: &Path,
    repo_url: &str,
    revision: &str,
) -> Result<(), Box<dyn Error>> {
    debug!("Cloning repository from: {}", repo_url);
    debug!("Target path: {}", project_path.display());
    debug!("Revision: {}", revision);

    // Create the target directory if it does not exist
    if !project_path.exists() {
        debug!("Creating target directory: {}", project_path.display());
        if let Err(e) = fs::create_dir_all(project_path) {
            error!("Failed to create target directory: {}", e);
            return Err(e.into());
        }
    }

    // Initialize a new git repository
    debug!(
        "Initializing new git repository at: {}",
        project_path.display()
    );
    if let Err(e) = run_git_command(project_path, &["init"]) {
        error!("Failed to initialize git repository: {}", e);
        return Err(e);
    }

    // Add the remote origin
    debug!("Adding remote origin: {}", repo_url);
    if let Err(e) = run_git_command(project_path, &["remote", "add", "origin", repo_url]) {
        error!("Failed to add remote origin: {}", e);
        return Err(e);
    }

    // Fetch the specific revision with depth 1
    debug!("Fetching revision with depth 1: {}", revision);
    if let Err(e) = run_git_command(project_path, &["fetch", "--depth", "1", "origin", revision]) {
        error!("Failed to fetch revision: {}", e);
        return Err(e);
    }

    // Checkout the fetched revision
    debug!("Checking out revision: {}", revision);
    if let Err(e) = run_git_command(project_path, &["checkout", "FETCH_HEAD"]) {
        error!("Failed to checkout revision: {}", e);
        return Err(e);
    }

    Ok(())
}

fn checkout_revision(project_path: &Path, revision: &str) -> Result<(), Box<dyn Error>> {
    run_git_command(project_path, &["checkout", revision])
}

fn run_git_command(project_path: &Path, args: &[&str]) -> Result<(), Box<dyn Error>> {
    DefaultGitCommandRunner
        .run_git_command(project_path, args)
        .map(|_| ())
}

fn handle_errors(
    errors: Arc<Mutex<Vec<(String, String)>>>,
    keep: bool,
) -> Result<(), Box<dyn Error>> {
    let errors = errors.lock().unwrap();
    if !errors.is_empty() {
        for (project, error) in errors.iter() {
            error!("Error in project '{}': {}", project, error);
        }
        if !keep {
            return Err("Sync failed due to errors".into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SyncOptions {
    pub current_branch_only: bool,
    pub detach: bool,
    pub force: bool,
    pub jobs: Option<usize>,
    pub quiet: bool,
    pub smart_sync: bool,
    pub keep: bool,
}
