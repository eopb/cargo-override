use anyhow::{bail, Context};

pub fn patch_manifest(
    manifest: &str,
    name: &str,
    registry: &str,
    path: &str,
) -> anyhow::Result<String> {
    let mut manifest: toml_edit::DocumentMut = manifest
        .parse()
        .context("patch manifest contains invalid toml")?;

    let manifest_table = manifest.as_table_mut();

    let patch_table = create_subtable(manifest_table, "patch", true)?;

    let registry_table = create_subtable(patch_table, &registry, false)?;

    let Ok(new_patch) = format!("{{ path = \"{}\" }}", path).parse::<toml_edit::Item>() else {
        todo!("We haven't escaped the path so we can't be sure this will parse")
    };

    toml_edit::Table::insert(registry_table, name, new_patch);

    Ok(manifest.to_string())
}

fn create_subtable<'a>(
    table: &'a mut toml_edit::Table,
    name: &str,
    dotted: bool,
) -> anyhow::Result<&'a mut toml_edit::Table> {
    let existing = &mut table[name];

    if existing.is_none() {
        // If the table does not exist, create it
        *existing = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // TODO: in the future we may be able to do cool things with miette
    let _span = existing.span();

    let Some(subtable) = existing.as_table_mut() else {
        bail!("{name} already exists but is not a table")
    };

    subtable.set_dotted(dotted);

    Ok(subtable)
}
