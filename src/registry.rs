use std::{collections::HashMap, env, ffi::OsString, io, path::PathBuf};

use anyhow::Context;
use cargo::{core::shell::Shell, util::context::GlobalContext};
use winnow::{token::take_until, PResult, Parser};

pub fn get_registry_name_from_url(
    working_dir: PathBuf,
    registry_url: &str,
) -> anyhow::Result<Option<String>> {
    if let Some(registry) = get_registry_from_env(env::vars_os(), registry_url) {
        return Ok(Some(registry));
    }

    let shell = Shell::from_write(Box::new(io::sink()));

    let global_context = GlobalContext::new(shell, working_dir.clone(), working_dir);
    let config_env = global_context
        .env_config()
        .context("failed to get [env] config")?;

    if let Some(registry) = get_registry_from_env(
        config_env.iter().map(|(key, value)| {
            (
                OsString::from(key),
                value.resolve(&global_context).into_owned(),
            )
        }),
        registry_url,
    ) {
        return Ok(Some(registry));
    }

    #[derive(serde::Deserialize)]
    struct Registry {
        index: String,
    }

    let cargo_config_map: Option<HashMap<String, Registry>> = global_context
        .get("registries")
        .context("failed to fetch registries from cargo global context")?;

    if let Some((key, _)) = cargo_config_map
        .into_iter()
        .flatten()
        .find(|(_, Registry { index })| index == registry_url)
    {
        return Ok(Some(key));
    }

    Ok(None)
}

fn get_registry_from_env(
    env: impl Iterator<Item = (OsString, OsString)>,
    url: &str,
) -> Option<String> {
    for (key, value) in env {
        let Ok(key) = key.into_string() else {
            // This env var key is not UTF8, let's ignore it
            continue;
        };
        let Ok(registry) = registry_key(&key) else {
            continue;
        };
        let Ok(registry_url) = value.into_string() else {
            // TODO: we should probably throw a warning here
            continue;
        };
        if registry_url == url {
            return Some(registry.replace('_', "-").to_lowercase());
        }
    }
    None
}

fn registry_key(input: &str) -> PResult<&str> {
    // Format CARGO_REGISTRIES_{REGISTRY_NAME}_INDEX
    let (_, (_, registry)) = ("CARGO_REGISTRIES_", take_until(0.., "_INDEX")).parse_peek(input)?;

    Ok(registry)
}
#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::OsString;

    use googletest::{
        expect_that,
        matchers::{eq, none, some},
    };

    #[googletest::test]
    fn find_registry_from_url() {
        let env = [
            (
                "CARGO_REGISTRIES_FOO_INDEX",
                "https://github.com/rust-lang/crates.io-index",
            ),
            (
                "CARGO_REGISTRIES_FOO_BAR_INDEX",
                "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git",
            ),
            ("CARGO_REGISTRIES_FOO_BAR_PROTOCOL", "sparse"),
            ("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUNNER", "HI"),
        ]
        .map(|(key, value)| (OsString::from(key), OsString::from(value)));
        expect_that!(
            get_registry_from_env(
                env.clone().into_iter(),
                "https://github.com/rust-lang/crates.io-index"
            ),
            some(eq("foo"))
        );
        expect_that!(
            get_registry_from_env(
                env.clone().into_iter(),
                "https://dl.cloudsmith.io/basic/private/registry/cargo/index.git"
            ),
            some(eq("foo-bar"))
        );
        expect_that!(
            get_registry_from_env(
                env.into_iter(),
                "https://github.com/eopb/cargo-override.git"
            ),
            none()
        );
    }
}
