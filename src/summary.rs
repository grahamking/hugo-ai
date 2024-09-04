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

pub fn run(dir: &str, model: super::ModelChoice, is_backup: bool) -> anyhow::Result<()> {
    let posts: Vec<fs::DirEntry> = fs::read_dir(dir)?.map(|x| x.unwrap()).collect();
    println!("Summarizing {} posts", posts.len());

    let mut written_count = 0;
    for entry in posts.into_iter() {
        let filepath = entry.path();
        let s = fs::read_to_string(&filepath)?;
        let front_matter_vec = FrontMatter::select(&s);
        let mut fm: HashMap<String, serde_yaml::Value> =
            serde_yaml::from_str(&front_matter_vec.join("\n"))?;
        if matches!(fm.get("draft"), Some(serde_yaml::Value::Bool(true))) {
            // Don't summarize drafts as they will change
            continue;
        }
        if fm.contains_key("synopsis") {
            // Skip if it already has a summary
            continue;
        }

        let body: String = s
            .lines()
            .skip(front_matter_vec.len() + 2) // Add the two dashes lines we must also skip
            .collect::<Vec<&str>>()
            .join("\n");
        if body.len() < 1000 {
            // Too short to be worth summarizing
            continue;
        }

        use super::ModelChoice::*;
        let maybe_summary = match model {
            Gpt4o => openai::summarize(openai::CHAT_MODEL_BIG, &body),
            Gpt4oMini => openai::summarize(openai::CHAT_MODEL_SMALL, &body),
            Claude35Sonnet => claude::summarize(claude::CHAT_MODEL_BIG, &body),
            Claude3Haiku => claude::summarize(claude::CHAT_MODEL_SMALL, &body),
        };
        let summary = maybe_summary.context(filepath.display().to_string().clone())?;

        fm.insert("synopsis".to_string(), serde_yaml::Value::String(summary));

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
        println!("Summarized: {}", filepath.display());
    }

    println!("\nUpdated {written_count} posts");
    Ok(())
}
