#![recursion_limit = "256"]

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::env;

mod api;

#[derive(Subcommand)]
enum Commands {
    List,
    Ask { id: String, message: String },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    env::set_var("CURL_IMPERSONATE", "ff91esr");

    let Cli { command } = Cli::parse();

    let token = env::var("CHATGPT_TOKEN")?;
    let cookie = env::var("CHATGPT_COOKIE")?;

    match command {
        Commands::List => {
            let conversations = api::conversations(&token, &cookie)?;

            for conversation in conversations.items {
                println!("{} - {}", conversation.title, conversation.id);
            }
        }
        Commands::Ask { id, message } => {
            api::conversation(&token, &cookie, &id, &message)?;
        }
    }

    Ok(())
}
