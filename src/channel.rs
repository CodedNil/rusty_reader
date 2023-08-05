use bincode::{deserialize, serialize};
use feed_rs::parser;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::{io::Cursor, sync::Arc};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct Channel {
    pub link: String,
    pub title: String,
    pub icon: String,
    pub palette: Vec<String>,
}

// Function to retrieve a channel from the database based on its link.
fn get_channel_from_db(db: &Db, link: &str) -> Result<Channel, Box<dyn std::error::Error>> {
    // Construct the key for the database lookup using the provided link.
    let key = format!("channel:{link}");

    // Attempt to retrieve the data associated with the key.
    match db.get(key)? {
        // If data is found, deserialize it from binary format to a Channel struct.
        Some(ivec) => {
            let channel: Channel = deserialize(&ivec)?;
            Ok(channel)
        }
        // If no data is found, return an error.
        None => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Channel not found",
        ))),
    }
}

// Function to store a channel into the database.
fn store_channel_to_db(db: &Db, channel: &Channel) -> Result<(), Box<dyn std::error::Error>> {
    // Construct the key for the database insertion using the link from the Channel struct.
    let key = format!("channel:{}", &channel.link);

    // Serialize the Channel struct into a binary format.
    let value = serialize(channel)?;

    // Insert the serialized data into the database with the constructed key.
    db.insert(key, sled::IVec::from(value))?;

    // If all operations are successful, return Ok.
    Ok(())
}

pub async fn get_channel_data(
    db: Arc<Db>,
    source: &str,
) -> Result<Channel, Box<dyn std::error::Error>> {
    // Attempt to retrieve the channel from the database.
    if let Ok(channel) = get_channel_from_db(&db, source) {
        return Ok(channel);
    }

    // If the channel is not in the database, fetch the page feed.
    let response = reqwest::get(source).await?;
    let bytes = response.bytes().await?;
    let cursor = Cursor::new(bytes);
    let feed = parser::parse(cursor)?;

    // Download the webpage to parse the HTML content.
    let resp = reqwest::get(source).await?;
    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // Attempt to extract the favicon URL from the HTML.
    let favicon_selector = Selector::parse("link[rel=\"shortcut icon\"]")?;
    let favicon = document
        .select(&favicon_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .and_then(|relative_url| url::Url::parse(source).ok()?.join(relative_url).ok())
        .map(|url| url.to_string());

    // If a favicon URL is found, fetch the image and extract its color palette.
    let palette = if let Some(favicon_url) = &favicon {
        let resp = reqwest::get(favicon_url).await?;
        let bytes = resp.bytes().await?;
        color_thief::get_palette(&bytes, color_thief::ColorFormat::Rgb, 10, 10)
            .map(|colors| {
                colors
                    .into_iter()
                    .map(|color| format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b))
                    .collect()
            })
            .ok()
    } else {
        None
    };

    // Construct the Channel object.
    let channel = Channel {
        link: feed.id.clone(),
        title: feed.title.clone().unwrap().content.clone(),
        icon: favicon.unwrap_or_default(),
        palette: palette.unwrap_or_default(),
    };

    // Store the newly constructed channel in the database.
    store_channel_to_db(&db, &channel)?;

    Ok(channel)
}
