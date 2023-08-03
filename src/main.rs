#![allow(non_snake_case)]
use bincode::{config, Decode, Encode};
use dioxus::prelude::*;
use feed_rs::parser;
use std::io::Cursor;

#[derive(Decode, Encode, PartialEq, Clone)]
pub enum ReadStatus {
    Fresh,
    Saved,
    Archived,
}

#[derive(Decode, Encode, PartialEq, Clone)]
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

#[derive(Decode, Encode, PartialEq, Clone)]
pub enum ArticleType {
    Article,
    Video,
}

#[derive(Decode, Encode, PartialEq, Clone)]
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

#[must_use]
pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, Vec::<Article>::new);

    cx.render(rsx! {
        div {
            display: "flex",
            flex_direction: "row",
            width: "100%",
            div {
                width: "100%",
                Articles {}
            }
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
        category,
        published,
        summary,
        ..
    } = article;

    cx.render(rsx! {
        div {
            padding: "0.5rem",
            position: "relative",
            div {
                font_size: "1.5rem",
                a {
                    href: link.as_str(),
                    "{title}"
                }
            }
            div {
                display: "flex",
                flex_direction: "row",
                color: "gray",
                div {
                    "{category.description()}"
                }
                div {
                    padding_left: "0.5rem",
                    "{published}"
                }
            }
            div {
                "{summary}"
            }
        }
    })
}

async fn get_articles() -> Result<Vec<Article>, reqwest::Error> {
    let rss_sources = vec![
        "https://www.theverge.com/rss/index.xml",
        "http://feeds.bbci.co.uk/news/technology/rss.xml",
    ];
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
    let feed = parser::parse(cursor)?;

    let mut articles = Vec::new();

    for entry in feed.entries {
        // If entry doesnt have title skip it
        if entry.title.is_none() {
            continue;
        }
        let entry_title = entry.title.unwrap().content;
        let entry_summary = entry.summary.map_or_else(String::new, |s| s.content);

        let entry_published = entry.published.unwrap_or_default();

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
        articles.push(article);
    }

    Ok(articles)
}

fn main() {
    dioxus_web::launch(App);
}
