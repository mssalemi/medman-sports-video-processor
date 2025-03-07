use std::process::Command;
use std::path::PathBuf;

pub struct WhisperClient {
    binary_path: String,
    model: String,
}

#[derive(Debug)]
pub struct TranscriptionResult {
    pub segments: Vec<Segment>,
}

#[derive(Debug)]
pub struct Segment {
    pub start: f64,
    pub end: f64,
    pub text: String,
}

impl WhisperClient {
    pub fn new() -> Self {
        Self {
            binary_path: "whisper".to_string(),
            model: "base".to_string(),
        }
    }

    pub fn transcribe(&self, input_path: &PathBuf) -> Result<TranscriptionResult, std::io::Error> {
        println!("Attempting to transcribe file: {:?}", input_path);
        
        let input_str = input_path.to_str()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid path"
            ))?;

        // Create videos directory for output
        let videos_dir = std::env::current_dir()?.join("src").join("videos");
        std::fs::create_dir_all(&videos_dir)?;
        
        println!("Running whisper command with path: {}", input_str);
        
        let output = Command::new(&self.binary_path)
            .arg(input_str)
            .args(&["--model", &self.model])
            .args(&["--output_dir", videos_dir.to_str().unwrap()])  // Add output directory
            .output()?;

        println!("Whisper command executed with status: {:?}", output.status);
        
        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            println!("Whisper error: {}", error);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string()
            ));
        }

        // Print the stdout to see what we're getting
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Whisper output: {}", stdout);

        let segments = self.parse_output(&stdout);
        println!("Parsed segments: {:?}", segments);

        Ok(TranscriptionResult { segments })
    }

    fn parse_output(&self, output: &str) -> Vec<Segment> {
        output
            .lines()
            .filter_map(|line| {
                if let Some(timing_and_text) = line.split_once(']') {
                    if let Some(times) = timing_and_text.0.trim_start_matches('[').split_once("-->") {
                        let start = parse_timestamp(times.0.trim());
                        let end = parse_timestamp(times.1.trim());
                        let text = timing_and_text.1.trim().to_string();
                        
                        Some(Segment { start, end, text })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

fn parse_timestamp(timestamp: &str) -> f64 {
    let parts: Vec<&str> = timestamp.split(':').collect();
    if parts.len() == 2 {
        let minutes: f64 = parts[0].parse().unwrap_or(0.0);
        let seconds: f64 = parts[1].parse().unwrap_or(0.0);
        minutes * 60.0 + seconds
    } else {
        0.0
    }
} 