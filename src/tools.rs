use std::fs;
use std::process::Command;

pub fn write_to_file(path: &str, content: &str) -> anyhow::Result<()> {
    fs::write(path, content)?;
    Ok(())
}

pub fn read_file(path: &str) -> anyhow::Result<String> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

pub fn run_shell_command(command: &str) -> anyhow::Result<String> {
    let output = Command::new("sh").arg("-c").arg(command).output()?;
    let result = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).to_string();
        anyhow::bail!("Command failed: {}. Stderr: {}", result, error);
    }
    Ok(result)
}
