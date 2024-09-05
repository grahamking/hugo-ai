// MIT License
// Copyright (c) 2024 Graham King

use anyhow::Context;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io;

use crate::claude;
use crate::front_matter::FrontMatter;
use crate::openai;

/// Fill a meta-data/front-matter field on each blog post using a set of prompts and a model
pub fn run(
    // The directory to look for Hugo Markdown posts in
    dir: &str,
    // The magic
    model: super::ModelChoice,
    // If true backup the file to a .BAK
    is_backup: bool,
    // The name of the front-matter field to populate
    field_name: &'static str,
    // System and user prompts to send to the model
    prompts: super::Prompts,
    // Ignore posts shorter than this
    min_len: usize,
) -> anyhow::Result<()> {
    let posts: Vec<fs::DirEntry> = fs::read_dir(dir)?.map(|x| x.unwrap()).collect();
    println!("Processing {} posts", posts.len());

    let mut written_count = 0;
    for entry in posts.into_iter() {
        let filepath = entry.path();
        let s = fs::read_to_string(&filepath)?;
        let front_matter_vec = FrontMatter::select(&s);
        let mut fm: HashMap<String, serde_yaml::Value> =
            serde_yaml::from_str(&front_matter_vec.join("\n"))
                .context(filepath.display().to_string())?;
        if matches!(fm.get("draft"), Some(serde_yaml::Value::Bool(true))) {
            // Don't process drafts as they will change
            continue;
        }
        if fm.contains_key(field_name) {
            // Skip if it already has one
            continue;
        }

        let body: String = s
            .lines()
            .skip(front_matter_vec.len() + 2) // Add the two dashes lines we must also skip
            .collect::<Vec<&str>>()
            .join("\n");
        if body.len() < min_len {
            // Too short to be interesting
            continue;
        }

        use super::ModelChoice::*;
        let maybe = match model {
            Gpt4o => openai::message(openai::CHAT_MODEL_BIG, &body, prompts),
            Gpt4oMini => openai::message(openai::CHAT_MODEL_SMALL, &body, prompts),
            Claude35Sonnet => claude::message(claude::CHAT_MODEL_BIG, &body, prompts),
            Claude3Haiku => claude::message(claude::CHAT_MODEL_SMALL, &body, prompts),
        };
        let field_value = maybe.context(filepath.display().to_string())?;

        fm.insert(
            field_name.to_string(),
            serde_yaml::Value::String(field_value),
        );

        let y = serde_yaml::to_string(&fm)?;
        let mut writer: Box<dyn io::Write> = if is_backup {
            let mut bak = filepath.clone();
            bak.set_extension("BAK");
            fs::rename(&filepath, bak)?;
            Box::new(File::create_new(&filepath)?)
        } else {
            Box::new(File::create(&filepath)?)
        };
        writeln!(writer, "---")?;
        write!(writer, "{y}")?;
        writeln!(writer, "---")?;
        write!(writer, "{body}")?;

        written_count += 1;
        println!("Processed: {}", filepath.display());
    }

    println!("\nUpdated {written_count} posts");
    Ok(())
}
