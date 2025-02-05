use manifest_parser::Manifest;
use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_parse_valid_manifest() {
    // Test parsing a valid manifest with various elements
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("valid_manifest.xml");
    let included_file_path = dir.path().join("included_manifest.xml");
    let mut file = File::create(&file_path).unwrap();
    let mut included_file = File::create(&included_file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <notice>This is a notice</notice>
        <remote name="origin" fetch="https://example.com/repo.git"/>
        <default remote="origin" revision="main"/>
        <manifest-server url="https://example.com/manifest"/>
        <submanifest name="sub1" remote="origin" project="subproject"/>
        <project name="project1" path="path/to/project1" remote="origin" revision="main"/>
        <extend-project name="project1" path="path/to/project1" revision="develop"/>
        <remove-project name="project2"/>
        <repo-hooks in-project="hooks" enabled-list="pre-upload"/>
        <superproject name="super" remote="origin" revision="main"/>
        <contactinfo bugurl="https://example.com/bugs"/>
        <include name="included_manifest.xml"/>
    </manifest>
    "#
    )
    .unwrap();

    writeln!(included_file, r#"
    <manifest>
        <project name="included_project" path="path/to/included_project" remote="origin" revision="main"/>
    </manifest>
    "#).unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    println!("{:?}", manifest);

    assert_eq!(manifest.notice, Some("This is a notice".to_string()));
    assert_eq!(manifest.remotes.len(), 1);
    assert_eq!(manifest.remotes[0].name, "origin");
    assert_eq!(
        manifest.default.as_ref().unwrap().remote,
        Some("origin".to_string())
    );
    assert_eq!(
        manifest.manifest_server.as_ref().unwrap().url,
        "https://example.com/manifest"
    );
    assert_eq!(manifest.submanifests.len(), 1);
    assert_eq!(manifest.projects.len(), 2); // Includes the project from the included manifest
    assert_eq!(manifest.extend_projects.len(), 1);
    assert_eq!(manifest.remove_projects.len(), 1);
    assert_eq!(manifest.repo_hooks.as_ref().unwrap().in_project, "hooks");
    assert_eq!(manifest.superproject.as_ref().unwrap().name, "super");
    assert_eq!(
        manifest.contactinfo.as_ref().unwrap().bugurl,
        "https://example.com/bugs"
    );
    assert_eq!(manifest.includes.len(), 1);
}

#[test]
fn test_parse_invalid_manifest() {
    // Test parsing an invalid manifest with missing closing tag
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid_manifest.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <remote name="origin" fetch="https://example.com/repo.git">
    </manifest>
    "#
    )
    .unwrap();

    let result = Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main"));
    assert!(result.is_err());
}

#[test]
fn test_missing_required_attributes() {
    // Test parsing a manifest with missing required attributes
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("missing_attributes.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <remote fetch="https://example.com/repo.git"/>
    </manifest>
    "#
    )
    .unwrap();

    let result = Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main"));
    assert!(result.is_err());
}

#[test]
fn test_invalid_xml_format() {
    // Test parsing a manifest with invalid XML format
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid_xml.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <remote name="origin" fetch="https://example.com/repo.git">
    </manifest
    "#
    )
    .unwrap();

    let result = Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main"));
    assert!(result.is_err());
}

#[test]
fn test_empty_manifest() {
    // Test parsing an empty manifest
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("empty_manifest.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
    </manifest>
    "#
    )
    .unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    assert!(manifest.notice.is_none());
    assert!(manifest.remotes.is_empty());
    // assert!(manifest.default.is_none());
    assert!(manifest.manifest_server.is_none());
    assert!(manifest.submanifests.is_empty());
    assert!(manifest.remove_projects.is_empty());
    assert!(manifest.projects.is_empty());
    assert!(manifest.extend_projects.is_empty());
    assert!(manifest.repo_hooks.is_none());
    assert!(manifest.superproject.is_none());
    assert!(manifest.contactinfo.is_none());
    assert!(manifest.includes.is_empty());
}

#[test]
fn test_invalid_element() {
    // Test parsing a manifest with an invalid element
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("invalid_element.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <invalid-element/>
    </manifest>
    "#
    )
    .unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    assert!(manifest.notice.is_none());
    assert!(manifest.remotes.is_empty());
    // assert!(manifest.default.is_none());
    assert!(manifest.manifest_server.is_none());
    assert!(manifest.submanifests.is_empty());
    assert!(manifest.remove_projects.is_empty());
    assert!(manifest.projects.is_empty());
    assert!(manifest.extend_projects.is_empty());
    assert!(manifest.repo_hooks.is_none());
    assert!(manifest.superproject.is_none());
    assert!(manifest.contactinfo.is_none());
    assert!(manifest.includes.is_empty());
}

#[test]
fn test_multiple_remotes() {
    // Test parsing a manifest with multiple remote elements
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("multiple_remotes.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <remote name="origin" fetch="https://example.com/repo.git"/>
        <remote name="backup" fetch="https://backup.example.com/repo.git"/>
    </manifest>
    "#
    )
    .unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    assert_eq!(manifest.remotes.len(), 2);
    assert_eq!(manifest.remotes[0].name, "origin");
    assert_eq!(manifest.remotes[1].name, "backup");
}

#[test]
fn test_project_with_annotations() {
    // Test parsing a project with annotations
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("project_with_annotations.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <project name="annotated_project" path="annotated_project">
            <annotation name="key1" value="value1"/>
            <annotation name="key2" value="value2"/>
        </project>
    </manifest>
    "#
    )
    .unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    assert_eq!(manifest.projects.len(), 1);
    assert_eq!(manifest.projects[0].name, "annotated_project");
    assert_eq!(
        manifest.projects[0].path.as_deref(),
        Some("annotated_project")
    );
    // Annotations are not directly parsed into the main projects list
}

#[test]
fn test_parse_valid_manifest_with_include() {
    // Test parsing a valid manifest with an include element
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("valid_manifest.xml");
    let included_file_path = dir.path().join("included_manifest.xml");
    let mut file = File::create(&file_path).unwrap();
    let mut included_file = File::create(&included_file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <notice>This is a notice</notice>
        <remote name="origin" fetch="https://example.com/repo.git"/>
        <default remote="origin" revision="main"/>
        <manifest-server url="https://example.com/manifest"/>
        <submanifest name="sub1" remote="origin" project="subproject"/>
        <project name="project1" path="path/to/project1" remote="origin" revision="main"/>
        <extend-project name="project1" path="path/to/project1" revision="develop"/>
        <remove-project name="project2"/>
        <repo-hooks in-project="hooks" enabled-list="pre-upload"/>
        <superproject name="super" remote="origin" revision="main"/>
        <contactinfo bugurl="https://example.com/bugs"/>
        <include name="included_manifest.xml"/>
    </manifest>
    "#
    )
    .unwrap();

    writeln!(included_file, r#"
    <manifest>
        <project name="included_project" path="path/to/included_project" remote="origin" revision="main"/>
    </manifest>
    "#).unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    println!("{:?}", manifest);

    assert_eq!(manifest.notice, Some("This is a notice".to_string()));
    assert_eq!(manifest.remotes.len(), 1);
    assert_eq!(manifest.remotes[0].name, "origin");
    assert_eq!(
        manifest.default.as_ref().unwrap().remote,
        Some("origin".to_string())
    );
    assert_eq!(
        manifest.manifest_server.as_ref().unwrap().url,
        "https://example.com/manifest"
    );
    assert_eq!(manifest.submanifests.len(), 1);
    assert_eq!(manifest.projects.len(), 2); // Includes the project from the included manifest
    assert_eq!(manifest.extend_projects.len(), 1);
    assert_eq!(manifest.remove_projects.len(), 1);
    assert_eq!(manifest.repo_hooks.as_ref().unwrap().in_project, "hooks");
    assert_eq!(manifest.superproject.as_ref().unwrap().name, "super");
    assert_eq!(
        manifest.contactinfo.as_ref().unwrap().bugurl,
        "https://example.com/bugs"
    );
    assert_eq!(manifest.includes.len(), 1);
}

#[test]
fn test_parse_valid_manifest_without_include() {
    // Test parsing a valid manifest without an include element
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("valid_manifest.xml");
    let mut file = File::create(&file_path).unwrap();

    writeln!(
        file,
        r#"
    <manifest>
        <notice>This is a notice</notice>
        <remote name="origin" fetch="https://example.com/repo.git"/>
        <default remote="origin" revision="main"/>
        <manifest-server url="https://example.com/manifest"/>
        <submanifest name="sub1" remote="origin" project="subproject"/>
        <project name="project1" path="path/to/project1" remote="origin" revision="main"/>
        <extend-project name="project1" path="path/to/project1" revision="develop"/>
        <remove-project name="project2"/>
        <repo-hooks in-project="hooks" enabled-list="pre-upload"/>
        <superproject name="super" remote="origin" revision="main"/>
        <contactinfo bugurl="https://example.com/bugs"/>
    </manifest>
    "#
    )
    .unwrap();

    let manifest =
        Manifest::from_file(file_path.to_str().unwrap(), Some("origin"), Some("main")).unwrap();
    println!("{:?}", manifest);

    assert_eq!(manifest.notice, Some("This is a notice".to_string()));
    assert_eq!(manifest.remotes.len(), 1);
    assert_eq!(manifest.remotes[0].name, "origin");
    assert_eq!(
        manifest.default.as_ref().unwrap().remote,
        Some("origin".to_string())
    );
    assert_eq!(
        manifest.manifest_server.as_ref().unwrap().url,
        "https://example.com/manifest"
    );
    assert_eq!(manifest.submanifests.len(), 1);
    assert_eq!(manifest.projects.len(), 1);
    assert_eq!(manifest.extend_projects.len(), 1);
    assert_eq!(manifest.remove_projects.len(), 1);
    assert!(manifest.repo_hooks.is_some());
    assert!(manifest.superproject.is_some());
    assert!(manifest.contactinfo.is_some());
    assert!(manifest.includes.is_empty());
}
