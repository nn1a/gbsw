use manifest_parser::sync::{load_and_merge_manifests, sync_repos, SyncOptions};
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_sync_repos() {
    // Test syncing repositories defined in the manifest
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_manifest.xml");
    let target_dir = dir.path().join("target");
    std::fs::create_dir(&target_dir).unwrap();
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <remote name="origin" fetch="https://github.com"/>
        <project name="nn1a/gbsw" path="nn1a/gbsw" remote="origin" revision="main"/>
    </manifest>
    "#
    )
    .unwrap();

    let options = SyncOptions {
        current_branch_only: false,
        detach: false,
        force: false,
        jobs: None,
        quiet: false,
        smart_sync: false,
        keep: false,
    };

    // Call sync_repos without mocking
    let result = sync_repos(
        file_path.to_str().unwrap(),
        None,
        options,
        target_dir.to_str().unwrap(),
    );

    // Check if the sync was successful
    assert!(result.is_ok());
    assert!(target_dir.join("nn1a").join("gbsw").exists());
}

#[test]
fn test_load_and_merge_manifests_with_remove_project() {
    // Test loading and merging manifests with a remove-project element
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test_manifest.xml");
    let local_manifest_path = dir.path().join(".repo/local_manifests/local_manifest.xml");
    std::fs::create_dir_all(local_manifest_path.parent().unwrap()).unwrap();

    let mut file = File::create(&file_path).unwrap();
    writeln!(
        file,
        r#"
    <manifest>
        <remote name="origin" fetch="https://github.com"/>
        <project name="nn1a/gbsw" path="nn1a/gbsw" remote="origin" revision="main"/>
        <project name="nn1a/another" path="nn1a/another" remote="origin" revision="main"/>
    </manifest>
    "#
    )
    .unwrap();

    let mut local_manifest_file = File::create(&local_manifest_path).unwrap();
    writeln!(
        local_manifest_file,
        r#"
    <manifest>
        <remove-project name="nn1a/another"/>
    </manifest>
    "#
    )
    .unwrap();

    let merged_manifest = load_and_merge_manifests(
        file_path.to_str().unwrap(),
        Some(local_manifest_path.parent().unwrap().to_str().unwrap()),
    )
    .unwrap();

    assert!(merged_manifest
        .projects
        .iter()
        .any(|p| p.name == "nn1a/gbsw"));
    assert!(!merged_manifest
        .projects
        .iter()
        .any(|p| p.name == "nn1a/another"));
}
