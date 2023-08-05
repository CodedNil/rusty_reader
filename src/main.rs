use axum::{
    extract::Path,
    response::{IntoResponse, Json},
    routing::{get, put},
    Router,
};
use feed_rs::parser;
use readability::extractor;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sled::Db;
use tokio::time::{interval, Duration};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

// Standard library
use std::{io::Cursor, net::SocketAddr, str::FromStr, sync::Arc};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db: Arc<Db> = Arc::new(sled::open("database").expect("Failed to open database"));
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();

    let app = Router::new()
        .nest_service("/", ServeDir::new("assets"))
        .route("/articles", get(move || get_articles(db_clone1)))
        .route(
            "/articles/:link/:new_status",
            put(|path: Path<(String, String)>| update_article_status(path, db_clone2)),
        )
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {addr}");
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let article_puller = async {
        let mut interval = interval(Duration::from_secs(20 * 60));
        loop {
            interval.tick().await;
            println!("Pulling articles");
            pull_articles(db.clone()).await;
        }
    };

    tokio::select! {
        _ = server => {
            eprintln!("Server exited.");
        }
        _ = article_puller => {
            eprintln!("Article puller exited.");
        }
    }
}

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
struct Channel {
    link: String,
    title: String,
    icon: String,
    palette: Vec<String>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
struct Article {
    link: String,
    channel: Channel,
    title: String,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

/// Get articles and write them to the database
async fn pull_articles(db: Arc<Db>) {
    let rss_sources = vec![
        "https://www.theverge.com/rss/index.xml",
        // "https://www.tomshardware.com/rss.xml",
        "https://hnrss.org/frontpage",
    ];
    // http://feeds.bbci.co.uk/news/technology/rss.xml

    for source in rss_sources {
        match process_source(source, db.clone()).await {
            Ok(_) => {}
            Err(e) => eprintln!("Error processing source {source}: {e}"),
        }
    }
}

#[derive(Debug)]
struct WebpageData {
    favicon: Option<String>,
    image: Option<String>,
    main_content: Option<String>,
    palette: Option<Vec<String>>,
}

/// Website scraping for data
async fn scrape_website(url: &str) -> Result<WebpageData, Box<dyn std::error::Error>> {
    // Download the webpage and parse the html
    let resp = reqwest::get(url).await?;
    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // Get the favicon
    let favicon = if let Ok(favicon_selector) = Selector::parse("link[rel=\"shortcut icon\"]") {
        document
            .select(&favicon_selector)
            .next()
            .and_then(|element| element.value().attr("href"))
            .and_then(|relative_url| url::Url::parse(url).ok()?.join(relative_url).ok())
            .map(|url| url.to_string())
    } else {
        None
    };

    // Get the favicon image and extract the color palette
    let palette = if let Some(favicon_url) = &favicon {
        if let Ok(resp) = reqwest::get(favicon_url).await {
            if let Ok(bytes) = resp.bytes().await {
                if let Ok(colors) =
                    color_thief::get_palette(&bytes, color_thief::ColorFormat::Rgb, 10, 10)
                {
                    Some(
                        colors
                            .into_iter()
                            .map(|color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b))
                            .collect(),
                    )
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Get the first image
    let image = if let Ok(image_selector) = Selector::parse("img") {
        document
            .select(&image_selector)
            .next()
            .and_then(|element| element.value().attr("src"))
            .map(String::from)
    } else {
        None
    };

    // Get the main content using the readability crate
    let main_content = if let Ok(url_obj) = url::Url::parse(url) {
        let mut body_cursor = Cursor::new(body.clone());
        extractor::extract(&mut body_cursor, &url_obj)
            .ok()
            .map(|content| content.content)
    } else {
        None
    };

    Ok(WebpageData {
        favicon,
        image,
        main_content,
        palette,
    })
}

async fn process_source(source: &str, db: Arc<Db>) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(source).await?;
    let bytes = response.bytes().await?;
    let cursor = Cursor::new(bytes);
    let feed = parser::parse(cursor).unwrap();

    for entry in feed.entries {
        let entry_title = entry.title.unwrap().content;
        let entry_summary = entry.summary.map_or_else(String::new, |s| s.content);
        let entry_published = entry.published.unwrap_or_default();

        // Check if the article is already in the database
        // if db.contains_key(&entry.id)? {
        //     continue;
        // }

        // Download the webpage and extract the image
        if let Ok(data) = scrape_website(&entry.id.clone()).await {
            let channel = Channel {
                link: feed.id.clone(),
                title: feed.title.clone().unwrap().content.clone(),
                icon: data.favicon.unwrap_or_default(),
                palette: data.palette.unwrap_or_default(),
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

            let key = article.link.clone();
            let value = serde_json::to_vec(&article).unwrap();
            db.insert(key, value).unwrap();
        };
    }

    Ok(())
}

/// Get articles from the database
#[allow(clippy::unused_async)]
async fn get_articles(db: Arc<Db>) -> impl IntoResponse {
    let articles: Vec<Article> = db
        .iter()
        .map(|item| serde_json::from_slice(&item.unwrap().1).unwrap())
        .collect();

    Json(json!(articles))
}

/// Move an article to a different read status
#[allow(clippy::unused_async)]
async fn update_article_status(
    Path((link, new_status)): Path<(String, String)>,
    db: Arc<Db>,
) -> impl IntoResponse {
    // Try to get the key from the database
    let value = match db.get(&link) {
        Ok(Some(value)) => value,
        Ok(None) => return Json(json!({"status": "error", "message": "Article not found"})),
        Err(_) => return Json(json!({"status": "error", "message": "Database error"})),
    };

    // Try to deserialize the value into an Article
    let mut article: Article = match serde_json::from_slice(&value) {
        Ok(article) => article,
        Err(e) => return Json(json!({"status": "error", "message": e.to_string()})),
    };

    // Try to convert the new_status string to a ReadStatus
    let new_status_enum = match ReadStatus::from_str(&new_status) {
        Ok(status) => status,
        Err(e) => return Json(json!({"status": "error", "message": e})),
    };

    // Update the read status
    article.read_status = new_status_enum;

    // Try to serialize the updated article
    let updated_value = match serde_json::to_vec(&article) {
        Ok(value) => value,
        Err(e) => return Json(json!({"status": "error", "message": e.to_string()})),
    };

    // Try to update the value in the database
    if let Err(e) = db.insert(link, updated_value) {
        return Json(json!({"status": "error", "message": e.to_string()}));
    }

    // Try to flush the database
    if let Err(e) = db.flush() {
        return Json(json!({"status": "error", "message": e.to_string()}));
    }

    // Return a success response
    Json(json!({"status": "success", "message": "Article status updated successfully"}))
}
