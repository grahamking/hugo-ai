// MIT License
// Copyright (c) 2024 Graham King

use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::process;

mod similar;

const DB_NAME: &str = "hugo-ai.db";
const CFG_DIR: &str = ".config/hugo-ai";

#[derive(Parser)]
struct Cli {
    /// Sets a custom database path
    #[arg(long, value_name = "PATH")]
    db_path: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Similar {
        #[clap(subcommand)]
        subcommand: similar::Commands,
    }, // Summarize
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let db_path = match cli.db_path {
        Some(db) => db,
        None => {
            let Ok(user_home) = env::var("HOME") else {
                eprintln!("$HOME not set");
                process::exit(1);
            };
            let cfg_dir = format!("{user_home}/{CFG_DIR}");
            fs::create_dir_all(&cfg_dir)?;
            format!("{cfg_dir}/{DB_NAME}")
        }
    };
    match cli.command {
        Commands::Similar { subcommand } => similar::run(&db_path, subcommand),
    }
}
