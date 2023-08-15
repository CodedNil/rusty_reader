use crate::gpt;
use base64::{engine::general_purpose, Engine};
use chrono::Utc;
use image::ImageOutputFormat;
use std::{
    fs::{create_dir_all, File},
    time::SystemTime,
};

const URL: &str =
    "https://api.stability.ai/v1/generation/stable-diffusion-xl-1024-v1-0/text-to-image";

#[allow(dead_code)]
pub enum WriteOption {
    None,
    Desktop,
    Mobile,
}

async fn fetch_weather() -> Result<String, Box<dyn std::error::Error>> {
    let response: serde_json::Value = reqwest::get("https://api.open-meteo.com/v1/forecast?latitude=52.6369&longitude=-1.1398&current_weather=true").await?.json().await?;
    let temperature = response["current_weather"]["temperature"]
        .as_f64()
        .unwrap_or(0.0);
    let weathercode = response["current_weather"]["weathercode"]
        .as_u64()
        .unwrap_or(0);

    let weather_description = match weathercode {
        0 => "Clear sky",
        1..=3 => "Mainly clear to overcast",
        45 | 48 => "Foggy conditions",
        51..=55 => "Drizzle",
        56..=57 => "Freezing Drizzle",
        61..=65 => "Rainy",
        66..=67 => "Freezing Rain",
        71..=75 => "Snow fall",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85..=86 => "Snow showers",
        95 => "Thunderstorm: Slight or moderate",
        96 | 99 => "Thunderstorm with hail",
        _ => "Unknown weather",
    };

    Ok(format!("{temperature:.2}°C, {weather_description}"))
}

pub async fn generate_prompt() -> Result<String, Box<dyn std::error::Error>> {
    let current_datetime = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let weather = fetch_weather().await?;

    let input = format!(
        "Using this data Date: {current_datetime}, Weather: {weather}, format a prompt for an image generator which creates a scenic wallpaper, season and seasonal/cultural events included for UK, be very concise and create a short prompt, the prompt should be a description of the wallpaper written like these examples 'Sunny afternoon in a british field wallpaper' 'Sunset over a foggy city wallpaper' 'Snowy night in a forest wallpaper', be creative and use interesting art styles"
    );
    gpt::process(input, "gpt-4", 128u16).await
}

pub async fn generate_image(
    prompt: &str,
    width: u32,
    height: u32,
    write_option: WriteOption,
) -> Result<(), Box<dyn std::error::Error>> {
    let body = serde_json::json!({
        "steps": 50,
        "width": width,
        "height": height,
        "seed": 0,
        "cfg_scale": 7,
        "samples": 1,
        "style_preset": "enhance",
        "text_prompts": [{
            "text": prompt,
            "weight": 1
        }]
    });

    let client = reqwest::Client::new();
    let api_key =
        std::env::var("STABLE_DIFFUSION_API_KEY").expect("STABLE_DIFFUSION_API_KEY not set");
    let response: serde_json::Value = client
        .post(URL)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {api_key}"))
        .json(&body)
        .send()
        .await?
        .json()
        .await?;

    // Ensure the wallpapers directory exists
    create_dir_all("./wallpapers")?;

    let date = SystemTime::now();
    let datetime = chrono::DateTime::<chrono::Utc>::from(date);
    let formatted_date = datetime.format("%d.%m");

    for image in response["artifacts"].as_array().unwrap() {
        let base64_str = image["base64"].as_str().unwrap();
        let decoded = general_purpose::STANDARD.decode(base64_str)?;
        let img = image::load_from_memory(&decoded)?;

        // Save as WebP
        let sanitized_prompt = prompt
            .chars()
            .filter_map(|c| match c {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => Some(c.to_ascii_lowercase()),
                ' ' => Some('_'),
                _ => None,
            })
            .collect::<String>()
            .chars()
            .take(50)
            .collect::<String>();
        let file_name = format!("./wallpapers/{formatted_date}-{sanitized_prompt}.webp");
        let mut output = File::create(&file_name)?;
        img.write_to(&mut output, ImageOutputFormat::WebP)?;

        match write_option {
            WriteOption::Desktop => {
                create_dir_all("./assets")?;
                let mut bg_output = File::create("./assets/background.webp")?;
                img.write_to(&mut bg_output, ImageOutputFormat::WebP)?;
            }
            WriteOption::Mobile => {
                create_dir_all("./assets")?;
                let mut bg_mobile_output = File::create("./assets/background_mobile.webp")?;
                img.write_to(&mut bg_mobile_output, ImageOutputFormat::WebP)?;
            }
            WriteOption::None => {}
        }
    }

    Ok(())
}
