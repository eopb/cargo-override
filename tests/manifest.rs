use std::fmt::Write as _;

/// Builder for project manifests used for testing
pub struct Manifest {
    header: Header,
    dependencies: Dependencies,
    bin: Option<Bin>,
}

impl Manifest {
    pub fn new(header: Header) -> Self {
        Self {
            header,
            dependencies: Dependencies::new(),
            bin: None,
        }
    }

    pub fn add_dependency(mut self, dependency: Dependency) -> Self {
        self.dependencies.add(dependency);
        self
    }

    pub fn add_bin(mut self, bin: Bin) -> Self {
        self.bin = Some(bin);
        self
    }

    pub fn render(self) -> String {
        let Self {
            header,
            dependencies,
            bin,
        } = self;

        let mut w = String::new();

        writeln!(w, "{}", header.render()).unwrap();
        write!(w, "{}", dependencies.render()).unwrap();
        if let Some(bin) = bin {
            write!(w, "{}", bin.render()).unwrap();
        }

        w
    }
}

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
    pub fn _edition(mut self, edition: impl Into<Option<String>>) -> Self {
        self.edition = edition.into();
        self
    }
    pub fn _default_comment(mut self, enable: bool) -> Self {
        self.default_comment = enable;
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

pub struct Bin {
    name: String,
    path: String,
}

impl Bin {
    pub fn new(name: impl AsRef<str>, path: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_owned(),
            path: path.as_ref().to_owned(),
        }
    }

    pub fn render(self) -> String {
        let mut w = String::new();
        writeln!(w).unwrap();
        writeln!(w, "[[bin]]").unwrap();
        writeln!(w, "name = \"{0}\"", self.name).unwrap();
        writeln!(w, "path = \"{0}\"", self.path).unwrap();
        w
    }
}

pub struct Dependency {
    name: String,
    version: String,
    registry: Option<String>,
}

impl Dependency {
    pub fn new(name: impl AsRef<str>, version: impl AsRef<str>) -> Dependency {
        Dependency {
            name: name.as_ref().to_owned(),
            version: version.as_ref().to_owned(),
            registry: None,
        }
    }

    pub fn registry(mut self, name: impl ToString) -> Dependency {
        self.registry = Some(name.to_string());
        self
    }

    fn render(self) -> String {
        let Self {
            name,
            version,
            registry,
        } = self;

        if let Some(registry) = registry {
            format!("{name} = {{ version = \"{version}\", registry = \"{registry}\" }}")
        } else {
            format!("{name} = \"{version}\"")
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
