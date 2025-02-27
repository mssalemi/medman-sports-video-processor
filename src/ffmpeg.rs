use std::process::{Command, Output};
use std::path::PathBuf;
use std::time::Duration;

pub struct FFmpegClient {
    binary_path: String,
    input_file: Option<PathBuf>,
    output_dir: Option<PathBuf>,
    chunk_duration: Option<Duration>,
}

#[derive(Debug)]
pub struct MediaInfo {
    pub duration: String,
    pub bitrate: String,
    pub format: String,
}

impl FFmpegClient {
    /// Create a new FFmpeg client
    pub fn new() -> Self {
        Self {
            binary_path: "ffmpeg".to_string(),
            input_file: None,
            output_dir: None,
            chunk_duration: None,
        }
    }

    /// Set the input file
    pub fn with_input<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.input_file = Some(path.into());
        self
    }

    /// Set the output directory
    pub fn with_output_dir<P: Into<PathBuf>>(&mut self, path: P) -> &mut Self {
        self.output_dir = Some(path.into());
        self
    }

    /// Set the chunk duration
    pub fn with_chunk_duration(&mut self, seconds: u64) -> &mut Self {
        self.chunk_duration = Some(Duration::from_secs(seconds));
        self
    }

    /// Get media info
    pub fn get_info(&self) -> Result<MediaInfo, std::io::Error> {
        let input = self.input_file.as_ref()
            .expect("Input file not set");

        let output = Command::new(&self.binary_path)
            .arg("-i")
            .arg(input)
            .output()?;

        // FFmpeg returns info on stderr with status 1 (by design)
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Parse the output into our MediaInfo struct
        let info = MediaInfo {
            duration: stderr
                .lines()
                .find(|line| line.contains("Duration:"))
                .and_then(|line| line.split("Duration: ").nth(1))
                .and_then(|time| time.split(',').next())
                .unwrap_or("unknown")
                .to_string(),
            bitrate: stderr
                .lines()
                .find(|line| line.contains("bitrate:"))
                .and_then(|line| line.split("bitrate: ").nth(1))
                .unwrap_or("unknown")
                .to_string(),
            format: stderr
                .lines()
                .find(|line| line.contains("Input #0,"))
                .and_then(|line| line.split("Input #0, ").nth(1))
                .and_then(|s| s.split(',').next())
                .unwrap_or("unknown")
                .to_string(),
        };

        Ok(info)
    }

    pub fn parse_info(output: &Output) -> MediaInfo {
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        let duration = stderr
            .lines()
            .find(|line| line.contains("Duration:"))
            .and_then(|line| line.split("Duration: ").nth(1))
            .and_then(|time| time.split(',').next())
            .unwrap_or("unknown");

        MediaInfo {
            duration: duration.to_string(),
            bitrate: "128 kb/s".to_string(),  // We can parse this too
            format: "mp3".to_string(),
        }
    }

    pub fn split_into_chunks(&self) -> Result<Vec<PathBuf>, std::io::Error> {
        let input = self.input_file.as_ref()
            .expect("Input file not set");
        let output_dir = self.output_dir.as_ref()
            .expect("Output directory not set");
        let duration = self.chunk_duration
            .expect("Chunk duration not set");

        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        // Create the output path first so it lives long enough
        let output_path = output_dir.join("chunk_%03d.mp3");
        let output_str = output_path.to_str()
            .expect("Invalid output path");

        // Store input path as string
        let input_str = input.to_str()
            .expect("Invalid input path");

        // Store duration string so it lives long enough
        let duration_str = duration.as_secs().to_string();

        let args = vec![
            "-i",
            input_str,
            "-f", "segment",
            "-segment_time", &duration_str,
            "-c", "copy",
            "-reset_timestamps", "1",
            output_str,
        ];

        println!("Running command: ffmpeg {}", args.join(" "));

        let output = Command::new(&self.binary_path)
            .args(&args)
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        // Get list of created chunks, sorted
        let mut chunks: Vec<PathBuf> = std::fs::read_dir(output_dir)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                // Only include numbered chunks, not the pattern file
                path.extension().and_then(|s| s.to_str()) == Some("mp3") && 
                path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| !s.contains("%03d"))
                    .unwrap_or(false)
            })
            .collect();

        // Sort chunks by name
        chunks.sort();

        if chunks.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No chunks were created".to_string()
            ));
        }

        Ok(chunks)
    }
} 