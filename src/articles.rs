use axum::{
    extract::Path,
    response::{IntoResponse, Json},
};
use bincode::{deserialize, serialize};
use feed_rs::parser;
use readability::extractor;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::Db;
use std::{io::Cursor, str::FromStr, sync::Arc};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}
impl FromStr for ReadStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fresh" => Ok(ReadStatus::Fresh),
            "Saved" => Ok(ReadStatus::Saved),
            "Archived" => Ok(ReadStatus::Archived),
            _ => Err(format!("'{s}' is not a valid read status")),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
struct Article {
    link: String,
    channel: String,
    title: String,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

#[derive(Debug)]
struct WebpageData {
    image: Option<String>,
    summary: Option<String>,
    new_title: Option<String>,
}

/// Website scraping for data
async fn scrape_website(
    db: Arc<Db>,
    title: String,
    url: &str,
) -> Result<WebpageData, Box<dyn std::error::Error>> {
    // Download the webpage and parse the html
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // Get the first image
    let image = Selector::parse("img").ok().and_then(|selector| {
        document
            .select(&selector)
            .next()
            .and_then(|element| element.value().attr("src"))
            .map(String::from)
    });

    // Get the main content using the readability crate
    let summary = match url::Url::parse(url) {
        Ok(url_obj) => {
            let mut body_cursor = Cursor::new(body);
            let main_content = extractor::extract(&mut body_cursor, &url_obj)
                .ok()
                .map(|content| content.content);

            if main_content.is_some() {
                // Use GPT3.5 to summarize the article
                let summary =
                    crate::gpt::summarise_article(db, title, main_content.unwrap()).await?;
                Some(summary)
            } else {
                None
            }
        }
        Err(_) => None,
    };

    if let Some(summary) = summary {
        return Ok(WebpageData {
            image,
            summary: Some(summary.summary),
            new_title: Some(summary.title),
        });
    }
    Ok(WebpageData {
        image,
        summary: None,
        new_title: None,
    })
}

pub async fn process_source(
    source: &String,
    db: Arc<Db>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Processing source {source}");
    let response = reqwest::get(source).await?;
    let bytes = response.bytes().await?;
    let cursor = Cursor::new(bytes);
    let feed = parser::parse(cursor)?;

    for entry in feed.entries {
        let entry_title = entry.title.map_or(String::new(), |t| t.content);
        let entry_summary = entry.summary.map_or(String::new(), |s| s.content);
        let entry_published = entry.published.unwrap_or_default();
        let entry_link = entry
            .links
            .first()
            .map_or(entry.id.clone(), |link| link.href.clone());

        // Check if the article is already in the database
        if !db.contains_key(format!("article:{}", &entry_link))? {
            // Download the webpage and extract the image
            if let Ok(data) = scrape_website(db.clone(), entry_title.clone(), &entry_link).await {
                let article = Article {
                    link: entry_link,
                    channel: source.clone(),
                    title: data.new_title.unwrap_or(entry_title),
                    published: entry_published.to_rfc3339(),
                    image: data.image.unwrap_or_default(),
                    summary: data.summary.unwrap_or(entry_summary),
                    read_status: ReadStatus::Fresh,
                };
                store_article_to_db(&db, &article)?;
            };
        }
    }

    Ok(())
}

// Function to retrieve a article from the database based on its link.
fn get_article_from_db(db: &Db, link: &str) -> Result<Article, Box<dyn std::error::Error>> {
    // Construct the key for the database lookup using the provided link.
    let key = format!("article:{link}");

    // Attempt to retrieve the data associated with the key.
    match db.get(key)? {
        // If data is found, deserialize it from binary format to a Article struct.
        Some(ivec) => {
            let article: Article = deserialize(&ivec)?;
            Ok(article)
        }
        // If no data is found, return an error.
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Article not found",
        ))),
    }
}

/// Function to store a article into the database.
fn store_article_to_db(db: &Db, article: &Article) -> Result<(), Box<dyn std::error::Error>> {
    println!("Storing article {}", article.link);
    let key = format!("article:{}", &article.link);
    db.insert(key, sled::IVec::from(serialize(article)?))?;
    db.flush()?;
    Ok(())
}

/// Struct to represent the full article with its associated channel.
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
struct FullArticle {
    link: String,
    channel: crate::channel::Channel,
    title: String,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

/// Get articles from the database
#[allow(clippy::unused_async, clippy::module_name_repetitions)]
pub async fn get_articles(db: Arc<Db>) -> impl IntoResponse {
    let articles: Vec<FullArticle> = db
        .scan_prefix("article:")
        .filter_map(Result::ok)
        .filter_map(|(_, value)| {
            let article: Article = deserialize(&value).ok()?;
            let channel = crate::channel::get_channel_from_db(&db, &article.channel).ok()?;
            Some(FullArticle {
                link: article.link,
                channel,
                title: article.title,
                published: article.published,
                image: article.image,
                summary: article.summary,
                read_status: article.read_status,
            })
        })
        .collect();

    Json(json!(articles))
}

/// Move an article to a different read status
#[allow(clippy::unused_async)]
pub async fn update_article_status(
    Path((link, new_status)): Path<(String, String)>,
    db: Arc<Db>,
) -> impl IntoResponse {
    // Decode link URI
    let link: String = match urlencoding::decode(&link) {
        Ok(link) => link.to_string(),
        Err(e) => return Json(json!({"status": "error", "message": e.to_string()})),
    };

    // Try to get the article from the database
    let mut article: Article = match get_article_from_db(&db, &link) {
        Ok(article) => article,
        Err(e) => {
            return Json(
                json!({"status": "error", "message": format!("Failed to get article from database: {}", e)}),
            )
        }
    };

    // Try to convert the new_status string to a ReadStatus
    let new_status_enum = match ReadStatus::from_str(&new_status) {
        Ok(status) => status,
        Err(e) => {
            return Json(
                json!({"status": "error", "message": format!("Failed to convert new status to ReadStatus: {}", e)}),
            )
        }
    };

    // Update the read status and store the article in the database
    article.read_status = new_status_enum;
    if let Err(e) = store_article_to_db(&db, &article) {
        return Json(
            json!({"status": "error", "message": format!("Failed to store updated article in database: {}", e)}),
        );
    }

    Json(json!({"status": "success", "message": "Article status updated successfully"}))
}
