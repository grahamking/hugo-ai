// MIT License
// Copyright (c) 2024 Graham King

use anyhow::Context;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::process;

use crate::front_matter::FrontMatter;

pub fn run(dir: &str, is_backup: bool) -> anyhow::Result<()> {
    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        eprintln!("Set variable OPENAI_KEY to your key");
        process::exit(2);
    };

    let posts: Vec<fs::DirEntry> = fs::read_dir(dir)?.map(|x| x.unwrap()).collect();
    println!("Summarizing {} posts", posts.len());

    let mut written_count = 0;
    for entry in posts.into_iter() {
        let filepath = entry.path();
        let s = fs::read_to_string(&filepath)?;
        let (mut fm, fm_size) = FrontMatter::extract(&s)?;
        if fm.draft || fm.synopsis.is_some() {
            // Don't summarize draft as they will change
            // Skip if it already has a summary
            continue;
        }
        let body: String = s
            .lines()
            .skip(fm_size + 2) // Add the two dashes lines we must also skip
            .collect::<Vec<&str>>()
            .join("\n");
        if body.len() < 1000 {
            // Too short to be worth summarizing
            continue;
        }

        let summary = super::openai::summarize(&api_key, &body)
            .context(filepath.display().to_string().clone())?;

        fm.synopsis = Some(summary);

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
