use quick_xml::events::Event;
use quick_xml::name::QName;
use quick_xml::Reader;
use std::fs::File;
use std::io::BufReader;

pub mod sync;

/// A struct representing a repo manifest.
///
/// A repo manifest describes the structure of a repo client; that is
/// the directories that are visible and where they should be obtained
/// from with git.
///
/// # Example
///
/// ```ignore
/// use manifest_parser::Manifest;
///
/// let manifest = Manifest::from_file("path/to/manifest.xml").unwrap();
/// println!("{:?}", manifest);
/// ```
///
/// The basic structure of a manifest is a bare Git repository holding
/// a single `default.xml` XML file in the top level directory.
///
/// Manifests are inherently version controlled, since they are kept
/// within a Git repository. Updates to manifests are automatically
/// obtained by clients during `repo sync`.
#[derive(Debug)]
pub struct Manifest {
    /// Arbitrary text that is displayed to users whenever `repo sync` finishes.
    pub notice: Option<String>,
    /// One or more remote elements may be specified.
    pub remotes: Vec<Remote>,
    /// At most one default element may be specified.
    pub default: Option<Default>,
    /// At most one manifest-server may be specified.
    pub manifest_server: Option<ManifestServer>,
    /// One or more submanifest elements may be specified.
    pub submanifests: Vec<Submanifest>,
    /// Deletes a project from the internal manifest table.
    pub remove_projects: Vec<RemoveProject>,
    /// One or more project elements may be specified.
    pub projects: Vec<Project>,
    /// Modify the attributes of the named project.
    pub extend_projects: Vec<ExtendProject>,
    /// Only one repo-hooks element may be specified at a time.
    pub repo_hooks: Option<RepoHooks>,
    /// This element is used to specify the URL of the superproject.
    pub superproject: Option<Superproject>,
    /// This element is used to let manifest authors self-register contact info.
    pub contactinfo: Option<ContactInfo>,
    /// This element provides the capability of including another manifest file.
    pub includes: Vec<Include>,
    /// One or more copyfile elements may be specified.
    pub copyfiles: Vec<CopyFile>,
    /// One or more linkfile elements may be specified.
    pub linkfiles: Vec<LinkFile>,
    /// One or more annotation elements may be specified.
    pub annotations: Vec<Annotation>,
}

#[derive(Debug)]
pub struct Remote {
    pub name: String,
    pub alias: Option<String>,
    pub fetch: String,
    pub pushurl: Option<String>,
    pub review: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug)]
pub struct Default {
    pub remote: Option<String>,
    pub revision: Option<String>,
    pub dest_branch: Option<String>,
    pub upstream: Option<String>,
    pub sync_j: Option<String>,
    pub sync_c: Option<String>,
    pub sync_s: Option<String>,
    pub sync_tags: Option<String>,
}

#[derive(Debug)]
pub struct ManifestServer {
    pub url: String,
}

#[derive(Debug)]
pub struct Submanifest {
    pub name: String,
    pub remote: Option<String>,
    pub project: Option<String>,
    pub manifest_name: Option<String>,
    pub revision: Option<String>,
    pub path: Option<String>,
    pub groups: Option<String>,
    pub default_groups: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Project {
    // "name" must not be empty, and may not Fbe an absolute path or use "." or ".."
    // path components.  It is always interpreted relative to the remote's fetch
    // settings, so if a different base path is needed, declare a different remote
    // with the new settings needed.
    // These restrictions are not enforced for [Local Manifests].
    pub name: String,
    // Attribute `path`: An optional path relative to the top directory
    // of the repo client where the submanifest repo client top directory
    // should be placed.  If not supplied, `revision` is used.
    // `path` may not be an absolute path or use "." or ".." path components.
    pub path: Option<String>,
    pub remote: Option<String>,
    pub revision: Option<String>,
    pub dest_branch: Option<String>,
    // Attribute `groups`: List of additional groups to which all projects
    // in the included submanifest belong. This appends and recurses, meaning
    // all projects in submanifests carry all parent submanifest groups.
    // Same syntax as the corresponding element of `project`.
    pub groups: Option<String>,
    pub sync_c: Option<String>,
    pub sync_s: Option<String>,
    pub sync_tags: Option<String>,
    pub upstream: Option<String>,
    pub clone_depth: Option<String>,
    pub force_path: Option<String>,
}

#[derive(Debug)]
pub struct ExtendProject {
    pub name: String,
    pub path: Option<String>,
    pub dest_path: Option<String>,
    pub groups: Option<String>,
    pub revision: Option<String>,
    pub remote: Option<String>,
    pub dest_branch: Option<String>,
    pub upstream: Option<String>,
    pub base_rev: Option<String>,
}

#[derive(Debug)]
pub struct RemoveProject {
    pub name: Option<String>,
    pub path: Option<String>,
    pub optional: Option<String>,
    pub base_rev: Option<String>,
}

#[derive(Debug)]
pub struct RepoHooks {
    pub in_project: String,
    pub enabled_list: String,
}

#[derive(Debug)]
pub struct Superproject {
    pub name: String,
    pub remote: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug)]
pub struct ContactInfo {
    pub bugurl: String,
}

#[derive(Debug, Clone)]
pub struct Include {
    pub name: String,
    pub groups: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug)]
pub struct CopyFile {
    pub src: String,
    pub dest: String,
}

#[derive(Debug)]
pub struct LinkFile {
    pub src: String,
    pub dest: String,
}

#[derive(Debug)]
pub struct Annotation {
    pub name: String,
    pub value: String,
    pub keep: bool,
}

impl Manifest {
    /// Parses a manifest XML file and returns a `Manifest` struct.
    ///
    /// # Arguments
    ///
    /// * `file_path` - A string slice that holds the path to the manifest XML file.
    /// * `default_remote` - An optional default remote to use if the manifest does not specify one.
    /// * `default_revision` - An optional default revision to use if the manifest does not specify one.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use manifest_parser::Manifest;
    ///
    /// let manifest = Manifest::from_file("path/to/manifest.xml", Some("origin"), Some("main")).unwrap();
    /// println!("{:?}", manifest);
    /// ```
    pub fn from_file(
        file_path: &str,
        default_remote: Option<&str>,
        default_revision: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut manifest = Manifest {
            notice: None,
            remotes: Vec::new(),
            default: None,
            manifest_server: None,
            submanifests: Vec::new(),
            remove_projects: Vec::new(),
            projects: Vec::new(),
            extend_projects: Vec::new(),
            repo_hooks: None,
            superproject: None,
            contactinfo: None,
            includes: Vec::new(),
            copyfiles: Vec::new(),
            linkfiles: Vec::new(),
            annotations: Vec::new(),
        };

        manifest.parse_file(file_path)?;

        // Set default values if the default element is missing
        if manifest.default.is_none() {
            manifest.default = Some(Default {
                remote: default_remote.map(String::from),
                revision: default_revision.map(String::from),
                dest_branch: None,
                upstream: None,
                sync_j: None,
                sync_c: None,
                sync_s: None,
                sync_tags: None,
            });
        }

        Ok(manifest)
    }

    fn parse_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let file = BufReader::new(file);
        let mut reader = Reader::from_reader(file);

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                    let element = e.to_owned();
                    self.parse_element(&element, &mut reader, &mut buf, file_path)?;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Box::new(e)),
                _ => (),
            }
            buf.clear();
        }

        Ok(())
    }

    fn parse_element(
        &mut self,
        e: &quick_xml::events::BytesStart,
        reader: &mut Reader<BufReader<File>>,
        buf: &mut Vec<u8>,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match e.name() {
            QName(b"notice") => {
                if let Ok(Event::Text(e)) = reader.read_event_into(buf) {
                    self.notice = Some(e.unescape()?.to_string());
                }
            }
            QName(b"remote") => self.parse_remote(e)?,
            QName(b"default") => self.parse_default(e)?,
            QName(b"manifest-server") => self.parse_manifest_server(e)?,
            QName(b"submanifest") => self.parse_submanifest(e)?,
            QName(b"remove-project") => self.parse_remove_project(e)?,
            QName(b"project") => self.parse_project(e)?,
            QName(b"extend-project") => self.parse_extend_project(e)?,
            QName(b"repo-hooks") => self.parse_repo_hooks(e)?,
            QName(b"superproject") => self.parse_superproject(e)?,
            QName(b"contactinfo") => self.parse_contactinfo(e)?,
            QName(b"include") => self.parse_include(e, file_path)?,
            QName(b"copyfile") => self.parse_copyfile(e)?,
            QName(b"linkfile") => self.parse_linkfile(e)?,
            QName(b"annotation") => self.parse_annotation(e)?,
            _ => (),
        }
        Ok(())
    }

    fn parse_remote(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut remote = Remote {
            name: String::new(),
            alias: None,
            fetch: String::new(),
            pushurl: None,
            review: None,
            revision: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => remote.name = attr.unescape_value()?.to_string(),
                b"alias" => remote.alias = Some(attr.unescape_value()?.to_string()),
                b"fetch" => remote.fetch = attr.unescape_value()?.to_string(),
                b"pushurl" => remote.pushurl = Some(attr.unescape_value()?.to_string()),
                b"review" => remote.review = Some(attr.unescape_value()?.to_string()),
                b"revision" => remote.revision = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        if remote.name.is_empty() || remote.fetch.is_empty() {
            return Err("Missing required attributes in remote element".into());
        }
        self.remotes.push(remote);
        Ok(())
    }

    fn parse_default(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut default = Default {
            remote: None,
            revision: None,
            dest_branch: None,
            upstream: None,
            sync_j: None,
            sync_c: None,
            sync_s: None,
            sync_tags: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"remote" => default.remote = Some(attr.unescape_value()?.to_string()),
                b"revision" => default.revision = Some(attr.unescape_value()?.to_string()),
                b"dest-branch" => default.dest_branch = Some(attr.unescape_value()?.to_string()),
                b"upstream" => default.upstream = Some(attr.unescape_value()?.to_string()),
                b"sync-j" => default.sync_j = Some(attr.unescape_value()?.to_string()),
                b"sync-c" => default.sync_c = Some(attr.unescape_value()?.to_string()),
                b"sync-s" => default.sync_s = Some(attr.unescape_value()?.to_string()),
                b"sync-tags" => default.sync_tags = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        self.default = Some(default);
        Ok(())
    }

    fn parse_manifest_server(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(attr) = e.attributes().find(|a| {
            a.as_ref()
                .map(|a| a.key.as_ref() == b"url")
                .unwrap_or(false)
        }) {
            self.manifest_server = Some(ManifestServer {
                url: attr?.unescape_value()?.to_string(),
            });
        }
        Ok(())
    }

    fn parse_submanifest(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut submanifest = Submanifest {
            name: String::new(),
            remote: None,
            project: None,
            manifest_name: None,
            revision: None,
            path: None,
            groups: None,
            default_groups: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => submanifest.name = attr.unescape_value()?.to_string(),
                b"remote" => submanifest.remote = Some(attr.unescape_value()?.to_string()),
                b"project" => submanifest.project = Some(attr.unescape_value()?.to_string()),
                b"manifest-name" => {
                    submanifest.manifest_name = Some(attr.unescape_value()?.to_string())
                }
                b"revision" => submanifest.revision = Some(attr.unescape_value()?.to_string()),
                b"path" => submanifest.path = Some(attr.unescape_value()?.to_string()),
                b"groups" => submanifest.groups = Some(attr.unescape_value()?.to_string()),
                b"default-groups" => {
                    submanifest.default_groups = Some(attr.unescape_value()?.to_string())
                }
                _ => (),
            }
        }
        self.submanifests.push(submanifest);
        Ok(())
    }

    fn parse_remove_project(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut remove_project = RemoveProject {
            name: None,
            path: None,
            optional: None,
            base_rev: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => remove_project.name = Some(attr.unescape_value()?.to_string()),
                b"path" => remove_project.path = Some(attr.unescape_value()?.to_string()),
                b"optional" => remove_project.optional = Some(attr.unescape_value()?.to_string()),
                b"base-rev" => remove_project.base_rev = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        self.remove_projects.push(remove_project);
        Ok(())
    }

    fn parse_project(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut project = Project {
            name: String::new(),
            path: None,
            remote: None,
            revision: None,
            dest_branch: None,
            groups: None,
            sync_c: None,
            sync_s: None,
            sync_tags: None,
            upstream: None,
            clone_depth: None,
            force_path: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => project.name = attr.unescape_value()?.to_string(),
                b"path" => project.path = Some(attr.unescape_value()?.to_string()),
                b"remote" => project.remote = Some(attr.unescape_value()?.to_string()),
                b"revision" => project.revision = Some(attr.unescape_value()?.to_string()),
                b"dest-branch" => project.dest_branch = Some(attr.unescape_value()?.to_string()),
                b"groups" => project.groups = Some(attr.unescape_value()?.to_string()),
                b"sync-c" => project.sync_c = Some(attr.unescape_value()?.to_string()),
                b"sync_s" => project.sync_s = Some(attr.unescape_value()?.to_string()),
                b"sync-tags" => project.sync_tags = Some(attr.unescape_value()?.to_string()),
                b"upstream" => project.upstream = Some(attr.unescape_value()?.to_string()),
                b"clone-depth" => project.clone_depth = Some(attr.unescape_value()?.to_string()),
                b"force-path" => project.force_path = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        if project.name.is_empty() {
            return Err("Missing required attribute 'name' in project element".into());
        }
        self.projects.push(project);
        Ok(())
    }

    fn parse_extend_project(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut extend_project = ExtendProject {
            name: String::new(),
            path: None,
            dest_path: None,
            groups: None,
            revision: None,
            remote: None,
            dest_branch: None,
            upstream: None,
            base_rev: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => extend_project.name = attr.unescape_value()?.to_string(),
                b"path" => extend_project.path = Some(attr.unescape_value()?.to_string()),
                b"dest-path" => extend_project.dest_path = Some(attr.unescape_value()?.to_string()),
                b"groups" => extend_project.groups = Some(attr.unescape_value()?.to_string()),
                b"revision" => extend_project.revision = Some(attr.unescape_value()?.to_string()),
                b"remote" => extend_project.remote = Some(attr.unescape_value()?.to_string()),
                b"dest-branch" => {
                    extend_project.dest_branch = Some(attr.unescape_value()?.to_string())
                }
                b"upstream" => extend_project.upstream = Some(attr.unescape_value()?.to_string()),
                b"base-rev" => extend_project.base_rev = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        self.extend_projects.push(extend_project);
        Ok(())
    }

    fn parse_repo_hooks(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repo_hooks = RepoHooks {
            in_project: String::new(),
            enabled_list: String::new(),
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"in-project" => repo_hooks.in_project = attr.unescape_value()?.to_string(),
                b"enabled-list" => repo_hooks.enabled_list = attr.unescape_value()?.to_string(),
                _ => (),
            }
        }
        self.repo_hooks = Some(repo_hooks);
        Ok(())
    }

    fn parse_superproject(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut superproject = Superproject {
            name: String::new(),
            remote: None,
            revision: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => superproject.name = attr.unescape_value()?.to_string(),
                b"remote" => superproject.remote = Some(attr.unescape_value()?.to_string()),
                b"revision" => superproject.revision = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        self.superproject = Some(superproject);
        Ok(())
    }

    fn parse_contactinfo(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(attr) = e.attributes().find(|a| {
            a.as_ref()
                .map(|a| a.key.as_ref() == b"bugurl")
                .unwrap_or(false)
        }) {
            self.contactinfo = Some(ContactInfo {
                bugurl: attr?.unescape_value()?.to_string(),
            });
        }
        Ok(())
    }

    fn parse_include(
        &mut self,
        e: &quick_xml::events::BytesStart,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut include = Include {
            name: String::new(),
            groups: None,
            revision: None,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => include.name = attr.unescape_value()?.to_string(),
                b"groups" => include.groups = Some(attr.unescape_value()?.to_string()),
                b"revision" => include.revision = Some(attr.unescape_value()?.to_string()),
                _ => (),
            }
        }
        self.includes.push(include.clone());
        let include_path = format!(
            "{}/{}",
            std::path::Path::new(file_path).parent().unwrap().display(),
            include.name
        );
        if let Err(e) = self.parse_file(&include_path) {
            eprintln!("Failed to parse included file '{}': {}", include_path, e);
            if !include.name.is_empty() {
                return Err(e);
            }
        }
        Ok(())
    }

    fn parse_copyfile(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut copyfile = CopyFile {
            src: String::new(),
            dest: String::new(),
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"src" => copyfile.src = attr.unescape_value()?.to_string(),
                b"dest" => copyfile.dest = attr.unescape_value()?.to_string(),
                _ => (),
            }
        }
        self.copyfiles.push(copyfile);
        Ok(())
    }

    fn parse_linkfile(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut linkfile = LinkFile {
            src: String::new(),
            dest: String::new(),
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"src" => linkfile.src = attr.unescape_value()?.to_string(),
                b"dest" => linkfile.dest = attr.unescape_value()?.to_string(),
                _ => (),
            }
        }
        self.linkfiles.push(linkfile);
        Ok(())
    }

    fn parse_annotation(
        &mut self,
        e: &quick_xml::events::BytesStart,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut annotation = Annotation {
            name: String::new(),
            value: String::new(),
            keep: true,
        };
        for attr in e.attributes() {
            let attr = attr?;
            match attr.key.as_ref() {
                b"name" => annotation.name = attr.unescape_value()?.to_string(),
                b"value" => annotation.value = attr.unescape_value()?.to_string(),
                b"keep" => {
                    annotation.keep = attr.unescape_value()?.to_string().to_lowercase() == "true"
                }
                _ => (),
            }
        }
        self.annotations.push(annotation);
        Ok(())
    }
}
