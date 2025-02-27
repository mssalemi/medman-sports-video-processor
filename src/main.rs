mod ffmpeg;

use axum::{
    routing::get,
    Router,
    response::Json,
};
use serde_json::{json, Value};
use ffmpeg::FFmpegClient;
use axum::extract::rejection::*;

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

#[tokio::main]
async fn main() {
    // Build our router
    let app = Router::new()
        .route("/hello", get(hello))
        .route("/media/info", get(media_info))
        .route("/split", get(split_video));

    // Run the server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Server running on http://127.0.0.1:3000");
    
    axum::serve(listener, app).await.unwrap();
}
