#![allow(non_snake_case)]
mod display;
mod feed;

#[cfg(not(feature = "backend"))]
use dioxus_fullstack::launch::LaunchBuilder;

#[cfg(not(feature = "backend"))]
fn main() {
    LaunchBuilder::new(display::App).launch();
}

#[cfg(feature = "backend")]
#[tokio::main]
async fn main() {
    println!("Pulling articles");
    feed::pull_articles().await;
}
