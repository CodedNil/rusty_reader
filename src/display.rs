#![allow(non_snake_case)]
use crate::feed::{Article, ArticleServe};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;

pub fn App(cx: Scope) -> Element {
    render! {
        div { display: "flex", flex_direction: "column", width: "100%", height: "calc(100vh - 16px)",
            div { width: "100%", height: "40%", background_color: "blue" }
            div { width: "100%", height: "60%", overflow_y: "scroll", ArticlesLists {} }
        }
    }
}

fn ArticlesLists(cx: Scope) -> Element {
    let articles = use_future(cx, (), |_| get_server_data()).value();

    match articles {
        Some(Ok(list)) => render! {
            div { display: "flex", flex_direction: "row", width: "100%",
                div { display: "flex", flex_direction: "column", width: "100%", background_color: "gold",
                    for article in &list.saved {
                        ArticleListing { article: article.clone() }
                    }
                }
                div { display: "flex", flex_direction: "column", width: "100%", background_color: "green",
                    for article in &list.fresh {
                        ArticleListing { article: article.clone() }
                    }
                }
                div { display: "flex", flex_direction: "column", width: "100%", background_color: "red",
                    for article in &list.archived {
                        ArticleListing { article: article.clone() }
                    }
                }
            }
        },
        Some(Err(e)) => render! {format!("Error: {}", e)},
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

    // Format the published data to local time in a compact display
    let naive = NaiveDateTime::parse_from_str(published, "%Y-%m-%d %H:%M:%S %z").unwrap();
    let datetime: DateTime<Utc> = Utc.from_local_datetime(&naive).single().unwrap();
    let formatted_published = datetime.format("%b %d, %H:%M").to_string();

    // Render out the article listing
    cx.render(rsx! {
        div { padding: "0.5rem", position: "relative",
            div { display: "flex", flex_direction: "row", color: "gray",
                a { href: link.as_str(), "{title}" }
                div { padding_left: "0.5rem", "{formatted_published}" }
                img { padding_left: "0.5rem", width: "auto", height: "100%", object_fit: "cover", src: icon.as_str() }
            }
        }
    })
}

#[server]
async fn get_server_data() -> Result<ArticleServe, ServerFnError> {
    Ok(crate::feed::get_articles())
}
