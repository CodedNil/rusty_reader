use bincode::{deserialize, serialize};
use feed_rs::parser;
use image::{DynamicImage, GenericImageView};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::{collections::HashMap, io::Cursor, sync::Arc};

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug)]
pub struct Channel {
    pub link: String,
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
    let resp = reqwest::get(base_url.clone()).await?;
    let body = resp.text().await?;
    let document = Html::parse_document(&body);

    // Get page title
    let title_selector = Selector::parse("title").unwrap();
    let title = document
        .select(&title_selector)
        .next()
        .map(|element| element.inner_html())
        .unwrap_or_default();

    // Attempt to extract the favicon URL from the HTML.
    let favicon_selector = Selector::parse("link[rel=\"shortcut icon\"]")?;
    let favicon = document
        .select(&favicon_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .and_then(|relative_url| url::Url::parse(source).ok()?.join(relative_url).ok())
        .map(|url| url.to_string());

    // If no favicon URL is found in the HTML, try fetching from the root of the domain.
    let favicon = if favicon.is_none() {
        let base_url_obj = url::Url::parse(&base_url)?;
        let favicon_url = base_url_obj.join("/favicon.ico")?;
        Some(favicon_url.to_string())
    } else {
        favicon
    };

    // If a favicon URL is found, fetch the image and extract its color palette.
    let dominant_color = if let Some(favicon_url) = &favicon {
        // Fetch the favicon
        let resp = reqwest::get(favicon_url).await?;
        let bytes = resp.bytes().await?;

        // Decode the .ico
        let img = image::load_from_memory(&bytes)
            .map_err(|_| format!("Failed to decode the image from {favicon_url}"))?;

        // Extract the dominant color from the image
        get_dominant_color(&img)
    } else {
        None
    };

    // Construct the Channel object.
    let channel = Channel {
        link: base_url.to_string().clone(),
        title,
        icon: favicon.unwrap_or_default(),
        dominant_color: dominant_color.unwrap_or("#000000".to_string()),
    };

    println!("Channel: {channel:?}");

    // Store the newly constructed channel in the database.
    store_channel_to_db(&db, &channel, source)?;

    Ok(channel)
}

fn get_dominant_color(img: &DynamicImage) -> Option<String> {
    let mut color_count: HashMap<(u8, u8, u8), usize> = HashMap::new();

    for (_x, _y, pixel) in img.pixels() {
        let r = pixel[0];
        let g = pixel[1];
        let b = pixel[2];
        let a = pixel[3];

        // Skip transparent pixels
        if a == 0 {
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
