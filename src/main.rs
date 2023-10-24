#![recursion_limit = "256"]

use anyhow::Result;
use api::{COOKIE, TOKEN};
use clap::{Parser, Subcommand};
use std::env::{set_var, var};

mod api;
mod util;

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
    let Cli { command } = Cli::parse();

    COOKIE.get_or_init(|| var("CHATGPT_COOKIE").expect("The environment variable is not set."));
    TOKEN.get_or_init(|| var("CHATGPT_TOKEN").expect("The environment variable is not set."));
    set_var("CURL_IMPERSONATE", "ff91esr");

    match command {
        Commands::List => {
            let conversations = api::conversations()?;

            for conversation in conversations.items {
                println!("{} - {}", conversation.title, conversation.id);
            }
        }
        Commands::Ask { id, message } => {
            api::conversation(&id, &message)?;
        }
    }

    Ok(())
}
