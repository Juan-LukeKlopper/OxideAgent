use OxideAgent::config::{
    AgentType, DiscordInterfaceConfig, InterfaceType, OxideConfig, TelegramInterfaceConfig,
    WebInterfaceConfig, default_api_base, default_model, default_name, default_provider,
    default_system_prompt,
};
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_config_from_json() {
    let json_content = r#"{ 
        "agent": {
            "agent_type": "Qwen",
            "model": "qwen3:4b",
            "name": "Qwen",
            "system_prompt": "You are a Rust programming expert."
        },
        "mcp": {
            "server": null,
            "auth_token": null,
            "tools": []
        },
        "no_stream": false,
        "session": null,
        "list_sessions": false,
        "interface": "Tui",
        "llm": {
            "provider": "ollama",
            "api_base": "",
            "api_key": null,
            "model": "qwen3:4b"
        }
    }"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    // Create a new file with .json extension
    let json_path = path.with_extension("json");
    std::fs::write(&json_path, json_content).unwrap();

    let config = OxideConfig::from_file(&json_path).unwrap();

    assert_eq!(config.agent.agent_type, AgentType::Qwen);
    assert_eq!(config.agent.model, "qwen3:4b");
    assert_eq!(config.agent.name, "Qwen");
    assert_eq!(
        config.agent.system_prompt,
        "You are a Rust programming expert."
    );
    assert_eq!(config.interface, InterfaceType::Tui);
    assert_eq!(config.llm.provider, "ollama");
}

#[test]
fn test_config_from_yaml() {
    let yaml_content = r#"---
agent:
  agent_type: "Llama"
  model: "llama3.2"
  name: "Llama"
  system_prompt: "You are a helpful assistant."
mcp:
  server: ~
  auth_token: ~
  tools: []
no_stream: true
session: "test-session"
list_sessions: false
interface: "Tui"
llm:
  provider: "ollama"
  api_base: ~
  api_key: ~
  model: ~
"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    // Create a new file with .yaml extension
    let yaml_path = path.with_extension("yaml");
    std::fs::write(&yaml_path, yaml_content).unwrap();

    let config = OxideConfig::from_file(&yaml_path).unwrap();

    assert_eq!(config.agent.agent_type, AgentType::Llama);
    assert_eq!(config.agent.model, "llama3.2");
    assert_eq!(config.agent.name, "Llama");
    assert_eq!(config.agent.system_prompt, "You are a helpful assistant.");
    assert!(config.no_stream);
    assert_eq!(config.session, Some("test-session".to_string()));
    assert_eq!(config.interface, InterfaceType::Tui);
}

#[test]
fn test_config_from_toml() {
    let toml_content = r#"no_stream = true
session = "toml-session"
list_sessions = false
interface = "Tui"

[agent]
agent_type = "Granite"
model = "granite:latest"
name = "Granite"
system_prompt = "You are a helpful assistant."

[mcp]
server = "http://localhost:8080"
auth_token = "secret-token"
tools = []

[llm]
provider = "openai"
api_key = "sk-abc123"
model = "gpt-4"
"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    // Create a new file with .toml extension
    let toml_path = path.with_extension("toml");
    std::fs::write(&toml_path, toml_content).unwrap();

    let config = OxideConfig::from_file(&toml_path).unwrap();

    assert_eq!(config.agent.agent_type, AgentType::Granite);
    assert_eq!(config.agent.model, "granite:latest");
    assert_eq!(config.agent.name, "Granite");
    assert!(config.no_stream);
    assert_eq!(config.session, Some("toml-session".to_string()));
    assert_eq!(config.mcp.server, Some("http://localhost:8080".to_string()));
    assert_eq!(config.mcp.auth_token, Some("secret-token".to_string()));
    assert_eq!(config.llm.provider, "openai");
    assert_eq!(config.llm.api_key, Some("sk-abc123".to_string()));
    assert_eq!(config.llm.model, Some("gpt-4".to_string()));
}

#[test]
fn test_config_from_nonexistent_file() {
    let result = OxideConfig::from_file("/nonexistent/path/config.json");
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to read config file")
    );
}

#[test]
fn test_config_from_invalid_json() {
    let invalid_json = r#"{ "invalid": json }"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(invalid_json.as_bytes()).unwrap();
    let path = temp_file.path();

    let result = OxideConfig::from_file(path);
    assert!(result.is_err());
}

#[test]
fn test_config_from_invalid_yaml() {
    let invalid_yaml = r#"invalid: - yaml: [unclosed"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(invalid_yaml.as_bytes()).unwrap();
    let path = temp_file.path();

    let result = OxideConfig::from_file(path);
    assert!(result.is_err());
}

#[test]
fn test_config_from_invalid_toml() {
    let invalid_toml = r#"invalid = toml content with [ unclosed"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(invalid_toml.as_bytes()).unwrap();
    let path = temp_file.path();

    let result = OxideConfig::from_file(path);
    assert!(result.is_err());
}

#[test]
fn test_config_from_unsupported_format() {
    let invalid_ext = r#"{ "some": "json" }"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(invalid_ext.as_bytes()).unwrap();
    // Change the extension to unsupported
    let path = temp_file.path().with_extension("unsupported");
    fs::write(&path, invalid_ext).unwrap();

    let result = OxideConfig::from_file(&path);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Unsupported config format")
    );
}

#[test]
fn test_config_from_no_extension() {
    let json_content = r#"{ 
        "agent": {
            "agent_type": "Qwen",
            "model": "qwen:latest"
        },
        "mcp": {
            "server": null,
            "auth_token": null,
            "tools": []
        },
        "no_stream": false,
        "session": null,
        "list_sessions": false,
        "interface": "Tui",
        "llm": {
            "provider": "ollama"
        }
    }"#;

    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();
    std::fs::write(path, json_content.as_bytes()).unwrap();

    let result = OxideConfig::from_file(path);
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Config file has no extension")
    );
}

#[test]
fn test_default_functions() {
    assert_eq!(default_model(), "");
    assert_eq!(default_name(), "Qwen");
    assert_eq!(
        default_system_prompt(),
        "You are a Rust programming expert."
    );
    assert_eq!(default_provider(), "ollama");
    assert_eq!(default_api_base(), "http://localhost:11434");
}

#[test]
fn test_interface_type_from_cli() {
    use OxideAgent::cli::InterfaceType as CliInterfaceType;

    assert_eq!(
        InterfaceType::from(CliInterfaceType::Tui),
        InterfaceType::Tui
    );
    assert_eq!(
        InterfaceType::from(CliInterfaceType::Web),
        InterfaceType::Web
    );
    assert_eq!(
        InterfaceType::from(CliInterfaceType::Telegram),
        InterfaceType::Telegram
    );
    assert_eq!(
        InterfaceType::from(CliInterfaceType::Discord),
        InterfaceType::Discord
    );
}

#[test]
fn test_config_validation_mcp_with_token() {
    let mut config = OxideConfig::default();
    config.mcp.server = Some("http://localhost:8080".to_string());
    config.mcp.auth_token = Some("token".to_string());
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_mcp_missing_token() {
    let mut config = OxideConfig::default();
    config.mcp.server = Some("http://localhost:8080".to_string());
    config.mcp.auth_token = None;
    assert!(config.validate().is_err());
    assert!(
        config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("MCP server specified but no auth token provided")
    );
}

#[test]
fn test_config_validation_session_with_invalid_chars() {
    let config = OxideConfig {
        session: Some("test/session".to_string()), // Contains invalid slash
        ..Default::default()
    };
    assert!(config.validate().is_err());

    let config = OxideConfig {
        session: Some("test\\session".to_string()), // Contains invalid backslash
        ..Default::default()
    };
    assert!(config.validate().is_err());

    let config = OxideConfig {
        session: Some("test:session".to_string()), // Contains invalid colon
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_empty_session() {
    let config = OxideConfig {
        session: Some("".to_string()),
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_config_interface_type_from_json_variants() {
    let web = r#"{
  "interface": "Web"
}"#;
    let telegram = r#"{
  "interface": "Telegram"
}"#;
    let discord = r#"{
  "interface": "Discord"
}"#;

    let web_config: OxideConfig = serde_json::from_str(web).unwrap();
    let telegram_config: OxideConfig = serde_json::from_str(telegram).unwrap();
    let discord_config: OxideConfig = serde_json::from_str(discord).unwrap();

    assert_eq!(web_config.interface, InterfaceType::Web);
    assert_eq!(telegram_config.interface, InterfaceType::Telegram);
    assert_eq!(discord_config.interface, InterfaceType::Discord);
}

#[test]
fn test_interface_transport_config_defaults_and_validation() {
    let mut config = OxideConfig {
        web: Some(WebInterfaceConfig::default()),
        telegram: Some(TelegramInterfaceConfig {
            bot_token: "telegram-token".to_string(),
            polling_interval_ms: 1000,
            request_timeout_secs: 30,
        }),
        discord: Some(DiscordInterfaceConfig {
            bot_token: "discord-token".to_string(),
            application_id: "123456".to_string(),
            guild_id: None,
        }),
        ..Default::default()
    };

    assert!(config.validate().is_ok());

    config.web = Some(WebInterfaceConfig {
        port: 0,
        ..WebInterfaceConfig::default()
    });
    assert!(config.validate().is_err());
}

#[test]
fn test_transport_config_parses_from_toml() {
    let toml_content = r#"
interface = "Web"

[web]
host = "0.0.0.0"
port = 8088
enable_cors = true
max_payload_bytes = 2048

[telegram]
bot_token = "telegram-token"
polling_interval_ms = 500
request_timeout_secs = 20

[discord]
bot_token = "discord-token"
application_id = "abc123"
guild_id = "guild-1"
"#;

    let config: OxideConfig = toml::from_str(toml_content).unwrap();

    assert_eq!(config.interface, InterfaceType::Web);
    assert_eq!(config.web.unwrap().port, 8088);
    assert_eq!(config.telegram.unwrap().polling_interval_ms, 500);
    assert_eq!(config.discord.unwrap().guild_id.as_deref(), Some("guild-1"));
}
