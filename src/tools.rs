use std::fs;

pub fn write_to_file(path: &str, content: &str) -> anyhow::Result<()> {
    fs::write(path, content)?;
    Ok(())
}
