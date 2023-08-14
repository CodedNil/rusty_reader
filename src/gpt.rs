use async_openai::{
    types::{ChatCompletionRequestMessageArgs, CreateChatCompletionRequestArgs, Role},
    Client,
};
use bincode::{deserialize, serialize};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Summary {
    pub title: String,
    pub summary: String,
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

    // Crop text to fit the limit of 4096 tokens (3 characters to a token on average) if necessary
    let text = if text.len() > 4096 * 3 {
        let mut text = text;
        text.truncate(4096 * 3);
        text
    } else {
        text
    };

    // Use GPT3.5 to summarise the article and title
    let client = Client::new();
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        .model("gpt-3.5-turbo")
        .messages([
            ChatCompletionRequestMessageArgs::default()
                .role(Role::User)
                // .content(format!("Summarize the following article and provide the summary in JSON format. The JSON should have keys 'title' (concisely rephrased from the original) and 'summary' (concisely rephrased from the original text, but still written in article format).\nOriginal title: {title}\nOriginal text: {text}
                .content(format!("Provide a concise summary of the following article in JSON format. The JSON should have keys 'title' (rephrased from the original for brevity) and 'summary' (condensed from the original text, maintaining the tone and style of the original author). Ensure the summary includes all relevant context so that someone unfamiliar with the topic can understand.\nOriginal title: {title}\nOriginal text: {text}"))
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
        Ok(summary) => summary,
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
