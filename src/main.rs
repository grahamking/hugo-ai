// MIT License
// Copyright (c) 2024 Graham King

use clap::{Parser, Subcommand, ValueEnum};
use std::env;
use std::fs;
use std::process;

mod article;
mod claude;
mod field;
mod front_matter;
mod openai;
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
    Tagline {
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

#[derive(Clone, Copy, Debug)]
struct Prompts {
    system: &'static str,
    user: &'static str,
}

const SUMMARIZE_PROMPTS: Prompts = Prompts{
    system: "Respond in the first-person as if you are the author. Never refer to the blog post directly.",
    user: "Re-write this as a single short concise paragraph, using an active voice. Be direct. Only cover the key points.",
};

const TAGLINE_PROMPTS: Prompts = Prompts {
    system: "Use the past tense",
    //user: "Write a tagline for this blog post. Try to make it witty, funny, light hearted. Answer with only the tagline. Answer in a single short sentence.",
    user: "Write a tagline for this blog post. Answer with only the tagline. Answer in a single short sentence.",
};

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
        } => field::run(
            &directory,
            model,
            !no_backup,
            "synopsis",
            SUMMARIZE_PROMPTS,
            1000,
        ),
        Commands::Tagline {
            directory,
            no_backup,
            model,
        } => field::run(
            &directory,
            model,
            !no_backup,
            "tagline",
            TAGLINE_PROMPTS,
            1000,
        ),
    }
}
