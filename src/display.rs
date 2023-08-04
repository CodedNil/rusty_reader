#![allow(non_snake_case)]
use crate::feed::{Article, ArticleServe};
use chrono::{Local, NaiveDateTime, TimeZone};
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;

pub fn App(cx: Scope) -> Element {
    let articles = use_future(cx, (), |_| get_server_data()).value();

    // Split out articles into their respective categories
    let mut articles_fresh = Vec::new();
    let mut articles_saved = Vec::new();
    let mut articles_archived = Vec::new();
    if let Some(Ok(list)) = &articles {
        articles_fresh = list.fresh.clone();
        articles_saved = list.saved.clone();
        articles_archived = list.archived.clone();
    }

    render! {
        style { include_str!("../src/style.css") }
        div { display: "flex", flex_direction: "row", width: "100%", height: "100vh",
            div { class: "article-list left saved",
                for article in &articles_saved {
                    ArticleListing { article: article.clone() }
                }
            }
            div { display: "flex", flex_direction: "column", width: "50%", height: "100vh",
                div { class: "article-preview" }
                div { class: "article-list center fresh",
                    for article in &articles_fresh {
                        ArticleListing { article: article.clone() }
                    }
                }
            }
            div { class: "article-list right archived",
                for article in &articles_archived {
                    ArticleListing { article: article.clone() }
                }
            }
        }
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

    let formatted_published = format_time_ago(published);

    // Render out the article listing
    cx.render(rsx! {
        div { padding: "0.5rem", position: "relative",
            div { display: "flex", flex_direction: "row",
                a { class: "feed-link", href: link.as_str(), "{title}" }
                div { class: "feed-date", "{formatted_published}" }
                img { class: "feed-icon", src: icon.as_str() }
            }
        }
    })
}

/// Format the published date to a human readable format, -30s, -2m 5s, -1h 30m, etc.
fn format_time_ago(published: &str) -> String {
    // Parse the published date
    let published_date = NaiveDateTime::parse_from_str(published, "%Y-%m-%d %H:%M:%S %z")
        .expect("Failed to parse published date");

    // Calculate the duration between the current time and the published date
    let duration =
        Local::now().signed_duration_since(Local.from_local_datetime(&published_date).unwrap());

    // Depending on the duration, format it in different ways
    match duration.num_seconds() {
        secs if secs < 60 => format!("-{secs}s"),
        secs => {
            let mins = secs / 60;
            match mins {
                mins if mins < 60 => format!("-{}m {}s", mins, secs % 60),
                mins => {
                    let hours = mins / 60;
                    match hours {
                        hours if hours < 24 => format!("-{}h {}m", hours, mins % 60),
                        hours => {
                            let days = hours / 24;
                            match days {
                                days if days < 7 => format!("-{}d {}h", days, hours % 24),
                                days => {
                                    let weeks = days / 7;
                                    match weeks {
                                        weeks if weeks < 4 => format!("-{}w {}d", weeks, days % 7),
                                        _ => format!("-{}m {}d", days / 30, days % 30),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn get_server_data() -> Result<ArticleServe, ServerFnError> {
    Ok(crate::feed::get_articles())
}
