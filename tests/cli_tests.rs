use OxideAgent::cli::{AgentType, InterfaceType};
use clap::Parser;

#[test]
fn test_agent_type_models() {
    let available_models = vec![
        "qwen:latest".to_string(),
        "llama3:latest".to_string(),
        "granite:latest".to_string(),
    ];

    assert_eq!(AgentType::Qwen.model(&available_models), "qwen:latest");
    assert_eq!(AgentType::Llama.model(&available_models), "llama3:latest");
    assert_eq!(
        AgentType::Granite.model(&available_models),
        "granite:latest"
    );
}

#[test]
fn test_agent_type_models_fallback_to_first() {
    // Test when the specific model is not found but there is a first model
    let available_models = vec!["other:latest".to_string(), "another:latest".to_string()];

    assert_eq!(AgentType::Qwen.model(&available_models), "other:latest");
    assert_eq!(AgentType::Llama.model(&available_models), "other:latest");
    assert_eq!(AgentType::Granite.model(&available_models), "other:latest");
}

#[test]
fn test_agent_type_models_fallback_to_empty() {
    // Test when no models are available at all
    let available_models: Vec<String> = vec![];

    assert_eq!(AgentType::Qwen.model(&available_models), "");
    assert_eq!(AgentType::Llama.model(&available_models), "");
    assert_eq!(AgentType::Granite.model(&available_models), "");
}

#[test]
fn test_agent_type_names() {
    assert_eq!(AgentType::Qwen.name(), "Qwen");
    assert_eq!(AgentType::Llama.name(), "Llama");
    assert_eq!(AgentType::Granite.name(), "Granite");
}

#[test]
fn test_agent_type_system_prompts() {
    assert_eq!(
        AgentType::Qwen.system_prompt(),
        "You are a Rust programming expert."
    );
    assert_eq!(
        AgentType::Llama.system_prompt(),
        "You are a helpful assistant."
    );
    assert_eq!(
        AgentType::Granite.system_prompt(),
        "You are a helpful assistant."
    );
}

#[test]
fn test_cli_parsing() {
    let args = vec!["oxide-agent", "--agent", "qwen"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.agent, Some(AgentType::Qwen));
    }
}

#[test]
fn test_cli_session_arg() {
    let args = vec!["oxide-agent", "--session", "test_session"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.session, Some("test_session".to_string()));
    }
}

#[test]
fn test_cli_list_sessions_arg() {
    let args = vec!["oxide-agent", "--list-sessions"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.list_sessions, Some(true));
    }
}

#[test]
fn test_cli_no_stream_arg() {
    let args = vec!["oxide-agent", "--no-stream"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.no_stream, Some(true));
    }
}

#[test]
fn test_cli_interface_arg() {
    let args = vec!["oxide-agent", "--interface", "tui"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.interface, Some(InterfaceType::Tui));
    }
}

#[test]
fn test_cli_mcp_server_arg() {
    let args = vec!["oxide-agent", "--mcp-server", "http://localhost:8080"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(
            cli_args.mcp_server,
            Some("http://localhost:8080".to_string())
        );
    }
}

#[test]
fn test_cli_mcp_auth_token_arg() {
    let args = vec!["oxide-agent", "--mcp-auth-token", "test_token"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.mcp_auth_token, Some("test_token".to_string()));
    }
}

#[test]
fn test_cli_config_arg() {
    let args = vec!["oxide-agent", "--config", "config.toml"];
    let parsed = std::panic::catch_unwind(|| OxideAgent::cli::Args::try_parse_from(args));

    assert!(parsed.is_ok());

    if let Ok(Ok(cli_args)) = parsed {
        assert_eq!(cli_args.config, Some("config.toml".to_string()));
    }
}
