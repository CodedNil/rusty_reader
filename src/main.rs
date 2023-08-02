use feed_rs::parser;
use reqwest::Error;
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::File,
    io::{Cursor, Write},
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Category {
    Entertainment,
    Tech,
    CodeUpdates,
    GameDev,
    Politics,
    Business,
    Science,
    Health,
    Sports,
    Other,
}

impl Category {
    #[must_use]
    pub fn description(&self) -> &str {
        match *self {
            Category::Entertainment => "Movies, music, and entertainment",
            Category::Tech => "Tech articles, gadget reviews, IT news",
            Category::CodeUpdates => "Updates from coding, software projects",
            Category::GameDev => "Game development insights, updates",
            Category::Politics => "Political analysis, government, elections",
            Category::Business => "Business insights, market trends, economic analyses",
            Category::Science => "Discoveries, research, science breakthroughs",
            Category::Health => "Health advice, medical research, nutrition",
            Category::Sports => "Sports updates, scores, news",
            Category::Other => "Content that doesn't fit other categories",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ArticleType {
    Article,
    Video,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
struct Article {
    link: String,
    title: String,
    category: Category,
    article_type: ArticleType,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rss_sources = vec!["https://www.theverge.com/rss/index.xml"];
    let mut articles = HashSet::new();

    for source in rss_sources {
        let response = reqwest::get(source)
            .await
            .expect("Failed to fetch the feed");
        let bytes = response
            .bytes()
            .await
            .expect("Failed to get bytes from the response");
        let cursor = Cursor::new(bytes);
        let feed = parser::parse(cursor).expect("Failed to parse the feed");

        for entry in feed.entries {
            // If entry doesnt have title skip it
            if entry.title.is_none() {
                continue;
            }
            let entry_title = entry.title.unwrap().content;

            println!("Title: {}", entry_title);
            println!("Link: {}", entry.id);
            let entry_summary = entry.summary.map_or_else(|| String::new(), |s| s.content);
            println!("Summary: {}", entry_summary);

            articles.insert(Article {
                link: entry.id,
                title: entry_title,
                category: Category::Tech,
                article_type: ArticleType::Article,
                image: String::new(),
                summary: entry_summary,
                read_status: ReadStatus::Fresh,
            });
        }
    }

    let file = File::create("articles.toml")?;
    let mut writer = std::io::BufWriter::new(file);
    write!(writer, "{}", toml::to_string(&articles)?)?;

    Ok(())
}
