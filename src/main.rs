#![allow(non_snake_case)]
use chrono::{DateTime, Utc};
use dioxus::prelude::*;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use std::result::Result;

const BASE_API_URL: &str = "https://hacker-news.firebaseio.com/v0/";
const ITEM_API: &str = "item/";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoryPageData {
    #[serde(flatten)]
    pub item: StoryItem,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoryItem {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub descendants: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    pub r#type: String,
}

pub async fn get_story(id: i64) -> Result<StoryPageData, reqwest::Error> {
    let url = format!("{BASE_API_URL}{ITEM_API}{id}.json");
    let story = reqwest::get(&url).await?.json::<StoryPageData>().await?;
    Ok(story)
}

pub async fn get_story_preview(id: i64) -> Result<StoryItem, reqwest::Error> {
    let url = format!("{BASE_API_URL}item/{id}.json");
    reqwest::get(&url).await?.json().await
}

pub async fn get_stories(count: usize) -> Result<Vec<StoryItem>, reqwest::Error> {
    let url = format!("{BASE_API_URL}topstories.json");
    let stories_ids = &reqwest::get(&url).await?.json::<Vec<i64>>().await?[..count];

    let story_futures = stories_ids[..usize::min(stories_ids.len(), count)]
        .iter()
        .map(|&story_id| get_story_preview(story_id));
    Ok(join_all(story_futures)
        .await
        .into_iter()
        .filter_map(Result::ok)
        .collect())
}

#[must_use]
pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, || PreviewState::Unset);

    cx.render(rsx! {
        div {
            display: "flex",
            flex_direction: "row",
            width: "100%",
            div {
                width: "50%",
                Stories {}
            }
            div {
                width: "50%",
                Preview {}
            }
        }
    })
}

fn Stories(cx: Scope) -> Element {
    let story = use_future(cx, (), |_| get_stories(10));

    match story.value() {
        Some(Ok(list)) => render! {
            div {
                for story in list {
                    StoryListing { story: story.clone() }
                }
            }
        },
        Some(Err(err)) => render! {"An error occurred while fetching stories {err}"},
        None => render! {"Loading items"},
    }
}

async fn resolve_story(
    full_story: UseRef<Option<StoryPageData>>,
    preview_state: UseSharedState<PreviewState>,
    story_id: i64,
) {
    if let Some(cached) = &*full_story.read() {
        *preview_state.write() = PreviewState::Loaded(cached.clone());
        return;
    }

    *preview_state.write() = PreviewState::Loading;
    if let Ok(story) = get_story(story_id).await {
        *preview_state.write() = PreviewState::Loaded(story.clone());
        *full_story.write() = Some(story);
    }
}

#[inline_props]
fn StoryListing(cx: Scope, story: StoryItem) -> Element {
    let preview_state = use_shared_state::<PreviewState>(cx).unwrap();
    let StoryItem {
        title,
        url,
        by,
        score,
        time,
        id,
        ..
    } = story;
    let full_story = use_ref(cx, || None);

    let url = url.as_deref().unwrap_or_default();
    let hostname = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let score = format!("{score} {}", if *score == 1 { " point" } else { " points" });
    let time = time.format("%D %l:%M %p");

    cx.render(rsx! {
        div {
            padding: "0.5rem",
            position: "relative",
            onmouseenter: move |_event| {
                resolve_story(full_story.clone(), preview_state.clone(), *id)
            },
            div {
                font_size: "1.5rem",
                a {
                    href: url,
                    onfocus: move |_event| {
                        resolve_story(full_story.clone(), preview_state.clone(), *id)
                    },
                    "{title}"
                }
                a {
                    color: "gray",
                    href: "https://news.ycombinator.com/from?site={hostname}",
                    text_decoration: "none",
                    " ({hostname})"
                }
            }
            div {
                display: "flex",
                flex_direction: "row",
                color: "gray",
                div {
                    "{score}"
                }
                div {
                    padding_left: "0.5rem",
                    "by {by}"
                }
                div {
                    padding_left: "0.5rem",
                    "{time}"
                }
            }
        }
    })
}

#[derive(Clone, Debug)]
enum PreviewState {
    Unset,
    Loading,
    Loaded(StoryPageData),
}

fn Preview(cx: Scope) -> Element {
    let preview_state = use_shared_state::<PreviewState>(cx)?;

    match &*preview_state.read() {
        PreviewState::Unset => render! {
            "Hover over a story to preview it here"
        },
        PreviewState::Loading => render! {
            "Loading..."
        },
        PreviewState::Loaded(story) => {
            let title = &story.item.title;
            let url = story.item.url.as_deref().unwrap_or_default();
            let text = story.item.text.as_deref().unwrap_or_default();
            render! {
                div {
                    padding: "0.5rem",
                    div {
                        font_size: "1.5rem",
                        a {
                            href: "{url}",
                            "{title}"
                        }
                    }
                    div {
                        dangerous_inner_html: "{text}",
                    }
                }
            }
        }
    }
}

fn main() {
    // launch the web app
    dioxus_web::launch(App);
}

// use bincode::{config, Decode, Encode};
// use feed_rs::parser;
// use std::io::Cursor;

// #[derive(Decode, Encode)]
// pub enum ReadStatus {
//     Fresh,
//     Saved,
//     Archived,
// }

// #[derive(Decode, Encode)]
// pub enum Category {
//     Entertainment,
//     Tech,
//     CodeUpdates,
//     GameDev,
//     Politics,
//     Business,
//     Science,
//     Health,
//     Sports,
//     Other,
// }

// impl Category {
//     #[must_use]
//     pub fn description(&self) -> &str {
//         match *self {
//             Category::Entertainment => "Movies, music, and entertainment",
//             Category::Tech => "Tech articles, gadget reviews, IT news",
//             Category::CodeUpdates => "Updates from coding, software projects",
//             Category::GameDev => "Game development insights, updates",
//             Category::Politics => "Political analysis, government, elections",
//             Category::Business => "Business insights, market trends, economic analyses",
//             Category::Science => "Discoveries, research, science breakthroughs",
//             Category::Health => "Health advice, medical research, nutrition",
//             Category::Sports => "Sports updates, scores, news",
//             Category::Other => "Content that doesn't fit other categories",
//         }
//     }
// }

// #[derive(Decode, Encode)]
// pub enum ArticleType {
//     Article,
//     Video,
// }

// #[derive(Decode, Encode)]
// struct Article {
//     link: String,
//     title: String,
//     category: Category,
//     article_type: ArticleType,
//     published: String,
//     image: String,
//     summary: String,
//     read_status: ReadStatus,
// }

// async fn process_source(source: &str, db: &sled::Db) -> Result<(), Box<dyn std::error::Error>> {
//     let response = reqwest::get(source).await?;
//     let bytes = response.bytes().await?;
//     let cursor = Cursor::new(bytes);
//     let feed = parser::parse(cursor)?;

//     for entry in feed.entries {
//         // If entry doesnt have title skip it
//         if entry.title.is_none() {
//             continue;
//         }
//         let entry_title = entry.title.unwrap().content;
//         let entry_summary = entry.summary.map_or_else(String::new, |s| s.content);

//         let entry_published = entry.published.unwrap_or_default();

//         let key = entry.id.clone();
//         let article = Article {
//             link: entry.id,
//             title: entry_title,
//             category: Category::Tech,
//             article_type: ArticleType::Article,
//             published: entry_published.to_string(),
//             image: String::new(),
//             summary: entry_summary,
//             read_status: ReadStatus::Fresh,
//         };
//         let value = bincode::encode_to_vec(article, config::standard()).unwrap();
//         db.insert(key, value)?;
//     }

//     Ok(())
// }

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn std::error::Error>> {
//     let rss_sources = vec!["https://www.theverge.com/rss/index.xml", "http://feeds.bbci.co.uk/news/technology/rss.xml"];
//     let db = sled::open("database").expect("Failed to open the database");

//     for source in rss_sources {
//         if let Err(e) = process_source(source, &db).await {
//             eprintln!("Failed to process source {source}: {e}");
//         }
//     }

//     if let Err(e) = db.flush() {
//         eprintln!("Failed to flush the database: {e}");
//     }

//     Ok(())
// }
