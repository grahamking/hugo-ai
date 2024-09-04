// MIT License
// Copyright (c) 2024 Graham King

use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::fs;
use std::process;

mod article;
mod claude;
mod front_matter;
mod openai;
mod similar;
mod summary;

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
    },
    Summary {
        /// The directory with the markdown files
        directory: String,

        /// Do no backup the file as a .BAK
        #[clap(long)]
        no_backup: bool,

        /// Use big model (gpt-4o or claude-3.5-sonnet) or
        /// small model (gpt-4o-mini or claude-3-haiku)
        #[clap(long)]
        model: ModelChoice,
    },
}

#[derive(Default, Clone, Copy, ValueEnum)]
enum ModelChoice {
    #[default]
    Gpt4o,
    Gpt4oMini,
    Claude35Sonnet,
    Claude3Haiku,
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
        Commands::Summary {
            directory,
            no_backup,
            model,
        } => summary::run(&directory, model, !no_backup),
    }
}
