#![allow(non_snake_case)]
use atom_syndication::Feed;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Deserialize, Serialize, PartialEq, Clone)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
struct Channel {
    link: String,
    title: String,
    icon: String,
}

#[derive(Deserialize, Serialize, PartialEq, Clone)]
struct Article {
    link: String,
    channel: Channel,
    title: String,
    published: String,
    image: String,
    summary: String,
    read_status: ReadStatus,
}

#[must_use]
pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, Vec::<Article>::new);

    cx.render(rsx! {
        div { display: "flex", flex_direction: "row", width: "100%",
            div { width: "100%", Articles {} }
        }
    })
}

fn Articles(cx: Scope) -> Element {
    let articles = use_future(cx, (), |_| get_articles());

    match articles.value() {
        Some(Ok(list)) => render! {
            div {
                for article in list {
                    ArticleListing { article: article.clone() }
                }
            }
        },
        Some(Err(err)) => render! {"An error occurred while fetching articles {err}"},
        None => render! {"Loading items"},
    }
}

#[inline_props]
fn ArticleListing(cx: Scope, article: Article) -> Element {
    let Article {
        title,
        link,
        published,
        ..
    } = article;
    let icon = &article.channel.icon;

    cx.render(rsx! {
        div { padding: "0.5rem", position: "relative",
            div { display: "flex", flex_direction: "row", color: "gray",
                div { font_size: "1.5rem", a { href: link.as_str(), "{title}" } }
                div { padding_left: "0.5rem", "{published}" }
                img { width: "1.5rem", height: "1.5rem", src: icon.as_str() }
            }
        }
    })
}

async fn get_articles() -> Result<Vec<Article>, reqwest::Error> {
    let rss_sources = vec!["https://www.theverge.com/rss/index.xml"];
    // http://feeds.bbci.co.uk/news/technology/rss.xml
    // https://hnrss.org/frontpage
    // let db = sled::open("database").expect("Failed to open the database");
    let mut articles = Vec::<Article>::new();

    for source in rss_sources {
        if let Err(e) = process_source(source).await {
            eprintln!("Failed to process source {source}: {e}");
        }

        match process_source(source).await {
            Ok(new_articles) => {
                articles.extend(new_articles);
            }
            Err(e) => {
                eprintln!("Failed to process source {source}: {e}");
            }
        }

        // let key = entry.id.clone();
        // let value = bincode::encode_to_vec(article, config::standard()).unwrap();
        // db.insert(key, value)?;
    }

    // if let Err(e) = db.flush() {
    //     eprintln!("Failed to flush the database: {e}");
    // }

    Ok(articles)
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

fn main() {
    dioxus_web::launch(App);
}
