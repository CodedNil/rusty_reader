use atom_syndication::Feed;
use serde::{Deserialize, Serialize};
use sled;
use std::io::Cursor;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct Channel {
    link: String,
    title: String,
    icon: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub struct Article {
    link: String,
    channel: Channel,
    title: String,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

/// Get articles and write them to the database
async fn get_articles() {
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
