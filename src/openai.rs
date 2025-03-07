use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

pub struct OpenAIClient {
    client: reqwest::Client,
    api_key: String,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug)]
pub enum PromptTemplate {
    MedManSports,
    MamaMeditations
}

impl OpenAIClient {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let api_key = env::var("OPENAI_API_KEY")
            .expect("OPENAI_API_KEY must be set");

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            api_key,
        })
    }

    pub async fn complete(&self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        let request = ChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request)
            .send()
            .await?
            .json::<ChatResponse>()
            .await?;

        Ok(response.choices[0].message.content.clone())
    }

    pub async fn generate_youtube_content(
        &self, 
        transcript: &str,
        template: PromptTemplate
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Get the appropriate template file
        let template_path = match template {
            PromptTemplate::MedManSports => "prompt_dev/medmansports.md",
            PromptTemplate::MamaMeditations => "prompt_dev/mamameditation.md",
        };

        println!("Using template: {}", template_path);

        // Read the prompt template
        let prompt_template = fs::read_to_string(template_path)
            .expect("Failed to read prompt template");

        // Replace the placeholder with actual transcript
        let prompt = prompt_template.replace("{{TRANSCRIPT_TEXT}}", transcript);

        println!("Sending prompt to OpenAI..."); // Debug line

        let request = ChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![ChatMessage {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&request)
            .send()
            .await?;

        // Debug response status
        println!("OpenAI Response Status: {}", response.status());

        if !response.status().is_success() {
            let error_text = response.text().await?;
            println!("OpenAI Error: {}", error_text);
            return Err(format!("OpenAI request failed: {}", error_text).into());
        }

        let response_json = response.json::<ChatResponse>().await?;
        
        if response_json.choices.is_empty() {
            return Err("No response choices returned".into());
        }

        Ok(response_json.choices[0].message.content.clone())
    }
} 