//! Builders for generating manifest files for tests

use std::{collections::HashMap, fmt::Write as _};

pub struct Manifest {
    header: Header,
    dependencies: Dependencies,
    patches: Patches,
    target: Option<Target>,
}

impl Manifest {
    pub fn new(header: Header) -> Self {
        Self {
            header,
            dependencies: Dependencies::new(),
            patches: Patches::new(),
            target: None,
        }
    }

    pub fn add_dependency(mut self, dependency: Dependency) -> Self {
        self.dependencies.add(dependency);
        self
    }

    pub fn add_target(mut self, target: Target) -> Self {
        self.target = Some(target);
        self
    }

    pub fn add_patch(mut self, registry: &str, patch: Patch) -> Self {
        self.patches.add(registry, patch);
        self
    }

    pub fn render(self) -> String {
        let Self {
            header,
            dependencies,
            patches,
            target,
        } = self;

        let mut w = String::new();

        writeln!(w, "{}", header.render()).unwrap();
        write!(w, "{}", dependencies.render()).unwrap();
        write!(w, "{}", patches.render()).unwrap();
        if let Some(target) = target {
            write!(w, "{}", target.render()).unwrap();
        }

        w
    }
}

/// `[package]` section of manifest
pub struct Header {
    name: Option<String>,
    version: Option<String>,
    edition: Option<String>,
    default_comment: bool,
}

impl Header {
    pub fn basic(name: impl AsRef<str>) -> Self {
        Self {
            name: Some(name.as_ref().to_owned()),
            version: Some("0.1.0".to_owned()),
            edition: Some("2021".to_owned()),
            default_comment: true,
        }
    }
    pub fn name(mut self, name: impl Into<Option<String>>) -> Self {
        self.name = name.into();
        self
    }
    pub fn version(mut self, version: impl Into<Option<String>>) -> Self {
        self.version = version.into();
        self
    }
    pub fn render(self) -> String {
        let Self {
            name,
            version,
            edition,
            default_comment,
        } = self;

        let mut w = String::new();

        writeln!(w, "[package]").unwrap();
        if let Some(name) = name {
            writeln!(w, "name = \"{name}\"").unwrap();
        }
        if let Some(version) = version {
            writeln!(w, "version = \"{version}\"").unwrap();
        }
        if let Some(edition) = edition {
            writeln!(w, "edition = \"{edition}\"").unwrap();
        }
        if default_comment {
            writeln!(w).unwrap();
            writeln!(w, "# See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html").unwrap();
        }

        w
    }
}

/// `[dependencies]` section of manifest
struct Dependencies(Vec<Dependency>);

impl Dependencies {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn add(&mut self, dependency: Dependency) {
        self.0.push(dependency);
    }

    fn render(self) -> String {
        let Self(dependencies) = self;

        let mut f = String::new();
        if !dependencies.is_empty() {
            writeln!(f, "[dependencies]").unwrap();
        }
        for dep in dependencies {
            writeln!(f, "{}", dep.render()).unwrap();
        }
        f
    }
}

/// `[lib]` or `[[bin]]` section for manifest
///
/// Cargo tries to imply the type of a crate by looking at its `src` directory and seeing if there's a `lib.rs` or a `main.rs` file.
///
/// We're lazy, so we often do not create these files.
/// To prevent cargo from choking, we specify a target in the manifest.
/// Adding a fake path that points to nowhere is often sufficient.
pub struct Target {
    name: String,
    path: String,
    r#type: TargetType,
}

enum TargetType {
    Bin,
    Lib,
}

impl Target {
    pub fn bin(name: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            path: path.as_ref().to_owned(),
            r#type: TargetType::Bin,
        }
    }
    pub fn lib(name: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            path: path.as_ref().to_owned(),
            r#type: TargetType::Lib,
        }
    }
    fn render(self) -> String {
        let Self { name, path, r#type } = self;
        let mut w = String::new();
        writeln!(w).unwrap();
        match r#type {
            TargetType::Bin => writeln!(w, "[[bin]]"),
            TargetType::Lib => writeln!(w, "[lib]"),
        }
        .unwrap();
        writeln!(w, "name = \"{0}\"", name).unwrap();
        writeln!(w, "path = \"{0}\"", path).unwrap();
        w
    }
}

pub struct Dependency {
    name: String,
    version: String,
    registry: Option<String>,
    registry_index: Option<String>,
}

impl Dependency {
    pub fn new(name: impl AsRef<str>, version: impl AsRef<str>) -> Dependency {
        Dependency {
            name: name.as_ref().to_owned(),
            version: version.as_ref().to_owned(),
            registry: None,
            registry_index: None,
        }
    }

    pub fn registry(mut self, name: impl ToString) -> Dependency {
        self.registry = Some(name.to_string());
        self
    }

    pub fn registry_index(mut self, name: impl ToString) -> Dependency {
        self.registry_index = Some(name.to_string());
        self
    }

    fn render(self) -> String {
        match self {
            Self {
                name,
                version,
                registry: Some(registry),
                registry_index: None,
            } => {
                format!("{name} = {{ version = \"{version}\", registry = \"{registry}\" }}")
            }
            Self {
                name,
                version,
                registry: None,
                registry_index: Some(registry),
            } => {
                format!("{name} = {{ version = \"{version}\", registry-index = \"{registry}\" }}")
            }
            Self {
                name,
                version,
                registry: None,
                registry_index: None,
            } => {
                format!("{name} = \"{version}\"")
            }
            Self {
                name: _,
                version: _,
                registry: Some(_),
                registry_index: Some(_),
            } => {
                unimplemented!("cannot set bot registry and registry-index")
            }
        }
    }
}

struct Patches {
    patches: HashMap<String, Vec<Patch>>,
}

impl Patches {
    pub fn new() -> Self {
        Self {
            patches: HashMap::new(),
        }
    }

    fn add(&mut self, registry: &str, patch: Patch) {
        self.patches
            .entry(registry.to_owned())
            .or_insert_with(Vec::new)
            .push(patch);
    }

    fn render(self) -> String {
        let Self { patches } = self;

        let mut f = String::new();
        for (registry, patches) in patches {
            writeln!(f, "[patch.{registry}]", registry = registry).unwrap();
            for patch in patches {
                writeln!(f, "{}", patch.render()).unwrap();
            }
        }
        f
    }
}

pub enum PatchType {
    Git(GitPatch),
    Local(LocalPatch),
}

impl PatchType {
    fn render(&self) -> String {
        match self {
            Self::Git(git) => git.render(),
            Self::Local(local) => local.render(),
        }
    }
}

pub struct Patch {
    name: String,
    package: Option<String>,
    r#type: PatchType,
}

impl Patch {
    pub fn new(name: impl AsRef<str>, r#type: PatchType) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            package: None,
            r#type,
        }
    }

    /// https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html#using-patch-with-multiple-versions
    pub fn new_renamed(name: impl AsRef<str>, package: impl AsRef<str>, r#type: PatchType) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            package: Some(package.as_ref().to_owned()),
            r#type,
        }
    }

    fn render(self) -> String {
        let Self {
            name,
            package,
            r#type,
        } = self;

        let mut f = String::new();
        write!(f, "{name} = {{", name = name).unwrap();
        if let Some(package) = package {
            write!(f, " package = \"{package}\",", package = package).unwrap();
        }
        write!(f, " {t} }}", t = r#type.render()).unwrap();
        f
    }
}

pub struct GitPatch {
    url: String,
    branch: Option<String>,
    rev: Option<String>,
}

impl GitPatch {
    pub fn new(
        url: impl AsRef<str>,
        branch: Option<impl AsRef<str>>,
        rev: Option<impl AsRef<str>>,
    ) -> Self {
        Self {
            url: url.as_ref().to_owned(),
            branch: branch.map(|b| b.as_ref().to_owned()),
            rev: rev.map(|r| r.as_ref().to_owned()),
        }
    }

    fn render(&self) -> String {
        let Self { url, branch, rev } = self;

        let mut f = String::new();
        write!(f, "git = \"{url}\"", url = url).unwrap();
        if let Some(branch) = branch {
            write!(f, ", branch = \"{branch}\"", branch = branch).unwrap();
        }
        if let Some(rev) = rev {
            write!(f, ", rev = \"{rev}\"", rev = rev).unwrap();
        }
        f
    }
}

pub struct LocalPatch {
    path: String,
}

impl LocalPatch {
    pub fn new(path: impl AsRef<str>) -> Self {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    fn render(&self) -> String {
        let Self { path } = self;

        format!("path = \"{path}\"", path = path)
    }
}

#[test]
fn manifest_with_deps() {
    let header = Header::basic("package-name");

    let manifest = Manifest::new(header)
        .add_dependency(Dependency::new("rand", "0.8"))
        .add_dependency(Dependency::new("redact", "0.1.10"))
        .render();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [dependencies]
    rand = "0.8"
    redact = "0.1.10"
    '''
    "###);
}

#[test]
fn basic_manifest() {
    let header = Header::basic("package-name");

    let manifest = Manifest::new(header).render();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    '''
    "###);
}

#[test]
fn manifest_with_patches() {
    let header = Header::basic("package-name");

    let manifest = Manifest::new(header)
        .add_patch(
            "crates-io",
            Patch::new(
                "test",
                PatchType::Git(GitPatch::new(
                    "https://github.com/test/test.git",
                    Option::<&str>::None,
                    Option::<&str>::None,
                )),
            ),
        )
        .add_patch(
            "crates-io",
            Patch::new_renamed(
                "test2",
                "test",
                PatchType::Git(GitPatch::new(
                    "https://github.com/test/test2.git",
                    Some("main"),
                    Some("324hb34"),
                )),
            ),
        )
        .add_patch(
            "crates-io",
            Patch::new_renamed(
                "test3",
                "test",
                PatchType::Local(LocalPatch::new("/path/to/local/crate/test3")),
            ),
        )
        .render();

    insta::assert_toml_snapshot!(manifest, @r###"
    '''
    [package]
    name = "package-name"
    version = "0.1.0"
    edition = "2021"

    # See more keys and their definitions at https://doc.rust-lang.org/cargo/reference/manifest.html

    [patch.crates-io]
    test = { git = "https://github.com/test/test.git" }
    test2 = { package = "test", git = "https://github.com/test/test2.git", branch = "main", rev = "324hb34" }
    test3 = { package = "test", path = "/path/to/local/crate/test3" }
    '''
    "###);
}
