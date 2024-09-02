// MIT License
// Copyright (c) 2024 Graham King

use std::path;

use crate::article::Article;

// Metadata at the top of a Hugo post
#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct FrontMatter {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(
        rename = "showSummary",
        default,
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub show_summary: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub draft: bool,

    // This is the field we set
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
}

impl From<FrontMatter> for Article {
    fn from(fm: FrontMatter) -> Self {
        Article {
            id: 0, // we don't know yet
            title: fm.title,
            url: fm.url.unwrap_or_default(),
            date: chrono::DateTime::parse_from_rfc3339(&fm.date).ok(),
            filename: path::PathBuf::new(),
            is_draft: fm.draft,
            chunks: vec![],
        }
    }
}

impl FrontMatter {
    // Extract the front matter, the part between the dashes
    // It's valid yaml
    pub fn extract(s: &str) -> anyhow::Result<(FrontMatter, usize)> {
        let line_iter = s.lines().skip(1); // skip first "---" line
        let front_matter_vec = line_iter
            .take_while(|line| !line.starts_with("---"))
            .collect::<Vec<&str>>();

        let fm: FrontMatter = serde_yaml::from_str(&front_matter_vec.join("\n"))?;
        Ok((fm, front_matter_vec.len()))
    }
}
