use std::process::{Command};
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

    /// Merge multiple audio chunks into a single file
    pub fn merge_chunks(&self, chunks: Vec<PathBuf>, output_path: PathBuf) -> Result<PathBuf, std::io::Error> {
        // Create a temporary concat file
        let concat_file = self.create_concat_file(&chunks)?;
        
        // Run FFmpeg concat command
        let output = Command::new(&self.binary_path)
            .args(&[
                "-f", "concat",           // Use concat demuxer
                "-safe", "0",             // Allow absolute paths
                "-i", concat_file.to_str().unwrap(),
                "-c", "copy",             // Copy codec (no re-encoding)
                output_path.to_str().unwrap()
            ])
            .output()?;

        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&output.stderr).to_string()
            ));
        }

        Ok(output_path)
    }

    /// Creates a temporary file listing chunks to concatenate
    fn create_concat_file(&self, chunks: &[PathBuf]) -> Result<PathBuf, std::io::Error> {
        let concat_file = std::env::temp_dir().join("concat.txt");
        let mut content = String::new();
        
        // Format required by FFmpeg concat demuxer
        for chunk in chunks {
            content.push_str(&format!("file '{}'\n", 
                chunk.to_str().unwrap().replace("'", r"'\''")
            ));
        }
        
        std::fs::write(&concat_file, content)?;
        Ok(concat_file)
    }

    /// Split audio into three chunks based on a selected region
    /// Returns paths to the three resulting chunks:
    /// 1. Before selection (0 to start)
    /// 2. Selected region (start to end)
    /// 3. After selection (end to file_end)
    pub fn split_at_region(&self, start_time: f64, end_time: f64) -> Result<Vec<PathBuf>, std::io::Error> {
        let input = self.input_file.as_ref()
            .expect("Input file not set");
        let output_dir = self.output_dir.as_ref()
            .expect("Output directory not set");

        // Create three chunks:
        // 1. From start of file to start_time
        let chunk1 = self.extract_chunk(input, 0.0, start_time, output_dir.join("chunk_before.mp3"))?;
        
        // 2. From start_time to end_time (selected region)
        let chunk2 = self.extract_chunk(input, start_time, end_time, output_dir.join("chunk_selected.mp3"))?;
        
        // 3. From end_time to end of file
        let chunk3 = self.extract_chunk(input, end_time, -1.0, output_dir.join("chunk_after.mp3"))?;

        Ok(vec![chunk1, chunk2, chunk3])
    }

    /// Helper method to extract a portion of audio
    fn extract_chunk(&self, input: &PathBuf, start: f64, end: f64, output: PathBuf) -> Result<PathBuf, std::io::Error> {
        // Create string values that live long enough
        let start_str = start.to_string();
        let input_str = input.to_str().unwrap();
        let output_str = output.to_str().unwrap();
        
        // Create all arguments as a Vec<String> first
        let mut args = vec![
            "-i".to_string(),
            input_str.to_string(),
            "-ss".to_string(),
            start_str,
        ];

        // If end is -1, we go to the end of the file
        if end >= 0.0 {
            let duration_str = (end - start).to_string();
            args.extend_from_slice(&[
                "-t".to_string(),
                duration_str,
            ]);
        }

        // Add output arguments
        args.extend_from_slice(&[
            "-c".to_string(),
            "copy".to_string(),
            output_str.to_string(),
        ]);

        // Convert Vec<String> to Vec<&str> for the Command
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        let command_output = Command::new(&self.binary_path)
            .args(&args_ref)
            .output()?;

        if !command_output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                String::from_utf8_lossy(&command_output.stderr).to_string()
            ));
        }

        Ok(output)
    }
} 