mod ffmpeg;
mod whisper;
mod openai;

use axum::{
    routing::get,
    Router,
    response::Json,
    extract::Query,
};
use serde_json::{json, Value};
use ffmpeg::FFmpegClient;
use whisper::WhisperClient;
use std::path::PathBuf;
use openai::OpenAIClient;
use serde::Deserialize;
use dotenv::dotenv;
use openai::PromptTemplate;

async fn hello() -> Json<Value> {
    println!("Hello, World!");
    Json(json!({ "message": "Hello, World!" }))
}

async fn media_info() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let audio_path = current_dir.join("src").join("audio.mp3");
    
    let mut ffmpeg = FFmpegClient::new();
    let info = ffmpeg
        .with_input(audio_path.to_str().unwrap())
        .get_info()
        .expect("Failed to execute ffmpeg");

    println!("Request Success");
    println!("Media Info: {:?}", info);

    Json(json!({ 
        "file": audio_path.to_str().unwrap(),
        "duration": info.duration,
        "format": info.format,
        "bitrate": info.bitrate,
        "sample_rate": "44100 Hz",
        "channels": "stereo"
    }))
}

async fn split_video() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let chunks_dir = current_dir.join("src").join("chunks");
    let audio_path = current_dir.join("src").join("audio.mp3");

    // Create chunks directory if it doesn't exist
    std::fs::create_dir_all(&chunks_dir).expect("Failed to create chunks directory");

    let mut ffmpeg = FFmpegClient::new();
    let chunks = ffmpeg
        .with_input(audio_path.to_str().unwrap())
        .with_output_dir(chunks_dir.to_str().unwrap())
        .with_chunk_duration(2)  // 2 seconds
        .split_into_chunks()
        .expect("Failed to split video");

    println!("Request Success");
    println!("chunks {:?}", chunks);

    Json(json!({
        "message": "Audio split successfully",
        "chunks": chunks
            .iter()
            .map(|p| p.to_str().unwrap())
            .collect::<Vec<_>>()
    }))
}

async fn merge_chunks() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let chunks_dir = current_dir.join("src").join("chunks");
    let output_path = current_dir.join("src").join("merged.mp3");

    // Get list of chunks (you might want to specify order differently)
    let chunks: Vec<PathBuf> = std::fs::read_dir(&chunks_dir)
        .expect("Failed to read chunks directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("mp3"))
        .collect();

    let ffmpeg = FFmpegClient::new();
    let merged_file = ffmpeg
        .merge_chunks(chunks, output_path)
        .expect("Failed to merge chunks");

    Json(json!({
        "message": "Chunks merged successfully",
        "output_file": merged_file.to_str().unwrap()
    }))
}

async fn split_region() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let chunks_dir = current_dir.join("src").join("chunks");
    let audio_path = current_dir.join("src").join("audio.mp3");

    std::fs::create_dir_all(&chunks_dir).expect("Failed to create chunks directory");

    let mut ffmpeg = FFmpegClient::new();
    let chunks = ffmpeg
        .with_input(audio_path.to_str().unwrap())
        .with_output_dir(chunks_dir.to_str().unwrap())
        .split_at_region(1.0, 1.5)  // Back to hardcoded values
        .expect("Failed to split region");

    Json(json!({
        "message": "Audio split by region successfully",
        "chunks": chunks
            .iter()
            .map(|p| p.to_str().unwrap())
            .collect::<Vec<_>>()
    }))
}

async fn transcribe() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let input_path = current_dir.join("src").join("video.mov");

    println!("Looking for file at: {:?}", input_path);
    
    // Verify file exists
    if !input_path.exists() {
        println!("File not found!");
        return Json(json!({
            "error": "File not found",
            "path": input_path.to_str()
        }));
    }

    let whisper = WhisperClient::new();
    let transcription = match whisper.transcribe(&input_path) {
        Ok(t) => t,
        Err(e) => {
            return Json(json!({
                "error": "Transcription failed",
                "details": e.to_string()
            }));
        }
    };

    Json(json!({
        "segments": transcription.segments.iter().map(|segment| {
            json!({
                "start": segment.start,
                "end": segment.end,
                "text": segment.text
            })
        }).collect::<Vec<_>>()
    }))
}

async fn transcribe_to_json() -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let input_path = current_dir.join("src").join("video.mov");

    println!("Looking for file at: {:?}", input_path);
    
    if !input_path.exists() {
        println!("File not found!");
        return Json(json!({
            "error": "File not found",
            "path": input_path.to_str()
        }));
    }

    let whisper = WhisperClient::new();
    let transcription = whisper
        .transcribe(&input_path)
        .expect("Failed to transcribe audio");

    // Concatenate all segments into one text
    let full_text: String = transcription.segments
        .iter()
        .map(|segment| segment.text.clone())
        .collect::<Vec<String>>()
        .join(" ");

    let word_count = full_text.split_whitespace().count();

    Json(json!({
        "text": full_text,
        "word_count": word_count,
        "duration_seconds": transcription.segments.last().map(|s| s.end).unwrap_or(0.0)
    }))
}

#[derive(Deserialize)]
struct TranscribeQuery {
    template: Option<String>,
}

async fn transcribe_and_optimize(Query(params): Query<TranscribeQuery>) -> Json<Value> {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let input_path = current_dir.join("src").join("video.mov");

    println!("Looking for file at: {:?}", input_path);
    
    if !input_path.exists() {
        println!("File not found!");
        return Json(json!({
            "error": "File not found",
            "path": input_path.to_str()
        }));
    }

    // First, transcribe the video
    let whisper = WhisperClient::new();
    let transcription = match whisper.transcribe(&input_path) {
        Ok(t) => t,
        Err(e) => {
            return Json(json!({
                "error": "Transcription failed",
                "details": e.to_string()
            }));
        }
    };

    // Combine all segments into one text
    let full_text: String = transcription.segments
        .iter()
        .map(|segment| segment.text.clone())
        .collect::<Vec<String>>()
        .join(" ");

    // Then optimize the content using OpenAI
    let openai = OpenAIClient::new()
        .expect("Failed to create OpenAI client");

    let template = match params.template.as_deref() {
        Some("mama-meditations") => PromptTemplate::MamaMeditations,
        Some("med-man-sports") => PromptTemplate::MedManSports,
        _ => PromptTemplate::MedManSports,
    };

    match openai.generate_youtube_content(&full_text, template).await {
        Ok(response) => {
            match serde_json::from_str::<Value>(&response) {
                Ok(content) => Json(content),
                Err(e) => {
                    println!("Failed to parse response as JSON: {}", e);
                    Json(json!({
                        "error": "Failed to parse YouTube content",
                        "raw_response": response
                    }))
                }
            }
        },
        Err(e) => {
            println!("YouTube content generation failed: {}", e);
            Json(json!({
                "error": format!("YouTube content generation failed: {}", e)
            }))
        }
    }
}

#[derive(Deserialize)]
struct PromptQuery {
    text: String,
}

async fn chat(Query(params): Query<PromptQuery>) -> Json<Value> {
    let client = OpenAIClient::new()
        .expect("Failed to create OpenAI client");

    match client.complete(&params.text).await {
        Ok(response) => Json(json!({
            "response": response
        })),
        Err(e) => Json(json!({
            "error": e.to_string()
        }))
    }
}

#[derive(Deserialize)]
struct GenerateQuery {
    template: Option<String>,
}

async fn generate_test_content(Query(params): Query<GenerateQuery>) -> Json<Value> {
    let client = OpenAIClient::new()
        .expect("Failed to create OpenAI client");

    println!("Starting YouTube content generation..."); 

    let test_transcript = "Okay, what is up Medman Sports? Today we're talking about tennis serves...";

    let template = match params.template.as_deref() {
        Some("mama-meditations") => PromptTemplate::MamaMeditations,
        Some("med-man-sports") => PromptTemplate::MedManSports,
        _ => PromptTemplate::MedManSports,
    };

    match client.generate_youtube_content(test_transcript, template).await {
        Ok(response) => {
            match serde_json::from_str::<Value>(&response) {
                Ok(content) => Json(content),
                Err(e) => {
                    println!("Failed to parse response as JSON: {}", e);
                    Json(json!({
                        "error": "Failed to parse YouTube content",
                        "raw_response": response
                    }))
                }
            }
        },
        Err(e) => {
            println!("YouTube content generation failed: {}", e);
            Json(json!({
                "error": format!("YouTube content generation failed: {}", e)
            }))
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenv().ok();

    // Build our router
    let app = Router::new()
        .route("/hello", get(hello))
        .route("/media/info", get(media_info))
        .route("/split", get(split_video))
        .route("/merge", get(merge_chunks))
        .route("/split-region", get(split_region))
        .route("/transcribe", get(transcribe))
        .route("/transcribe-to-json", get(transcribe_to_json))
        .route("/transcribe-and-optimize", get(transcribe_and_optimize))
        .route("/chat", get(chat))
        .route("/generate", get(generate_test_content));

    // Run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Server running on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await.unwrap();
}
