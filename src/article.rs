// MIT License
// Copyright (c) 2024 Graham King

use std::path;

use crate::front_matter::FrontMatter;

const CHUNK_SIZE: usize = 2000;
const MIN_CHUNK: usize = 2500;

#[derive(Debug)]
pub struct Article {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub date: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub filename: path::PathBuf,
    pub is_draft: bool,
    pub chunks: Vec<String>,
}

impl Article {
    pub fn parse(filepath: &path::Path, s: &str) -> anyhow::Result<Article> {
        let (fm, fm_size) = FrontMatter::extract(s)?;

        let header = [fm.title.clone(), fm.date.clone()];

        // Now gather the body into CHUNK_SIZE chunks

        let mut body: String = s
            .lines()
            .skip(fm_size + 2) // Add the two dashes lines we must also skip
            .collect::<Vec<&str>>()
            .join("\n");
        let mut chunks = Vec::new();
        while body.len() > MIN_CHUNK {
            let mut split_pos = CHUNK_SIZE;
            while split_pos < body.len() && body.as_bytes()[split_pos] != b' ' {
                split_pos += 1;
            }
            let rest = body.split_off(split_pos);
            let mut embed_unit = header.join("\n");
            embed_unit.push_str("\n\n");
            embed_unit.push_str(&body);
            chunks.push(embed_unit);
            body = rest;
        }

        // Add the title and date to each chunk
        // I figure it helps the embedding

        let mut embed_unit = header.join("\n");
        embed_unit.push_str(&body);
        chunks.push(embed_unit);

        let mut article: Article = fm.into();
        article.chunks = chunks;
        article.filename = filepath.to_path_buf();
        Ok(article)
    }
}
