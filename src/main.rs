mod feed;

use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::time::{interval, Duration};
use tower_http::services::ServeDir;

use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new()
        .nest_service("/", ServeDir::new("assets"))
        .route("/articles", get(feed::get_articles))
        .layer(ServiceBuilder::new().layer(CompressionLayer::new()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {addr}");
    let server = axum::Server::bind(&addr).serve(app.into_make_service());

    let article_puller = async {
        let mut interval = interval(Duration::from_secs(20 * 60));
        loop {
            interval.tick().await;
            println!("Pulling articles");
            feed::pull_articles().await;
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
