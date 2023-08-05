use atom_syndication::Feed;
use axum::{
    extract::Path,
    response::{IntoResponse, Json},
};
use readability::extractor;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{io::Cursor, str::FromStr};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub enum ReadStatus {
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

/// Get articles and write them to the database
pub async fn pull_articles() {
    let rss_sources = vec!["https://www.theverge.com/rss/index.xml"];
    // http://feeds.bbci.co.uk/news/technology/rss.xml  https://hnrss.org/frontpage
    let mut new_articles = Vec::new();

    for source in rss_sources {
        match process_source(source).await {
            Ok(articles) => {
                new_articles.extend(articles);
            }
            Err(e) => {
                eprintln!("Failed to process source {source}: {e}");
            }
        }
    }

    // Write to database
    let db = sled::open("database").expect("Failed to open the database");
    for article in new_articles {
        let key = article.link.clone();
        let value = serde_json::to_vec(&article).unwrap();
        db.insert(key, value).unwrap();
    }
    if let Err(e) = db.flush() {
        eprintln!("Failed to flush the database: {e}");
    }
}

#[derive(Debug)]
pub struct WebpageData {
    favicon: Option<String>,
    image: Option<String>,
    main_content: Option<String>,
}

/// Website scraping for data
pub async fn scrape_website(url: &str) -> Result<WebpageData, Box<dyn std::error::Error>> {
    // Download the webpage
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;

    // Parse the HTML
    let document = Html::parse_document(&body);

    // Get the favicon
    let favicon_selector = Selector::parse("link[rel=\"shortcut icon\"]").unwrap();
    let favicon = document
        .select(&favicon_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .and_then(|relative_url| url::Url::parse(url).ok()?.join(relative_url).ok())
        .map(|url| url.to_string());

    // Get the first image
    let image_selector = Selector::parse("img").unwrap();
    let image = document
        .select(&image_selector)
        .next()
        .and_then(|element| element.value().attr("src"))
        .map(String::from);

    // Get the main content using the readability crate
    let url = url::Url::parse(url)?;
    let mut body_cursor = Cursor::new(body);
    let main_content = extractor::extract(&mut body_cursor, &url)?;

    Ok(WebpageData {
        favicon,
        image,
        main_content: Some(main_content.content),
    })
}

async fn process_source(source: &str) -> Result<Vec<Article>, Box<dyn std::error::Error>> {
    let response = reqwest::get(source).await?;
    let bytes = response.bytes().await?;
    let cursor = Cursor::new(bytes);
    let feed = Feed::read_from(cursor).unwrap();

    let mut articles = Vec::new();

    for entry in feed.entries {
        let entry_title = entry.title.value;
        let entry_summary = entry.summary.map_or_else(String::new, |s| s.value);
        let entry_published = entry.published.unwrap_or_default();

        // Download the webpage and extract the image
        let data = scrape_website(&entry.id.clone()).await?;

        let channel = Channel {
            link: feed.id.clone(),
            title: feed.title.value.clone(),
            icon: data
                .favicon
                .unwrap_or(feed.icon.clone().unwrap_or_default()),
        };

        let article = Article {
            link: entry.id,
            channel,
            title: entry_title,
            published: entry_published.to_string(),
            image: data.image.unwrap_or_default(),
            summary: data.main_content.unwrap_or(entry_summary),
            read_status: ReadStatus::Fresh,
        };
        articles.push(article);
    }

    Ok(articles)
}

/// Get articles from the database
#[allow(clippy::unused_async)]
pub async fn get_articles() -> impl IntoResponse {
    let db = sled::open("database").expect("Failed to open the database");
    let articles: Vec<Article> = db
        .iter()
        .map(|item| serde_json::from_slice(&item.unwrap().1).unwrap())
        .collect();

    Json(json!(articles))
}

/// Move an article to a different read status
#[allow(clippy::unused_async)]
pub async fn update_article_status(
    Path((article_link, new_status)): Path<(String, String)>,
) -> impl IntoResponse {
    let db = sled::open("database").expect("Failed to open the database");

    // Look for the key in the database
    match db.get(&article_link) {
        Ok(Some(value)) => {
            // Deserialize the value into an Article
            let mut article: Article = serde_json::from_slice(&value).unwrap();

            // Convert the new_status string to a ReadStatus
            match ReadStatus::from_str(&new_status) {
                Ok(new_status) => {
                    // Update the read status
                    article.read_status = new_status;

                    // Serialize the updated article and update the value in the database
                    let updated_value = serde_json::to_vec(&article).unwrap();
                    db.insert(article_link, updated_value).unwrap();

                    // Flush the database
                    if let Err(e) = db.flush() {
                        eprintln!("Failed to flush the database: {e}");
                    }

                    // Return a success response
                    Json(
                        json!({"status": "success", "message": "Article status updated successfully"}),
                    )
                }
                Err(e) => {
                    // Return an error response if the new_status string is not a valid ReadStatus
                    Json(json!({"status": "error", "message": e}))
                }
            }
        }
        Ok(None) => {
            // The article link does not exist in the database
            Json(json!({"status": "error", "message": "Article not found"}))
        }
        Err(_) => {
            // An error occurred while accessing the database
            Json(json!({"status": "error", "message": "Database error"}))
        }
    }
}
