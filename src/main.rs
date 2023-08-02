use bincode::{config, Decode, Encode};
use feed_rs::parser;
use std::io::Cursor;

#[derive(Decode, Encode)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Decode, Encode)]
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

#[derive(Decode, Encode)]
pub enum ArticleType {
    Article,
    Video,
}

#[derive(Decode, Encode)]
struct Article {
    link: String,
    title: String,
    category: Category,
    article_type: ArticleType,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rss_sources = vec!["https://www.theverge.com/rss/index.xml"];
    let db = sled::open("database").unwrap();

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
            let entry_summary = entry.summary.map_or_else(String::new, |s| s.content);

            let entry_published = entry.published.unwrap_or_default();

            let key = entry.id.clone();
            let article = Article {
                link: entry.id,
                title: entry_title,
                category: Category::Tech,
                article_type: ArticleType::Article,
                published: entry_published.to_string(),
                image: String::new(),
                summary: entry_summary,
                read_status: ReadStatus::Fresh,
            };
            let value = bincode::encode_to_vec(article, config::standard()).unwrap();
            db.insert(key, value)?;
        }
    }

    Ok(())
}
