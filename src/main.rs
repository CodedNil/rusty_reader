mod articles;
mod channel;
mod gpt;
mod wallpaper;

use axum::{
    extract::Path,
    routing::{get, put},
    Router,
};
use futures::{stream, StreamExt};
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
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Database setup
    let db = Arc::new(sled::open("database").expect("Failed to open database"));

    // Create clones for the router
    let db_for_get = db.clone();
    let db_for_put = db.clone();

    // Router setup
    let app = Router::new()
        .nest_service("/", ServeDir::new("assets"))
        .route("/articles", get(move || articles::get_articles(db_for_get)))
        .route(
            "/articles/:link/:new_status",
            put(move |path: Path<(String, String)>| {
                articles::update_article_status(path, db_for_put)
            }),
        )
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()));

    // Server setup
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {addr}");
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    // Article puller setup
    let article_puller = async {
        let mut interval = interval(Duration::from_secs(20 * 60));
        loop {
            interval.tick().await;
            println!("Pulling articles");
            pull_articles(db.clone()).await;
            println!("Done pulling articles");
        }
    };

    // Wallpaper generator setup
    let wallpaper_generator = async {
        let mut interval = interval(Duration::from_secs(360 * 60));
        loop {
            interval.tick().await;
            println!("Generating wallpaper");
            // Attempt to generate a prompt for the wallpaper
            match wallpaper::generate_prompt().await {
                Ok(prompt) => {
                    // Log the generated prompt
                    println!("Prompt result: {prompt:?}");

                    // Attempt to generate the wallpaper image using the prompt
                    match wallpaper::generate_image(
                        &prompt,
                        1344,
                        768,
                        wallpaper::WriteOption::Desktop,
                    )
                    .await
                    {
                        Ok(wallpaper_result) => {
                            // Log the successful generation of the wallpaper
                            println!("Wallpaper result: {wallpaper_result:?}");
                        }
                        Err(e) => {
                            // Log any errors encountered during wallpaper generation
                            println!("Error generating wallpaper {e}");
                        }
                    }
                }
                Err(prompt_result) => {
                    // Log any errors encountered during prompt generation
                    println!("Error generating prompt {prompt_result:?}");
                }
            }
        }
    };

    // Run server and article puller concurrently
    tokio::select! {
        _ = server => {
            eprintln!("Server exited.");
        }
        _ = article_puller => {
            eprintln!("Article puller exited.");
        }
        _ = wallpaper_generator => {
            eprintln!("Wallpaper generator exited.");
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    rss: Vec<channel::ChannelOptional>,
}

/// Get articles and write them to the database
async fn pull_articles(db: Arc<Db>) {
    let contents = read_to_string("feeds.toml").expect("Failed to read the file");
    let mut config: Config = toml::from_str(&contents).expect("Failed to parse the TOML");
    let mut config_changed = false;
    let mut new_rss = Vec::new();

    for feed in &config.rss {
        let needs_fresh =
            feed.title.is_none() || feed.icon.is_none() || feed.dominant_color.is_none();

        match channel::get_channel_data(&db, needs_fresh, feed).await {
            Ok(channel_data) => {
                new_rss.push(channel_data);
                config_changed = true;
            }
            Err(e) => {
                eprintln!("Error getting channel data for {}: {}", feed.rss_url, e);
                new_rss.push(feed.clone());
            }
        }
    }

    if config_changed {
        config.rss = new_rss;
        let toml = toml::to_string(&config).expect("Failed to serialize the TOML");
        write("feeds.toml", toml).expect("Failed to write to file");
    }

    stream::iter(config.rss.iter())
        .for_each_concurrent(2, |source| {
            let db = db.clone();
            async move {
                if let Err(e) = articles::process_source(&source.rss_url, db).await {
                    eprintln!("Error processing source {}: {}", source.rss_url, e);
                }
            }
        })
        .await;
}
