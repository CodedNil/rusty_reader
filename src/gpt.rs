use crate::articles::Summary;
use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use bincode::{deserialize, serialize};
use sled::Db;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::{collections::hash_map::DefaultHasher, sync::Arc};

// Helper function to compute the hash of a string
fn compute_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub async fn summarise_article(
    db: Arc<Db>,
    title: String,
    text: String,
) -> Result<Summary, Box<dyn Error>> {
    // Attempt to retrieve the summary from the database, return that if found
    let key = format!("summary:{}", compute_hash(&format!("{title}{text}")));
    if db.contains_key(key.clone())? {
        let ivec = db.get(key.clone())?.unwrap();
        let summary: Summary = deserialize(&ivec)?;
        return Ok(summary);
    }

    // If the text is long it requires higher context model
    let is_lengthy = text.len() > 4096 * 3;
    let model = if is_lengthy {
        "gpt-3.5-turbo-16k"
    } else {
        "gpt-3.5-turbo"
    };

    // If its still too long (over 30k characters), error out
    if text.len() > 16384 * 3 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Text too long",
        )));
    };

    // Use GPT3.5 to summarise the article and title
    let client = Client::new();

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(1024u16)
        .model(model)
        .messages([
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                .content(format!("Provide a concise summary of the following content in JSON format. If it's an article, use the provided text. If it's a video, use the provided subtitles. The JSON should have keys 'title' (rephrased from the original for brevity) and 'summary' (condensed from the original content, maintaining the tone and style of the original). Ensure the summary includes all relevant context so that someone unfamiliar with the topic can understand.\nOriginal title: {title}\nOriginal content: {text}"))
                .build()?,
        ])
        .build()?;
    let response = client.chat().create(request).await?;
    let result = response
        .choices
        .first()
        .unwrap()
        .message
        .content
        .clone()
        .unwrap();

    // Parse json, with error handling
    let result: Summary = match serde_json::from_str(&result) {
        Ok(summary) => {
            println!("Successfully parsed GPT3.5 response for {title}");
            summary
        }
        Err(e) => {
            println!("Error parsing GPT3.5 response: {e}");
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Error parsing GPT3.5 response",
            )));
        }
    };

    // Store the summary in the database
    let ivec = serialize(&result)?;
    db.insert(key, ivec)?;
    db.flush()?;

    Ok(result)
}

pub async fn process(
    input: String,
    model: &str,
    max_tokens: u16,
) -> Result<String, Box<dyn Error>> {
    // Use GPT3.5 to summarise the article and title
    let client = Client::new();
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(max_tokens)
        .model(model)
        .messages([ChatCompletionRequestMessageArgs::default()
            .role(Role::User)
            .content(input)
            .build()?])
        .build()?;
    let response = client.chat().create(request).await?;
    let result = response
        .choices
        .first()
        .unwrap()
        .message
        .content
        .clone()
        .unwrap();

    Ok(result)
}
