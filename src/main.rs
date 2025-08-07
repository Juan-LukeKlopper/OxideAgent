mod agents;
mod cli;
mod ollama;
mod types;

use agents::Agent;
use clap::Parser;
use cli::Args;
use reqwest::Client;
use tokio;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let client = Client::new();

    let mut agent = Agent::new(args.agent.name(), args.agent.model());

    println!("Chatting with: {}\n", agent.name);

    loop {
        use std::io::{self, Write};
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "/exit" {
            break;
        }

        agent.add_user_message(input);
        agent.chat(&client, !args.no_stream).await?;
    }

    Ok(())
}

