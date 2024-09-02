// MIT License
// Copyright (c) 2024 Graham King

use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path;
use std::process;

use anyhow::Context;
use rusqlite::OptionalExtension;

use super::article::Article;
use super::front_matter::FrontMatter;

mod db;

const MIN_SIMILARITY: f64 = 0.4;

#[derive(clap::Subcommand)]
pub enum Commands {
    /// 1. Parse markdown articles, chunk them, and store in sqlite db
    Gather {
        /// The directory to embed
        directory: String,
    },

    /// 2. Call OpenAI's text-embedding-3-small for each chunk, store in db.
    ///    This part costs money (my whole blog costs less than $0.01) and requires
    ///    an OpenAI API key in environment variable OPENAI_API_KEY
    Embed,

    /// 3. Iterate all the articles comparing them pair-wise and store the results in db
    Calc,

    /// 4. Write a list of related articles to the front-matter of each of your blog posts.
    ///    Backup your files first!
    Write {
        /// The directory to embed
        directory: String,
        /// Do no backup the file as a .BAK
        #[clap(long)]
        no_backup: bool,
        /// Don't actually change anything, print the changes to stdout
        #[clap(long)]
        dry_run: bool,
    },

    /// Delete before pushing
    FixUp,
}

pub fn run(db_path: &str, cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Gather { directory } => do_gather(db_path, &directory),
        Commands::Embed => do_embed(db_path),
        Commands::Calc => do_calc(db_path),
        Commands::Write {
            directory,
            no_backup,
            dry_run,
        } => do_write(db_path, &directory, dry_run, !no_backup),
        Commands::FixUp => do_fixup(db_path),
    }
}

fn do_gather(db_path: &str, dir: &str) -> anyhow::Result<()> {
    let db_conn = rusqlite::Connection::open(db_path)?;
    db_conn.execute(db::CREATE_ARTICLE_TABLE, ())?;
    db_conn.execute(db::CREATE_CHUNK_TABLE, ())?;

    let posts: Vec<fs::DirEntry> = fs::read_dir(dir)?.map(|x| x.unwrap()).collect();
    println!("Gathering {} posts from {dir} into {db_path}", posts.len());

    // This is so fast we don't need to show progress
    for entry in posts.into_iter() {
        let filepath = entry.path();
        gather_file(&db_conn, &filepath)?;
    }

    Ok(())
}

fn do_embed(db_path: &str) -> anyhow::Result<()> {
    let Ok(api_key) = env::var("OPENAI_API_KEY") else {
        eprintln!("Set variable OPENAI_KEY to your key");
        process::exit(2);
    };
    let mut db_conn = rusqlite::Connection::open(db_path)?;

    let articles = load_all_active_articles(&db_conn)?;
    let total = articles.len();
    println!("Embedding {total} non-draft articles");

    let width = get_terminal_width();
    let mut stdout = io::stdout();
    for (idx, article) in articles.into_iter().enumerate() {
        let progress = format!("{} / {total}", idx + 1);
        let spaces = " ".repeat(width - (article.title.len() + progress.len() + 2));
        write!(stdout, "\r[{}{spaces}{progress}]", article.title)?;
        stdout.flush()?;

        let tx = db_conn.transaction()?;
        let mut stmt = tx.prepare(
            "UPDATE article_chunk SET embed = ?1 WHERE chunk_id = ?2 AND article_id = ?3",
        )?;
        let chunks = load_embed_chunks(&tx, article.id)?;
        for (chunk_id, text, current_embed) in chunks {
            if !current_embed.is_empty() {
                // embeds cost money, don't recalculate existing ones
                // this means if the text changes need to edit db to force this
                continue;
            }
            let embed = super::openai::embed(&api_key, &text)?;
            stmt.execute((f64_vec_to_u8_vec(embed), chunk_id, article.id))?;
        }
        stmt.finalize()?;
        tx.commit()?;
    }
    println!();
    Ok(())
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() {
        panic!("Vectors a and b must be of the same length");
    }

    let dot_product: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();

    let magnitude_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let magnitude_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

    dot_product / (magnitude_a * magnitude_b)
}

fn do_calc(db_path: &str) -> anyhow::Result<()> {
    let mut db_conn = rusqlite::Connection::open(db_path)?;
    db_conn.execute(db::CREATE_SIMILARITY_TABLE, ())?;

    let articles = load_all_active_articles(&db_conn)?;
    println!(
        "Calculating similarity for {} non-draft articles",
        articles.len()
    );

    let mut count = 0;
    for (idx, a) in articles.iter().enumerate() {
        // Do one article at a time
        let tx = db_conn.transaction()?;
        let mut stmt = tx.prepare(
            r#"INSERT INTO article_similiarity (article_a, article_b, similarity)
               VALUES (?1, ?2, ?3)
               ON CONFLICT DO UPDATE SET similarity = excluded.similarity
            "#,
        )?;
        for b in articles.iter().skip(idx + 1) {
            let similarity = compare_articles(&tx, a, b)?;
            stmt.execute((a.id, b.id, similarity))?;
            println!(
                "{count}: {} {} -> {similarity}",
                a.filename.file_name().unwrap().to_string_lossy(),
                b.filename.file_name().unwrap().to_string_lossy()
            );
            count += 1;
        }
        stmt.finalize()?;
        tx.commit()?;
    }

    Ok(())
}

fn do_write(
    db_path: &str,
    directory: &str,
    is_dry_run: bool,
    is_backup: bool,
) -> anyhow::Result<()> {
    let db_conn = rusqlite::Connection::open(db_path)?;
    let articles = load_all_active_articles(&db_conn)?;
    let dir = path::PathBuf::from(directory);
    println!(
        "Calculating similar articles for {} non-draft posts in {directory}",
        articles.len()
    );
    let width = if is_dry_run { get_terminal_width() } else { 0 };

    let mut stmt_first = db_conn.prepare(
        r#"
        SELECT a.filename, s.similarity
        FROM article_similiarity s, article a
        WHERE NOT a.is_draft
         AND ((s.article_a = ?1 AND s.article_b = a.id) OR (s.article_a = a.id AND s.article_b = ?1))
        ORDER BY s.similarity DESC
        LIMIT 3"#,
    )?;

    let mut written_count = 0;
    for article in articles {
        let mut related = Vec::new();
        let results = stmt_first.query_map([article.id], |row| {
            let filename: String = row.get(0)?;
            let similarity: f64 = row.get(1)?;
            Ok((filename, similarity))
        })?;
        for (filename, similarity) in results.map(|x| x.unwrap()) {
            if similarity < MIN_SIMILARITY {
                continue;
            }
            let p = path::PathBuf::from(filename);
            let os_name = p.file_name().unwrap();
            related.push(os_name.to_string_lossy().to_string());
        }

        if related.is_empty() {
            // No other articles are similar enough
            continue;
        }

        let full_path = dir.join(&article.filename);
        let contents =
            fs::read_to_string(&full_path).with_context(|| format!("{}", full_path.display()))?;
        let (mut fm, fm_size) = FrontMatter::extract(&contents)?;
        if !fm.related.is_empty() {
            // Don't overwrite existing related articles
            continue;
        }
        let body: String = contents
            .lines()
            .skip(fm_size + 2) // Add the two dashes lines we must also skip
            .collect::<Vec<&str>>()
            .join("\n");

        fm.related = related;
        let y = serde_yaml::to_string(&fm)?;

        let mut writer: Box<dyn io::Write> = if is_dry_run {
            let article_changed = article.filename.file_name().unwrap().to_string_lossy();
            let spaces = "+".repeat((width - (article_changed.len() + 2)) / 2);
            println!("\n\n{spaces} {article_changed} {spaces}");
            Box::new(io::stdout())
        } else if is_backup {
            let mut bak = full_path.clone();
            bak.set_extension("BAK");
            fs::rename(&full_path, bak)?;
            Box::new(File::create_new(&full_path)?)
        } else {
            Box::new(File::create(&full_path)?)
        };
        writeln!(writer, "---")?;
        write!(writer, "{y}")?;
        writeln!(writer, "---")?;
        write!(writer, "{body}")?;

        written_count += 1;
    }
    println!("\nUpdated {written_count} posts");

    Ok(())
}

// Does not include draft articles
fn load_all_active_articles(db_conn: &rusqlite::Connection) -> anyhow::Result<Vec<Article>> {
    let mut stmt = db_conn
        .prepare("select id, title, url, date, filename, is_draft from article order by id")?;
    let article_iter = stmt.query_map((), |row| {
        // Convert each row into an Article instance
        let id: usize = row.get(0)?;
        let title: String = row.get(1)?;
        let url: String = row.get(2)?;
        let date: Option<String> = row.get(3)?; // Dates are stored as strings in SQLite
        let filename: String = row.get(4)?;
        let is_draft: bool = row.get(5)?;

        // Attempt to parse the date if it exists
        let date = date
            .as_deref()
            .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok());

        Ok(Article {
            id,
            title,
            url,
            date,
            filename: path::PathBuf::from(filename),
            is_draft,
            chunks: vec![],
        })
    })?;

    // Collect all Article instances into a vector and return
    // Skip draft articles
    let mut articles = Vec::new();
    for article in article_iter.filter(|a| !a.as_ref().unwrap().is_draft) {
        articles.push(article?);
    }
    Ok(articles)
}

fn compare_articles(
    db_conn: &rusqlite::Connection,
    a: &Article,
    b: &Article,
) -> anyhow::Result<f64> {
    let a_chunks = load_embed_chunks(db_conn, a.id)?;
    let b_chunks = load_embed_chunks(db_conn, b.id)?;
    let mut simis = Vec::new();
    for (_, _, a_embedding) in a_chunks.into_iter() {
        for (_, _, b_embedding) in b_chunks.iter() {
            let v = cosine_similarity(&a_embedding, b_embedding);
            simis.push(v);
        }
    }
    Ok(simis.iter().sum::<f64>() / simis.len() as f64)
}

fn load_embed_chunks(
    db_conn: &rusqlite::Connection,
    article_id: usize,
) -> anyhow::Result<Vec<(usize, String, Vec<f64>)>> {
    let mut out = Vec::new();
    let mut stmt =
        db_conn.prepare("SELECT chunk_id, text, embed FROM article_chunk WHERE article_id = ?1")?;
    let mut rows = stmt.query(rusqlite::params![article_id])?;
    while let Some(row) = rows.next()? {
        let chunk_id: usize = row.get(0)?;
        let text: String = row.get(1)?;
        let blob: Option<Vec<u8>> = row.get(2)?;
        out.push((
            chunk_id,
            text,
            blob.map(u8_vec_to_f64_vec).unwrap_or_default(),
        ));
    }
    Ok(out)
}

// - Parse the post metadata as yaml
// - Insert it into article table
// - Calculate the chunks
// - Insert them into article_chunk
fn gather_file(db_conn: &rusqlite::Connection, filepath: &path::Path) -> anyhow::Result<Article> {
    let contents = fs::read_to_string(filepath)?;
    let article = Article::parse(filepath, &contents)?;
    let mut stmt = db_conn.prepare(
        r#"INSERT INTO article (filename, title, url, date, is_draft)
        VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(filename) DO UPDATE SET
            title = excluded.title, url = excluded.url, date = excluded.date
        RETURNING (id)"#,
    )?;
    let id = stmt
        .query_row(
            (
                filepath.file_name().unwrap().to_string_lossy(),
                &article.title,
                &article.url,
                article.date,
                article.is_draft,
            ),
            |row| row.get::<_, usize>(0),
        )
        .optional()
        .with_context(|| format!("filename={}", filepath.display()))?;
    let article_id = id.unwrap();

    // If the chunk text hasn't changed skip it
    let mut exists_stmt = db_conn
        .prepare("SELECT chunk_id FROM article_chunk WHERE article_id = ?1 AND text = ?2")?;
    let mut chunk_stmt = db_conn
        .prepare("INSERT INTO article_chunk (article_id, chunk_id, text) VALUES (?1, ?2, ?3) ON CONFLICT(article_id, chunk_id) DO UPDATE SET text = excluded.text")?;
    for (idx, c) in article.chunks.iter().enumerate() {
        let maybe_chunk_id = exists_stmt
            .query_row((article_id, c), |row| row.get::<_, usize>(0))
            .optional()?;
        if maybe_chunk_id.is_none() {
            chunk_stmt.execute((article_id, idx, c))?;
        }
    }
    Ok(article)
}

fn f64_vec_to_u8_vec(vec: Vec<f64>) -> Vec<u8> {
    let mut u8_vec: Vec<u8> = Vec::with_capacity(vec.len() * std::mem::size_of::<f64>());
    for num in vec {
        u8_vec.extend_from_slice(&num.to_ne_bytes());
    }
    u8_vec
}

fn u8_vec_to_f64_vec(vec: Vec<u8>) -> Vec<f64> {
    assert_eq!(vec.len() % std::mem::size_of::<f64>(), 0);
    let mut f64_vec: Vec<f64> = Vec::with_capacity(vec.len() / std::mem::size_of::<f64>());
    for chunk in vec.chunks_exact(std::mem::size_of::<f64>()) {
        let num = f64::from_ne_bytes(chunk.try_into().expect("slice with incorrect length"));
        f64_vec.push(num);
    }
    f64_vec
}

fn do_fixup(_db_path: &str) -> anyhow::Result<()> {
    let dir = "/home/graham/src/darkcoding/content/posts";
    for entry in fs::read_dir(dir)? {
        let filepath = entry?.path();
        let s = fs::read_to_string(&filepath)?;
        let a = Article::parse(&filepath, &s)?;
        println!("{a:?}");
    }

    Ok(())
}

#[repr(C)]
struct Winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

fn get_terminal_width() -> usize {
    let mut winsize: Winsize = unsafe { std::mem::zeroed() };
    let fd = 0; // standard input
    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut winsize) } == -1 {
        eprintln!("Error getting terminal size");
        std::process::exit(1);
    }
    winsize.ws_col as usize
}
