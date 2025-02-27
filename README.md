# Med Man Sports - FFmpeg Audio Processing API

A Rust web service that provides FFmpeg functionality through HTTP endpoints. This project demonstrates various Rust concepts through practical implementation of audio file processing.

## Features
- Get media file information
- Split audio files into chunks
- RESTful API using Axum

## Rust Concepts Demonstrated

### Basic Concepts
- **Modules**: Using `mod` to organize code (`mod ffmpeg`)
- **Structs**: Custom types like `FFmpegClient` and `MediaInfo`
- **Enums**: Used in error handling
- **Implementations**: Using `impl` blocks for struct methods
- **String vs &str**: String ownership in FFmpeg paths and commands

### Ownership & Borrowing
- **References**: Using `&self` and `&mut self` in methods
- **Lifetimes**: Managing string lifetimes in command arguments
- **Ownership**: Moving and borrowing PathBuf and String values
- **Temporary Values**: Handling lifetimes of temporary strings and paths

### Traits
- **Into<T>**: Generic constraints for flexible path inputs
- **AsRef<Path>**: Path reference conversions
- **Debug**: Derived debug printing for structs

### Error Handling
- **Result Type**: Returning `Result<T, E>` from FFmpeg operations
- **Option Type**: Handling optional fields like `input_file`
- **Unwrap/Expect**: Handling potential None values

### Modern Rust Patterns
- **Builder Pattern**: Fluent interface for FFmpeg client configuration
- **Method Chaining**: `with_input()`, `with_output_dir()`, etc.
- **Type Safety**: Strong typing for FFmpeg operations

### Async Programming
- **Async/Await**: Asynchronous HTTP endpoints
- **Tokio**: Async runtime for web server
- **Future**: Async function returns

### File System Operations
- **PathBuf**: Path manipulation for files
- **std::fs**: Directory creation and file listing
- **File Extensions**: Filtering files by extension

### Command Execution
- **std::process::Command**: Running FFmpeg commands
- **Command Builder**: Constructing complex FFmpeg commands
- **Output Parsing**: Processing FFmpeg stderr output

### Web Framework (Axum)
- **Routing**: HTTP endpoint definition
- **JSON Responses**: Using serde_json for API responses
- **HTTP Server**: TCP listener setup

## Project Structure

### Routes
- `GET /hello` - Basic health check endpoint
- `GET /media/info` - Get media file information
- `GET /split` - Split audio into chunks

### Implementation Details

#### FFmpeg Integration
Currently uses CLI-based FFmpeg execution:
- Uses `std::process::Command` to run FFmpeg commands
- Parses FFmpeg output from stderr
- No direct FFmpeg library bindings

#### FFmpegClient Structure

```rust
pub struct FFmpegClient {
binary_path: String, // Path to FFmpeg executable
input_file: Option<PathBuf>, // Input file to process
output_dir: Option<PathBuf>, // Output directory for chunks
chunk_duration: Option<Duration>, // Duration for splitting
}
```

#### Split Operation
- Creates chunks of specified duration
- Uses FFmpeg segment feature
- Output format: `chunk_000.mp3`, `chunk_001.mp3`, etc.
- Maintains original audio codec and quality

### Future Improvements
1. Switch to FFmpeg library bindings for better performance
2. Add more audio processing features
3. Implement proper error handling
4. Add configuration options
5. Add progress tracking for long operations

### Dependencies

```toml
[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde_json = "1.0"
```

### Requirements
- FFmpeg installed and available in PATH
- Rust 1.75 or later
- Available port 3000

### Next Steps: Speech-to-Text Integration

#### Planned Features
1. **Speech-to-Text Processing**
   - Process audio chunks in parallel
   - Output timestamped transcriptions
   - Support multiple languages

#### Potential Implementations

1. **Whisper Integration** (Recommended)
```rust
// Using whisper-rs crate
use whisper_rs::{WhisperContext, WhisperContextParameters};

pub struct TranscriptionResult {
    text: String,
    timestamp: f32,
    confidence: f32,
}
```

2. **Cloud Services Option**
   - Google Cloud Speech-to-Text
   - AWS Transcribe
   - Azure Speech Services

3. **Local Processing**
   - Vosk (offline speech recognition)
   - Mozilla DeepSpeech

#### New Endpoints to Add
```
POST /transcribe
- Input: Audio file
- Output: JSON with transcription chunks

GET /transcribe/{id}/status
- Check transcription progress

GET /transcribe/{id}/result
- Get final transcription with timestamps
```

#### Data Structure
```json
{
    "chunks": [
        {
            "start_time": "00:00:00",
            "end_time": "00:00:02",
            "text": "transcribed text here",
            "confidence": 0.95
        }
    ],
    "metadata": {
        "language": "en",
        "model": "whisper-large",
        "total_duration": "00:03:24"
    }
}
```

#### Technical Considerations
1. **Processing Pipeline**
   - Audio splitting (current functionality)
   - Parallel transcription of chunks
   - Result aggregation and timestamp alignment

2. **Performance Optimization**
   - Chunk caching
   - Background processing
   - Progress tracking

3. **Error Handling**
   - Audio quality issues
   - Language detection
   - Failed transcriptions

4. **Storage**
   - Temporary chunk storage
   - Transcription results
   - Processing status

#### Transcriber Abstraction Design
1. **Trait-Based Design**
```rust
// Core trait for different transcription implementations
trait Transcriber {
    fn transcribe(&self, audio: AudioChunk) -> Result<TranscriptionResult, TranscriberError>;
}
```

2. **Potential Implementations**
   - `WhisperTranscriber`: Local processing using Whisper
   - `GoogleTranscriber`: Cloud-based using Google API
   - `VoskTranscriber`: Offline processing using Vosk
   - `MockTranscriber`: For testing purposes

3. **Integration with Current System**
   - Extend FFmpegClient to work with Transcriber implementations
   - Process chunks in parallel using tokio
   - Aggregate results maintaining timestamp order

4. **Benefits**
   - Swap transcription backends without changing core logic
   - Easy to test with mock implementation
   - Consistent interface for all transcription methods
   - Simple to add new transcription services

5. **Considerations**
   - Error handling across different implementations
   - Consistent result format
   - Performance characteristics
   - Resource management (memory, API quotas)
   - Async processing coordination
```

### Setup Instructions

1. **Prerequisites**
   - Rust (1.75 or later)
   - FFmpeg

2. **Install FFmpeg**
   ```bash
   # macOS
   brew install ffmpeg

   # Ubuntu/Debian
   sudo apt-get install ffmpeg

   # Windows (with Chocolatey)
   choco install ffmpeg
   ```

3. **Verify FFmpeg Installation**
   ```bash
   ffmpeg -version
   ```

4. **Project Setup**
   ```bash
   # Create required directories
   mkdir -p src/chunks

   # Build and run
   cargo run
   ```

5. **Test File**
   - Place an `audio.mp3` file in the `src` directory for testing
   - Or update the path in `main.rs` to point to your audio file

### Troubleshooting

1. **FFmpeg Not Found**
   - Ensure FFmpeg is in your system PATH
   - Try running `ffmpeg` in terminal
   - Restart your terminal after installation

2. **Common Issues**
   - Missing `src/chunks` directory
   - Missing test audio file
   - Port 3000 already in use (change in `main.rs`)

3. **Verification**
   ```bash
   # Test the API
   curl http://localhost:3000/hello
   curl http://localhost:3000/media/info
   ```
# medman-sports-video-processor
