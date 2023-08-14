use bincode::{deserialize, serialize};
use feed_rs::parser;
use image::{DynamicImage, GenericImageView};
use piped::PipedClient;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::{collections::HashMap, io::Cursor};

const PIPED_INSTANCE: &'static str = "https://pipedapi.kavin.rocks";

/// Struct to represent a channel.
#[allow(clippy::module_name_repetitions)]
#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct ChannelOptional {
    pub category: Option<String>,
    pub rss_url: String,
    pub title: Option<String>,
    pub icon: Option<String>,
    pub dominant_color: Option<String>,
}

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct Channel {
    pub category: String,
    pub rss_url: String,
    pub title: String,
    pub icon: String,
    pub dominant_color: String,
}

// Function to retrieve a channel from the database based on its link.
pub fn get_channel_from_db(db: &Db, link: &str) -> Result<Channel, Box<dyn std::error::Error>> {
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
fn store_channel_to_db(
    db: &Db,
    channel: &Channel,
    link: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    db.insert(
        format!("channel:{link}"),
        sled::IVec::from(serialize(channel)?),
    )?;
    db.flush()?;
    Ok(())
}

/// Function to retrieve a channel from the database or create it if it does not exist.
pub async fn get_channel_data(
    db: &Db,
    needs_fresh: bool,
    source: &ChannelOptional,
) -> Result<ChannelOptional, Box<dyn std::error::Error>> {
    if needs_fresh {
        // Fetch the page feed.
        let response = reqwest::get(source.rss_url.clone()).await?;
        let bytes = response.bytes().await?;
        let cursor = Cursor::new(bytes);
        let feed = parser::parse(cursor)?;

        // Get first link or default to feed.id
        let feed_link = feed
            .links
            .first()
            .map_or(feed.id.clone(), |link| link.href.clone());

        // Get the base URL
        let parsed_url = url::Url::parse(feed_link.as_str())?;
        let base_url = if let Some(host) = parsed_url.host_str() {
            format!("{}://{}", parsed_url.scheme(), host)
        } else {
            feed_link.clone()
        };

        // Download the webpage to parse the HTML content.
        let channel_url = base_url.clone();
        let is_youtube = base_url.contains("youtube.com");
        let document = Html::parse_document(&reqwest::get(channel_url).await?.text().await?);

        // Get page title
        let mut title = if let Some(source_title) = source.title.clone() {
            source_title
        } else if is_youtube && feed.title.is_some() {
            feed.title.unwrap().content
        } else {
            document
                .select(&Selector::parse("title").unwrap())
                .next()
                .map(|element| element.inner_html())
                .unwrap_or_default()
        };

        // Get favicon
        let mut favicon = if let Some(source_icon) = source.icon.clone() {
            source_icon
        } else {
            url::Url::parse(&base_url)?
                .join("/favicon.ico")?
                .to_string()
        };
        // Youtube specific title and icon with piped
        if is_youtube {
            println!("Youtube channel detected, using piped to get title and icon");
            let channel_id = source.rss_url.split('=').last().unwrap().to_string();
            let client = PipedClient::new(&Client::new(), PIPED_INSTANCE);
            let channel = client.channel_from_id(channel_id).await.unwrap();

            title = channel.name;
            favicon = channel.avatar_url;
        }

        // Extract dominant color
        let dominant_color = if let Some(source_dominant_color) = source.dominant_color.clone() {
            source_dominant_color
        } else {
            // Extract the dominant color from the image
            get_dominant_color(
                &image::load_from_memory(&reqwest::get(&favicon).await?.bytes().await?)
                    .map_err(|_| format!("Failed to decode the image from {}", &favicon))?,
            )
            .unwrap_or("#000000".to_string())
        };

        // Construct the Channel object.
        let channel = Channel {
            rss_url: source.rss_url.clone(),
            category: source.category.clone().unwrap_or_default(),
            title,
            icon: favicon,
            dominant_color,
        };
        let channel_optional = ChannelOptional {
            rss_url: channel.rss_url.clone(),
            category: Some(channel.category.clone()),
            title: Some(channel.title.clone()),
            icon: Some(channel.icon.clone()),
            dominant_color: Some(channel.dominant_color.clone()),
        };

        // Store the newly constructed channel in the database.
        println!("Channel: {channel:?}");
        store_channel_to_db(db, &channel, &source.rss_url)?;

        Ok(channel_optional)
    } else {
        // Store the fed channel in the database.
        let channel = Channel {
            rss_url: source.rss_url.clone(),
            category: source.category.clone().unwrap_or_default(),
            title: source.title.clone().unwrap_or_default(),
            icon: source.icon.clone().unwrap_or_default(),
            dominant_color: source.dominant_color.clone().unwrap_or_default(),
        };
        store_channel_to_db(db, &channel, &source.rss_url)?;

        Ok(source.clone())
    }
}

/// Get the dominant color from an image.
fn get_dominant_color(img: &DynamicImage) -> Option<String> {
    let mut color_count: HashMap<(u8, u8, u8), usize> = HashMap::new();

    for (_x, _y, pixel) in img.pixels() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];
        let a = pixel[3];

        // Skip transparent and pure white pixels
        if a == 0 || (r == 255 && g == 255 && b == 255) {
            continue;
        }

        let color = (r, g, b);
        match color_count.get_mut(&color) {
            Some(count) => *count += 1,
            None => {
                color_count.insert(color, 1);
            }
        }
    }

    color_count
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|((r, g, b), _)| format!("#{r:02x}{g:02x}{b:02x}"))
}
