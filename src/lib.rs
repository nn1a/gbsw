use std::collections::HashMap;
use std::process::{Command, ExitStatus, Stdio};

// positional arguments:
//   gitdir                git repository path, which can contain multiple packages, in this case, all packages will be
//                         built in dependency order

// options:
//   -h, --help            show this help message and exit

// build configuration options:
//   -A ARCH, --arch ARCH  build target arch. Supported arch types are: x86_64 i586 armv6l armv7hl armv7l aarch64 mips
//                         mipsel
//   -D DIST, --dist DIST  specify project (build) configuration file. User can specify build config in [profile.xx]
//                         section of gbs.conf using 'buildconf' key, and the value is local path of build conf file
//   -P PROFILE, --profile PROFILE
//                         profile to be used for building, it is defined in .gbs.conf, can be given without the
//                         "profile." prefix
//   -R REPOSITORIES, --repository REPOSITORIES
//                         specify package repositories, only rpm-md format is supported
//   --skip-conf-repos     skip repositories mentioned in config file, and only use repos from command line -R option
//   --overwrite           overwrite existing binaries and build them anyway
//   --define DEFINE       define macro X with value Y with format "X Y"
//   --debug               debug output
//   --baselibs            create -32bit/-64bit/-x86 rpms for other architectures

// build env options:
//   -B BUILDROOT, --buildroot BUILDROOT
//                         specify build root to setup chroot environment. By default, ~/GBS-ROOT/ will be used. User can
//                         specify customized build root in gbs.conf with 'buildroot' key, which can be set in [general]
//                         section for default build root, or in [profile.xx] section for profile special build root
//   -C, --clean           delete old build root before initialization
//   --clean-once          clean the build environment only once when you start building multiple packages, after that
//                         use existing environment for all packages.
//   --clean-repos         clean up local repos created by gbs build before building packages
//   --fail-fast           stop build if one of packages fails
//   --keepgoing KEEPGOING
//                         If a package build fails, do not abort and continuebuilding other packages in the queue
//   --extra-packs EXTRA_PACKS
//                         specify extra packages to install to build root, Multiple packages can be separated by
//                         comma(,)
//   --keep-packs          keep unused packages in build root. without this option, unused packages will be removed from
//                         build root
//   --use-higher-deps     Which repo provides higher version deps, use it
//   --kvm                 Launch a kvm machine to build package instead of using chroot
//   --vm-memory VM_MEMORY
//                         The memory size of kvm machine
//   --vm-disk VM_DISK     The disk size of kvm machine
//   --vm-swap VM_SWAP     The swap size of kvm machine
//   --vm-diskfilesystem VM_DISKFILESYSTEM
//                         The filesystem type of kvm machine
//   --vm-initrd VM_INITRD
//                         The initrd of kvm machine
//   --vm-kernel VM_KERNEL
//                         The kernel of kvm machine
//   --not-export-source   Do not export source, use git source to build directly
//   --full-build          Download all the package sources except local package in gbs.conf, and do build
//   --deps-build          Download packages depends on local package from gbs.conf, and do build
//   --snapshot SNAPSHOT   Specify snapshot id to use

// speed up building options:
//   --incremental         build a package from the local git tree incremental. If the build fails, changes can be done
//                         directly to the source and build can continue from where it stopped
//   --no-configure        this option disables running configure scripts and auto generation of auto-tools to make
//                         incremental build possible. This requires the configure scripts in the spec to be referenced
//                         using the %configure, %reconfigure and %autogen macros
//   --noinit              working in offline mode. Start building directly
//   --ccache              use ccache to speed up rebuilds
//   --pkg-ccache PKG_CCACHE
//                         set ccache.tar file and enable ccache option , use ccache.tar file to speed up rebuilds
//   --icecream ICECREAM   Use N parallel build jobs with icecream
//   --threads THREADS     number of threads to build multiple packages in parallel
//   --skip-srcrpm         don't build source rpm file

// git-tree options:
//   -c COMMIT, --commit COMMIT
//                         specify a commit ID to build
//   --include-all         uncommitted changes and untracked files would be included while generating tar ball
//   --packaging-dir PACKAGING_DIR
//                         directory containing packaging files
//   --spec SPEC           specify a spec file to use. It should be a file name that GBS will find it in packaging dir
//   --upstream-branch UPSTREAM_BRANCH
//                         upstream branch
//   --upstream-tag UPSTREAM_TAG
//                         upstream tag format, '${upstreamversion}' is expanded to the version in the spec file. E.g.
//                         'v${upstreamversion}'
//   --fallback-to-native  Fall back to native packaging mode (i.e. create tarball directly from exported commit, not
//                         from upstream tag, and without patches) in case patch or upstream tarball generation fails.
//   --squash-patches-until SQUASH_PATCHES_UNTIL
//                         when generating patches, squash patches up to given commit-ish into one monolithic diff file.
//                         Format is the commit-ish optionally followed by a colon and diff filename base.
//   --no-patch-export     don't create patches between upstream and export-treeish, and create tar ball from the export-
//                         treeish instead of upstream branch

// package selection options:
//   --package-list PACKAGE_LIST
//                         specify a package list to be built. Multiple packages can be separated by comma(,). Note:
//                         packages are package dir name
//   --package-from-file PACKAGE_FROM_FILE
//                         specify a package list file. Packages listed in this file will be selected to be built. The
//                         format of file is one package dir for one line
//   --binary-list BINARY_LIST
//                         specify a package list to be built. Multiple packages can be separated by comma(,). Note:
//                         package names are from spec files, not the package dir name
//   --binary-from-file BINARY_FROM_FILE
//                         specify a binary package list file. Packages listed in this file will be selected to be built.
//                         The format of binary-list file is one package for one line, and only binary RPM name is
//                         accepted
//   --exclude EXCLUDE     specify a package list to be excluded for building. Multiple packages can be separated by
//                         comma(,)
//   --exclude-from-file EXCLUDE_FROM_FILE
//                         specify an exclude package list text file, the format is one package in one line, and only
//                         binary RPM package name is accepted. Packages listed in this file will be skipped to be built.
//   --deps                build specified packages and all packages they depend on, such as A depends B,C,D, first build
//                         B,C,D and then build A
//   --rdeps               build specified packages and all packages depend on them, such as A B C depends D, first build
//                         D and then build A,B,C
//   --disable-debuginfo   Do not create debuginfo packages when building
//   --style STYLE         specify source type: git, or tar, default is git
//   --export-only         only export, not building
//   --preordered-list PREORDERED_LIST
//                         List of package to support manual ordering, either comma separated string or local file
//                         location.
//   --profiling PROFILING
//                         Profiling report location to be used package ordering.
//   --with-submodules     build project with submodules togerther
//   --release RELEASE     Override Release in spec file
//   --nocumulate          without cumulative build

/// Represents the options for the `gbs build` command.
#[derive(Default, Debug)]
pub struct GbsBuildOptions {
    // Positional arguments
    pub gitdir: Option<String>,

    // Build configuration options
    pub arch: Option<String>,
    pub dist: Option<String>,
    pub profile: Option<String>,
    pub repositories: Option<Vec<String>>,
    pub skip_conf_repos: bool,
    pub overwrite: bool,
    pub define: Option<HashMap<String, String>>,
    pub debug: bool,
    pub baselibs: bool,
    pub clean: bool,
    pub incremental: bool,
    pub no_configure: bool,
    pub noinit: bool,
    pub ccache: bool,
    pub pkg_ccache: Option<String>,
    pub icecream: Option<u32>,
    pub threads: Option<u32>,
    pub skip_srcrpm: bool,

    // Build environment options
    pub buildroot: Option<String>,
    pub clean_once: bool,
    pub clean_repos: bool,
    pub fail_fast: bool,
    pub keepgoing: Option<u32>,
    pub extra_packs: Option<Vec<String>>,
    pub keep_packs: bool,
    pub use_higher_deps: bool,
    pub kvm: bool,
    pub vm_memory: Option<String>,
    pub vm_disk: Option<String>,
    pub vm_swap: Option<String>,
    pub vm_diskfilesystem: Option<String>,
    pub vm_initrd: Option<String>,
    pub vm_kernel: Option<String>,

    // Additional options
    pub not_export_source: bool,
    pub full_build: bool,
    pub deps_build: bool,
    pub snapshot: Option<String>,

    // Git-tree options
    pub commit: Option<String>,
    pub include_all: bool,
    pub packaging_dir: Option<String>,
    pub spec: Option<String>,
    pub upstream_branch: Option<String>,
    pub upstream_tag: Option<String>,
    pub fallback_to_native: bool,
    pub squash_patches_until: Option<String>,
    pub no_patch_export: bool,

    // Package selection options
    pub package_list: Option<Vec<String>>,
    pub package_from_file: Option<String>,
    pub binary_list: Option<Vec<String>>,
    pub binary_from_file: Option<String>,
    pub exclude: Option<Vec<String>>,
    pub exclude_from_file: Option<String>,
    pub deps: bool,
    pub rdeps: bool,
    pub disable_debuginfo: bool,
    pub style: Option<String>,
    pub export_only: bool,
    pub preordered_list: Option<String>,
    pub profiling: Option<String>,
    pub with_submodules: bool,
    pub release: Option<String>,
    pub nocumulate: bool,
}

/// Represents the options for building with GBS (Git Build System).
///
/// This struct provides various configuration options for building packages
/// using GBS. It includes options for build configuration, build environment,
/// speed optimization, git-tree options, package selection, and positional
/// arguments.
///
/// # Methods
///
/// - `builder() -> GbsBuildOptionsBuilder`
///
///   Returns a builder for constructing `GbsBuildOptions` using the builder pattern.
///
/// - `to_args(&self) -> Vec<String>`
///
///   Converts the options into a vector of command-line arguments that can be
///   passed to the `gbs build` command.
///
/// - `execute(&self) -> Result<ExitStatus, std::io::Error>`
///
///   Executes the `gbs build` command with the specified options and returns
///   the output of the command.
///
/// # Fields
///
/// - `arch: Option<String>`
///
///   Specifies the target architecture for the build.
///
/// - `dist: Option<String>`
///
///   Specifies the distribution for the build.
///
/// - `profile: Option<String>`
///
///   Specifies the build profile.
///
/// - `repositories: Option<Vec<String>>`
///
///   Specifies additional repositories to use during the build.
///
/// - `skip_conf_repos: bool`
///
///   Skips the configuration repositories.
///
/// - `overwrite: bool`
///
///   Overwrites existing files.
///
/// - `define: Option<HashMap<String, String>>`
///
///   Defines additional variables for the build.
///
/// - `debug: bool`
///
///   Enables debug mode.
///
/// - `baselibs: bool`
///
///   Includes base libraries in the build.
///
/// - `buildroot: Option<String>`
///
///   Specifies the build root directory.
///
/// - `clean: bool`
///
///   Cleans the build directory before starting.
///
/// - `clean_once: bool`
///
///   Cleans the build directory once.
///
/// - `clean_repos: bool`
///
///   Cleans the repositories before starting.
///
/// - `fail_fast: bool`
///
///   Fails the build immediately on the first error.
///
/// - `keepgoing: Option<u32>`
///
///   Specifies the number of errors to tolerate before failing.
///
/// - `extra_packs: Option<Vec<String>>`
///
///   Specifies additional packages to include in the build.
///
/// - `keep_packs: bool`
///
///   Keeps the packages after the build.
///
/// - `use_higher_deps: bool`
///
///   Uses higher versions of dependencies.
///
/// - `kvm: bool`
///
///   Enables KVM (Kernel-based Virtual Machine) support.
///
/// - `vm_memory: Option<String>`
///
///   Specifies the amount of memory for the virtual machine.
///
/// - `vm_disk: Option<String>`
///
///   Specifies the disk size for the virtual machine.
///
/// - `vm_swap: Option<String>`
///
///   Specifies the swap size for the virtual machine.
///
/// - `vm_diskfilesystem: Option<String>`
///
///   Specifies the filesystem for the virtual machine disk.
///
/// - `vm_initrd: Option<String>`
///
///   Specifies the initrd image for the virtual machine.
///
/// - `vm_kernel: Option<String>`
///
///   Specifies the kernel image for the virtual machine.
///
/// - `not_export_source: bool`
///
///   Does not export the source code.
///
/// - `full_build: bool`
///
///   Performs a full build.
///
/// - `deps_build: bool`
///
///   Builds dependencies.
///
/// - `snapshot: Option<String>`
///
///   Specifies the snapshot to use for the build.
///
/// - `incremental: bool`
///
///   Enables incremental builds.
///
/// - `no_configure: bool`
///
///   Skips the configure step.
///
/// - `noinit: bool`
///
///   Skips the initialization step.
///
/// - `ccache: bool`
///
///   Enables ccache support.
///
/// - `pkg_ccache: Option<String>`
///
///   Specifies the ccache package.
///
/// - `icecream: Option<u32>`
///
///   Enables icecream distributed compilation.
///
/// - `skip_srcrpm: bool`
///
///   Skips the source RPM generation.
///
/// - `threads: Option<u32>`
///
///   Specifies the number of threads to use.
///
/// - `commit: Option<String>`
///
///   Specifies the commit to build.
///
/// - `include_all: bool`
///
///   Includes all files in the build.
///
/// - `packaging_dir: Option<String>`
///
///   Specifies the packaging directory.
///
/// - `spec: Option<String>`
///
///   Specifies the spec file.
///
/// - `upstream_branch: Option<String>`
///
///   Specifies the upstream branch.
///
/// - `upstream_tag: Option<String>`
///
///   Specifies the upstream tag.
///
/// - `fallback_to_native: bool`
///
///   Falls back to native build if cross-compilation fails.
///
/// - `squash_patches_until: Option<String>`
///
///   Squashes patches until the specified commit.
///
/// - `no_patch_export: bool`
///
///   Disables patch export.
///
/// - `package_list: Option<Vec<String>>`
///
///   Specifies the list of packages to build.
///
/// - `package_from_file: Option<String>`
///
///   Specifies a file containing the list of packages to build.
///
/// - `binary_list: Option<Vec<String>>`
///
///   Specifies the list of binaries to build.
///
/// - `binary_from_file: Option<String>`
///
///   Specifies a file containing the list of binaries to build.
///
/// - `exclude: Option<Vec<String>>`
///
///   Specifies the list of packages to exclude from the build.
///
/// - `exclude_from_file: Option<String>`
///
///   Specifies a file containing the list of packages to exclude.
///
/// - `deps: bool`
///
///   Includes dependencies in the build.
///
/// - `rdeps: bool`
///
///   Includes reverse dependencies in the build.
///
/// - `disable_debuginfo: bool`
///
///   Disables debug information generation.
///
/// - `style: Option<String>`
///
///   Specifies the build style.
///
/// - `export_only: bool`
///
///   Only exports the source code.
///
/// - `preordered_list: Option<String>`
///
///   Specifies a preordered list of packages.
///
/// - `profiling: Option<String>`
///
///   Enables profiling.
///
/// - `with_submodules: bool`
///
///   Includes submodules in the build.
///
/// - `release: Option<String>`
///
///   Specifies the release version.
///
/// - `nocumulate: bool`
///
///   Disables cumulative builds.
///
/// - `gitdir: Option<String>`
///
///   Specifies the git directory.
impl GbsBuildOptions {
    /// Builder pattern for GbsBuildOptions
    pub fn builder() -> GbsBuildOptionsBuilder {
        GbsBuildOptionsBuilder::default()
    }

    /// Converts the options into a vector of command-line arguments.
    pub fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Build configuration options
        if let Some(arch) = &self.arch {
            args.push("-A".to_string());
            args.push(arch.clone());
        }

        if let Some(dist) = &self.dist {
            args.push("-D".to_string());
            args.push(dist.clone());
        }

        if let Some(profile) = &self.profile {
            args.push("-P".to_string());
            args.push(profile.clone());
        }

        if let Some(repositories) = &self.repositories {
            for repo in repositories {
                args.push("-R".to_string());
                args.push(repo.clone());
            }
        }

        if self.skip_conf_repos {
            args.push("--skip-conf-repos".to_string());
        }

        if self.overwrite {
            args.push("--overwrite".to_string());
        }

        if let Some(define) = &self.define {
            let mut define_vec: Vec<_> = define.iter().collect();
            define_vec.sort_by_key(|&(key, _)| key);
            for (key, value) in define_vec {
                args.push("--define".to_string());
                args.push(format!("{} {}", key, value));
            }
        }

        if self.debug {
            args.push("--debug".to_string());
        }

        if self.baselibs {
            args.push("--baselibs".to_string());
        }

        // Build env options
        if let Some(buildroot) = &self.buildroot {
            args.push("-B".to_string());
            args.push(buildroot.clone());
        }

        if self.clean {
            args.push("-C".to_string());
        }

        if self.clean_once {
            args.push("--clean-once".to_string());
        }

        if self.clean_repos {
            args.push("--clean-repos".to_string());
        }

        if self.fail_fast {
            args.push("--fail-fast".to_string());
        }

        if let Some(keepgoing) = self.keepgoing {
            args.push("--keepgoing".to_string());
            args.push(keepgoing.to_string());
        }

        if let Some(extra_packs) = &self.extra_packs {
            args.push("--extra-packs".to_string());
            args.push(extra_packs.join(","));
        }

        if self.keep_packs {
            args.push("--keep-packs".to_string());
        }

        if self.use_higher_deps {
            args.push("--use-higher-deps".to_string());
        }

        if self.kvm {
            args.push("--kvm".to_string());
        }

        if let Some(vm_memory) = &self.vm_memory {
            args.push("--vm-memory".to_string());
            args.push(vm_memory.clone());
        }

        if let Some(vm_disk) = &self.vm_disk {
            args.push("--vm-disk".to_string());
            args.push(vm_disk.clone());
        }

        if let Some(vm_swap) = &self.vm_swap {
            args.push("--vm-swap".to_string());
            args.push(vm_swap.clone());
        }

        if let Some(vm_diskfilesystem) = &self.vm_diskfilesystem {
            args.push("--vm-diskfilesystem".to_string());
            args.push(vm_diskfilesystem.clone());
        }

        if let Some(vm_initrd) = &self.vm_initrd {
            args.push("--vm-initrd".to_string());
            args.push(vm_initrd.clone());
        }

        if let Some(vm_kernel) = &self.vm_kernel {
            args.push("--vm-kernel".to_string());
            args.push(vm_kernel.clone());
        }

        if self.not_export_source {
            args.push("--not-export-source".to_string());
        }

        if self.full_build {
            args.push("--full-build".to_string());
        }

        if self.deps_build {
            args.push("--deps-build".to_string());
        }

        if let Some(snapshot) = &self.snapshot {
            args.push("--snapshot".to_string());
            args.push(snapshot.clone());
        }

        // Speed up building options
        if self.incremental {
            args.push("--incremental".to_string());
        }

        if self.no_configure {
            args.push("--no-configure".to_string());
        }

        if self.noinit {
            args.push("--noinit".to_string());
        }

        if self.ccache {
            args.push("--ccache".to_string());
        }

        if let Some(pkg_ccache) = &self.pkg_ccache {
            args.push("--pkg-ccache".to_string());
            args.push(pkg_ccache.clone());
        }

        if let Some(icecream) = self.icecream {
            args.push("--icecream".to_string());
            args.push(icecream.to_string());
        }

        if self.skip_srcrpm {
            args.push("--skip-srcrpm".to_string());
        }

        if let Some(threads) = self.threads {
            args.push("--threads".to_string());
            args.push(threads.to_string());
        }

        // Git-tree options
        if let Some(commit) = &self.commit {
            args.push("-c".to_string());
            args.push(commit.clone());
        }

        if self.include_all {
            args.push("--include-all".to_string());
        }

        if let Some(packaging_dir) = &self.packaging_dir {
            args.push("--packaging-dir".to_string());
            args.push(packaging_dir.clone());
        }

        if let Some(spec) = &self.spec {
            args.push("--spec".to_string());
            args.push(spec.clone());
        }

        if let Some(upstream_branch) = &self.upstream_branch {
            args.push("--upstream-branch".to_string());
            args.push(upstream_branch.clone());
        }

        if let Some(upstream_tag) = &self.upstream_tag {
            args.push("--upstream-tag".to_string());
            args.push(upstream_tag.clone());
        }

        if self.fallback_to_native {
            args.push("--fallback-to-native".to_string());
        }

        if let Some(squash_patches_until) = &self.squash_patches_until {
            args.push("--squash-patches-until".to_string());
            args.push(squash_patches_until.clone());
        }

        if self.no_patch_export {
            args.push("--no-patch-export".to_string());
        }

        // Package selection options
        if let Some(package_list) = &self.package_list {
            for package in package_list {
                args.push("--package".to_string());
                args.push(package.clone());
            }
        }

        if let Some(package_from_file) = &self.package_from_file {
            args.push("--package-from-file".to_string());
            args.push(package_from_file.clone());
        }

        if let Some(binary_list) = &self.binary_list {
            for binary in binary_list {
                args.push("--binary".to_string());
                args.push(binary.clone());
            }
        }

        if let Some(binary_from_file) = &self.binary_from_file {
            args.push("--binary-from-file".to_string());
            args.push(binary_from_file.clone());
        }

        if let Some(exclude) = &self.exclude {
            for exclude in exclude {
                args.push("--exclude".to_string());
                args.push(exclude.clone());
            }
        }

        if let Some(exclude_from_file) = &self.exclude_from_file {
            args.push("--exclude-from-file".to_string());
            args.push(exclude_from_file.clone());
        }

        if self.deps {
            args.push("--deps".to_string());
        }

        if self.rdeps {
            args.push("--rdeps".to_string());
        }

        if self.disable_debuginfo {
            args.push("--disable-debuginfo".to_string());
        }

        if let Some(style) = &self.style {
            args.push("--style".to_string());
            args.push(style.clone());
        }

        if self.export_only {
            args.push("--export-only".to_string());
        }

        if let Some(preordered_list) = &self.preordered_list {
            args.push("--preordered-list".to_string());
            args.push(preordered_list.clone());
        }

        if let Some(profiling) = &self.profiling {
            args.push("--profiling".to_string());
            args.push(profiling.clone());
        }

        if self.with_submodules {
            args.push("--with-submodules".to_string());
        }

        if let Some(release) = &self.release {
            args.push("--release".to_string());
            args.push(release.clone());
        }

        if self.nocumulate {
            args.push("--nocumulate".to_string());
        }

        // Positional arguments
        // keep last
        if let Some(gitdir) = &self.gitdir {
            args.push(gitdir.clone());
        }

        args
    }

    /// Executes the `gbs build` command with the specified options.
    pub fn execute(&self) -> Result<ExitStatus, std::io::Error> {
        let mut command = Command::new("gbs");
        command.arg("build");
        command.args(self.to_args());

        let mut child = command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        child.wait()
    }
}

#[derive(Default)]
pub struct GbsBuildOptionsBuilder {
    options: GbsBuildOptions,
}

impl GbsBuildOptionsBuilder {
    // Build configuration options
    pub fn arch(mut self, arch: String) -> Self {
        self.options.arch = Some(arch);
        self
    }

    pub fn dist(mut self, dist: String) -> Self {
        self.options.dist = Some(dist);
        self
    }

    pub fn profile(mut self, profile: String) -> Self {
        self.options.profile = Some(profile);
        self
    }

    pub fn repositories(mut self, repositories: Vec<String>) -> Self {
        self.options.repositories = Some(repositories);
        self
    }
    pub fn repository(mut self, repository: String) -> Self {
        if let Some(repos) = &mut self.options.repositories {
            repos.push(repository);
        } else {
            self.options.repositories = Some(vec![repository]);
        }
        self
    }

    pub fn skip_conf_repos(mut self, skip: bool) -> Self {
        self.options.skip_conf_repos = skip;
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.options.overwrite = overwrite;
        self
    }

    pub fn define(mut self, define: HashMap<String, String>) -> Self {
        self.options.define = Some(define);
        self
    }

    pub fn debug(mut self, debug: bool) -> Self {
        self.options.debug = debug;
        self
    }

    pub fn baselibs(mut self, baselibs: bool) -> Self {
        self.options.baselibs = baselibs;
        self
    }

    // Build env options
    pub fn buildroot(mut self, buildroot: String) -> Self {
        self.options.buildroot = Some(buildroot);
        self
    }
    pub fn clean(mut self, clean: bool) -> Self {
        self.options.clean = clean;
        self
    }

    pub fn clean_once(mut self, clean_once: bool) -> Self {
        self.options.clean_once = clean_once;
        self
    }

    pub fn clean_repos(mut self, clean_repos: bool) -> Self {
        self.options.clean_repos = clean_repos;
        self
    }

    pub fn fail_fast(mut self, fail_fast: bool) -> Self {
        self.options.fail_fast = fail_fast;
        self
    }

    pub fn keepgoing(mut self, keepgoing: u32) -> Self {
        self.options.keepgoing = Some(keepgoing);
        self
    }

    pub fn extra_packs(mut self, extra_packs: Vec<String>) -> Self {
        self.options.extra_packs = Some(extra_packs);
        self
    }

    pub fn keep_packs(mut self, keep_packs: bool) -> Self {
        self.options.keep_packs = keep_packs;
        self
    }

    pub fn use_higher_deps(mut self, use_higher_deps: bool) -> Self {
        self.options.use_higher_deps = use_higher_deps;
        self
    }

    pub fn kvm(mut self, kvm: bool) -> Self {
        self.options.kvm = kvm;
        self
    }

    pub fn vm_memory(mut self, vm_memory: String) -> Self {
        self.options.vm_memory = Some(vm_memory);
        self
    }

    pub fn vm_disk(mut self, vm_disk: String) -> Self {
        self.options.vm_disk = Some(vm_disk);
        self
    }

    pub fn vm_swap(mut self, vm_swap: String) -> Self {
        self.options.vm_swap = Some(vm_swap);
        self
    }

    pub fn vm_diskfilesystem(mut self, vm_diskfilesystem: String) -> Self {
        self.options.vm_diskfilesystem = Some(vm_diskfilesystem);
        self
    }

    pub fn vm_initrd(mut self, vm_initrd: String) -> Self {
        self.options.vm_initrd = Some(vm_initrd);
        self
    }

    pub fn vm_kernel(mut self, vm_kernel: String) -> Self {
        self.options.vm_kernel = Some(vm_kernel);
        self
    }

    pub fn not_export_source(mut self, not_export_source: bool) -> Self {
        self.options.not_export_source = not_export_source;
        self
    }

    pub fn full_build(mut self, full_build: bool) -> Self {
        self.options.full_build = full_build;
        self
    }

    pub fn deps_build(mut self, deps_build: bool) -> Self {
        self.options.deps_build = deps_build;
        self
    }

    pub fn snapshot(mut self, snapshot: String) -> Self {
        self.options.snapshot = Some(snapshot);
        self
    }

    // Speed up building options
    pub fn incremental(mut self, incremental: bool) -> Self {
        self.options.incremental = incremental;
        self
    }

    pub fn no_configure(mut self, no_configure: bool) -> Self {
        self.options.no_configure = no_configure;
        self
    }

    pub fn noinit(mut self, noinit: bool) -> Self {
        self.options.noinit = noinit;
        self
    }

    pub fn ccache(mut self, ccache: bool) -> Self {
        self.options.ccache = ccache;
        self
    }

    pub fn pkg_ccache(mut self, pkg_ccache: String) -> Self {
        self.options.pkg_ccache = Some(pkg_ccache);
        self
    }

    pub fn icecream(mut self, icecream: u32) -> Self {
        self.options.icecream = Some(icecream);
        self
    }

    pub fn threads(mut self, threads: u32) -> Self {
        self.options.threads = Some(threads);
        self
    }

    pub fn skip_srcrpm(mut self, skip_srcrpm: bool) -> Self {
        self.options.skip_srcrpm = skip_srcrpm;
        self
    }

    // Git-tree options
    pub fn commit(mut self, commit: String) -> Self {
        self.options.commit = Some(commit);
        self
    }

    pub fn include_all(mut self, include_all: bool) -> Self {
        self.options.include_all = include_all;
        self
    }

    pub fn packaging_dir(mut self, packaging_dir: String) -> Self {
        self.options.packaging_dir = Some(packaging_dir);
        self
    }

    pub fn spec(mut self, spec: String) -> Self {
        self.options.spec = Some(spec);
        self
    }

    pub fn upstream_branch(mut self, upstream_branch: String) -> Self {
        self.options.upstream_branch = Some(upstream_branch);
        self
    }

    pub fn upstream_tag(mut self, upstream_tag: String) -> Self {
        self.options.upstream_tag = Some(upstream_tag);
        self
    }

    pub fn fallback_to_native(mut self, fallback_to_native: bool) -> Self {
        self.options.fallback_to_native = fallback_to_native;
        self
    }

    pub fn squash_patches_until(mut self, squash_patches_until: String) -> Self {
        self.options.squash_patches_until = Some(squash_patches_until);
        self
    }

    pub fn no_patch_export(mut self, no_patch_export: bool) -> Self {
        self.options.no_patch_export = no_patch_export;
        self
    }

    // Package selection options
    pub fn package_list(mut self, package_list: Vec<String>) -> Self {
        self.options.package_list = Some(package_list);
        self
    }

    pub fn package_from_file(mut self, package_from_file: String) -> Self {
        self.options.package_from_file = Some(package_from_file);
        self
    }

    pub fn binary_list(mut self, binary_list: Vec<String>) -> Self {
        self.options.binary_list = Some(binary_list);
        self
    }

    pub fn binary_from_file(mut self, binary_from_file: String) -> Self {
        self.options.binary_from_file = Some(binary_from_file);
        self
    }

    pub fn exclude(mut self, exclude: Vec<String>) -> Self {
        self.options.exclude = Some(exclude);
        self
    }

    pub fn exclude_from_file(mut self, exclude_from_file: String) -> Self {
        self.options.exclude_from_file = Some(exclude_from_file);
        self
    }

    pub fn deps(mut self, deps: bool) -> Self {
        self.options.deps = deps;
        self
    }

    pub fn rdeps(mut self, rdeps: bool) -> Self {
        self.options.rdeps = rdeps;
        self
    }

    pub fn disable_debuginfo(mut self, disable_debuginfo: bool) -> Self {
        self.options.disable_debuginfo = disable_debuginfo;
        self
    }

    pub fn style(mut self, style: String) -> Self {
        self.options.style = Some(style);
        self
    }

    pub fn export_only(mut self, export_only: bool) -> Self {
        self.options.export_only = export_only;
        self
    }

    pub fn preordered_list(mut self, preordered_list: String) -> Self {
        self.options.preordered_list = Some(preordered_list);
        self
    }

    pub fn profiling(mut self, profiling: String) -> Self {
        self.options.profiling = Some(profiling);
        self
    }

    pub fn with_submodules(mut self, with_submodules: bool) -> Self {
        self.options.with_submodules = with_submodules;
        self
    }

    pub fn release(mut self, release: String) -> Self {
        self.options.release = Some(release);
        self
    }

    pub fn nocumulate(mut self, nocumulate: bool) -> Self {
        self.options.nocumulate = nocumulate;
        self
    }

    pub fn gitdir(mut self, gitdir: String) -> Self {
        self.options.gitdir = Some(gitdir);
        self
    }

    pub fn build(self) -> GbsBuildOptions {
        self.options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_with_clean() {
        let options = GbsBuildOptions::builder()
            .arch("x86_64".to_string())
            .dist("tizen_5.5.conf".to_string())
            .profile("profile.tizen_5.5".to_string())
            .clean(true)
            .build();

        assert_eq!(
            options.to_args(),
            vec![
                "-A".to_string(),
                "x86_64".to_string(),
                "-D".to_string(),
                "tizen_5.5.conf".to_string(),
                "-P".to_string(),
                "profile.tizen_5.5".to_string(),
                "-C".to_string(),
            ]
        );
    }

    #[test]
    fn test_builder_with_incremental_and_no_configure() {
        let options = GbsBuildOptions::builder()
            .arch("aarch64".to_string())
            .dist("tizen_6.0.conf".to_string())
            .profile("profile.tizen_6.0".to_string())
            .incremental(true)
            .no_configure(true)
            .build();

        assert_eq!(
            options.to_args(),
            vec![
                "-A".to_string(),
                "aarch64".to_string(),
                "-D".to_string(),
                "tizen_6.0.conf".to_string(),
                "-P".to_string(),
                "profile.tizen_6.0".to_string(),
                "--incremental".to_string(),
                "--no-configure".to_string(),
            ]
        );
    }

    #[test]
    fn test_builder_with_all_options() {
        let options = GbsBuildOptions::builder()
            .arch("mips".to_string())
            .dist("tizen_7.0.conf".to_string())
            .profile("profile.tizen_7.0".to_string())
            .repositories(vec![
                "http://download.tizen.org/snapshots/TIZEN/Tizen/Tizen-Unified/reference/repos/standard/packages/".to_string(),
                "http://download.tizen.org/snapshots/TIZEN/Tizen/Tizen-Unified/reference/repos/emulator/packages/".to_string(),
            ])
            .clean(true)
            .incremental(true)
            .no_configure(true)
            .noinit(true)
            .ccache(true)
            .pkg_ccache("chromium-efl".to_string())
            .build();

        assert_eq!(options.to_args(), vec![
            "-A".to_string(), "mips".to_string(),
            "-D".to_string(), "tizen_7.0.conf".to_string(),
            "-P".to_string(), "profile.tizen_7.0".to_string(),
            "-R".to_string(), "http://download.tizen.org/snapshots/TIZEN/Tizen/Tizen-Unified/reference/repos/standard/packages/".to_string(),
            "-R".to_string(), "http://download.tizen.org/snapshots/TIZEN/Tizen/Tizen-Unified/reference/repos/emulator/packages/".to_string(),
            "-C".to_string(),
            "--incremental".to_string(),
            "--no-configure".to_string(),
            "--noinit".to_string(),
            "--ccache".to_string(),
            "--pkg-ccache".to_string(), "chromium-efl".to_string(),
        ]);
    }

    #[test]
    fn test_builder_with_gitdir() {
        let options = GbsBuildOptions::builder()
            .gitdir("/path/to/gitdir".to_string())
            .build();

        assert_eq!(options.to_args(), vec!["/path/to/gitdir".to_string()]);
    }

    #[test]
    fn test_builder_with_debug_and_baselibs() {
        let options = GbsBuildOptions::builder()
            .debug(true)
            .baselibs(true)
            .build();

        assert_eq!(
            options.to_args(),
            vec!["--debug".to_string(), "--baselibs".to_string(),]
        );
    }

    #[test]
    fn test_builder_with_vm_options() {
        let options = GbsBuildOptions::builder()
            .kvm(true)
            .vm_memory("4G".to_string())
            .vm_disk("20G".to_string())
            .vm_swap("2G".to_string())
            .vm_diskfilesystem("ext4".to_string())
            .vm_initrd("/path/to/initrd".to_string())
            .vm_kernel("/path/to/kernel".to_string())
            .build();

        assert_eq!(
            options.to_args(),
            vec![
                "--kvm".to_string(),
                "--vm-memory".to_string(),
                "4G".to_string(),
                "--vm-disk".to_string(),
                "20G".to_string(),
                "--vm-swap".to_string(),
                "2G".to_string(),
                "--vm-diskfilesystem".to_string(),
                "ext4".to_string(),
                "--vm-initrd".to_string(),
                "/path/to/initrd".to_string(),
                "--vm-kernel".to_string(),
                "/path/to/kernel".to_string(),
            ]
        );
    }

    #[test]
    fn test_builder_with_package_selection() {
        let options = GbsBuildOptions::builder()
            .package_list(vec!["package1".to_string(), "package2".to_string()])
            .exclude(vec!["package3".to_string()])
            .build();

        assert_eq!(
            options.to_args(),
            vec![
                "--package".to_string(),
                "package1".to_string(),
                "--package".to_string(),
                "package2".to_string(),
                "--exclude".to_string(),
                "package3".to_string(),
            ]
        );
    }

    #[test]
    fn test_define_option() {
        let mut define = HashMap::new();
        define.insert("FOO".to_string(), "bar".to_string());
        define.insert("BAZ".to_string(), "qux".to_string());

        let options = GbsBuildOptions::builder().define(define).build();

        assert_eq!(
            options.to_args(),
            vec![
                "--define".to_string(),
                "BAZ qux".to_string(),
                "--define".to_string(),
                "FOO bar".to_string(),
            ]
        );
    }
}
