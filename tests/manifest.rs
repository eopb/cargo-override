//! Builders for generating manifest files for tests

use std::fmt::Write as _;

pub struct Manifest {
    header: Header,
    dependencies: Dependencies,
    target: Option<Target>,
}

impl Manifest {
    pub fn new(header: Header) -> Self {
        Self {
            header,
            dependencies: Dependencies::new(),
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

    pub fn render(self) -> String {
        let Self {
            header,
            dependencies,
            target,
        } = self;

        let mut w = String::new();

        writeln!(w, "{}", header.render()).unwrap();
        write!(w, "{}", dependencies.render()).unwrap();
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
