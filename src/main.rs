mod articles;
mod channel;

use axum::{
    extract::Path,
    routing::{get, put},
    Router,
};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::{
    fs::{read_to_string, write},
    net::SocketAddr,
    sync::Arc,
};
use tokio::time::{interval, Duration};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db: Arc<Db> = Arc::new(sled::open("database").expect("Failed to open database"));
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();

    let app = Router::new()
        .nest_service("/", ServeDir::new("assets"))
        .route("/articles", get(move || articles::get_articles(db_clone1)))
        .route(
            "/articles/:link/:new_status",
            put(|path: Path<(String, String)>| articles::update_article_status(path, db_clone2)),
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
            println!("Done pulling articles");
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

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    rss: Vec<channel::ChannelOptional>,
}

/// Get articles and write them to the database
async fn pull_articles(db: Arc<Db>) {
    // Read and parse the TOML file, grab needed info
    let contents = read_to_string("feeds.toml").expect("Something went wrong reading the file");
    let mut config: Config = toml::from_str(&contents).expect("Error parsing the TOML");
    let mut config_changed = false;
    let mut new_rss = Vec::new();
    for feed in &config.rss {
        // If any of feeds variables are Option<None> then get new data
        let needs_fresh =
            feed.title.is_none() || feed.icon.is_none() || feed.dominant_color.is_none();
        let new_data = match channel::get_channel_data(&db.clone(), needs_fresh, feed).await {
            Ok(channel_data) => channel_data,
            Err(e) => {
                eprintln!("Error getting channel data for {}: {}", feed.rss_url, e);
                new_rss.push(feed.clone());
                continue;
            }
        };
        // Overwrite the feed in config with new_data
        new_rss.push(new_data.clone());
        config_changed = true;
    }
    // Write config if anything has changed
    if config_changed {
        config.rss = new_rss;
        let toml = toml::to_string(&config).expect("Error serializing the TOML");
        write("feeds.toml", toml).expect("Error writing to file");
    }

    // Pull the articles data
    for source in config.rss {
        match articles::process_source(&source.rss_url, db.clone()).await {
            Ok(_) => {}
            Err(e) => eprintln!("Error processing source {}: {}", source.rss_url, e),
        }
    }
}
