use atom_syndication::Feed;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::Cursor;

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct Channel {
    pub link: String,
    pub title: String,
    pub icon: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct Article {
    pub link: String,
    pub channel: Channel,
    pub title: String,
    pub published: String,
    pub image: String,
    pub summary: String,
    pub read_status: ReadStatus,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct ArticleServe {
    pub fresh: Vec<Article>,
    pub saved: Vec<Article>,
    pub archived: Vec<Article>,
}

/// Get articles and write them to the database
pub async fn pull_articles() {
    let rss_sources = vec!["https://www.theverge.com/rss/index.xml"];
    // http://feeds.bbci.co.uk/news/technology/rss.xml  https://hnrss.org/frontpage
    let db = sled::open("database").expect("Failed to open the database");

    for source in rss_sources {
        match process_source(source).await {
            Ok(new_articles) => {
                for article in new_articles {
                    let key = article.link.clone();
                    let value = serde_json::to_vec(&article).unwrap();
                    db.insert(key, value).unwrap();
                }
            }
            Err(e) => {
                eprintln!("Failed to process source {source}: {e}");
            }
        }
    }

    if let Err(e) = db.flush() {
        eprintln!("Failed to flush the database: {e}");
    }
}

async fn process_source(source: &str) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    let response = reqwest::get(source).await?;
    let bytes = response.bytes().await?;
    let cursor = Cursor::new(bytes);
    let feed = Feed::read_from(cursor).unwrap();

    let mut articles = Vec::new();

    let channel = Channel {
        link: feed.id,
        title: feed.title.value,
        icon: feed.icon.unwrap_or_default(),
    };

    for entry in feed.entries {
        let entry_title = entry.title.value;
        let entry_summary = entry.summary.map_or_else(String::new, |s| s.value);
        let entry_published = entry.published.unwrap_or_default();

        let article = Article {
            link: entry.id,
            channel: channel.clone(),
            title: entry_title,
            published: entry_published.to_string(),
            image: String::new(),
            summary: entry_summary,
            read_status: ReadStatus::Fresh,
        };
        articles.push(article);
    }

    Ok(articles)
}

/// Get articles from the database
#[allow(clippy::unused_async)]
pub async fn get_articles() -> Json<Value> {
    let db = sled::open("database").expect("Failed to open the database");
    let mut articles = ArticleServe {
        fresh: Vec::new(),
        saved: Vec::new(),
        archived: Vec::new(),
    };

    for item in db.iter() {
        let (_, v) = item.unwrap();
        let article: Article = serde_json::from_slice(&v).unwrap();
        match article.read_status {
            ReadStatus::Fresh => articles.fresh.push(article),
            ReadStatus::Saved => articles.saved.push(article),
            ReadStatus::Archived => articles.archived.push(article),
        }
    }

    Json(json!(articles))
}
