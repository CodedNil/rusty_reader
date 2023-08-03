#![allow(non_snake_case)]
use crate::feed::Article;
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;

pub fn App(cx: Scope) -> Element {
    use_shared_state_provider(cx, Vec::<Article>::new);

    cx.render(rsx! {
        div { display: "flex", flex_direction: "row", width: "100%",
            div { width: "100%", Articles {} }
        }
    })
}

fn Articles(cx: Scope) -> Element {
    let articles = use_future(cx, (), |_| grab_articles());

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

#[server]
async fn grab_articles() -> Result<Vec<Article>, ServerFnError> {
    // Perform some expensive computation or access a database on the server
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let result = number * 2;
    println!("server calculated {result}");
    Ok(Vec::new())
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
