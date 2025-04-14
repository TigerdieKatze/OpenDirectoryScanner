use clap::{Arg, Command};
use image::ImageReader;
use nsfw::{create_model, examine};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;
use std::time::Duration;

struct NSFWDetector {
    model: nsfw::Model,
}

impl NSFWDetector {
    fn new() -> Result<Self, Box<dyn Error>> {
        // load file from project directory
        let model_data = include_bytes!("../resources/model.onnx");
        let model = create_model(&model_data[..])?;

        Ok(NSFWDetector { model })
    }

    fn is_nsfw(&self, image_url: &str, client: &Client) -> Result<bool, Box<dyn Error>> {
        // Download the image
        let response = client.get(image_url).send()?;
        if !response.status().is_success() {
            return Err(format!("Failed to download image: {}", response.status()).into());
        }

        let image_data = response.bytes()?;
        let img = ImageReader::new(Cursor::new(image_data))
            .with_guessed_format()?
            .decode()?;

        // Convert to Rgba8 format as required by the examine function
        let rgba_img = img.to_rgba8();

        // Analyze the image
        let result = examine(&self.model, &rgba_img)?;

        // Print the results for debugging
        println!("NSFW results for {}: {:?}", image_url, result);

        // Check if any of the explicit categories have high scores
        let check_results = result
            .iter()
            .any(|classification| match classification.metric {
                nsfw::model::Metric::Hentai => classification.score > 0.6,
                nsfw::model::Metric::Porn => classification.score > 0.5,
                nsfw::model::Metric::Sexy => classification.score > 0.8,
                _ => false,
            });

        Ok(check_results)
    }
}

#[derive(Debug, Clone)]
struct FileInfo {
    name: String,
    url: String,
    size: u64,
    file_type: FileType,
    #[allow(dead_code)]
    is_nsfw: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
enum FileType {
    Directory,
    Image(String),    // format
    Video(String),    // format
    Audio(String),    // format
    Document(String), // format
    Other(String),    // extension
}

struct DirectoryReport {
    total_files: usize,
    total_directories: usize,
    total_size: u64,
    files_by_type: HashMap<String, usize>,
    image_count: usize,
    video_count: usize,
    audio_count: usize,
    document_count: usize,
    other_count: usize,
    nsfw_count: usize,
    largest_file: Option<FileInfo>,
    largest_directory: Option<(String, u64)>,
}

impl DirectoryReport {
    fn new() -> Self {
        DirectoryReport {
            total_files: 0,
            total_directories: 0,
            total_size: 0,
            files_by_type: HashMap::new(),
            image_count: 0,
            video_count: 0,
            audio_count: 0,
            document_count: 0,
            other_count: 0,
            nsfw_count: 0,
            largest_file: None,
            largest_directory: None,
        }
    }

    fn print(&self) {
        println!("=== Directory Scan Report ===");
        println!("Total files: {}", self.total_files);
        println!("Total directories: {}", self.total_directories);
        println!(
            "Total size: {} bytes ({} MB)",
            self.total_size,
            self.total_size / 1_048_576
        );

        println!("\n=== File Type Breakdown ===");
        println!("Images: {} files", self.image_count);
        println!("Videos: {} files", self.video_count);
        println!("Audio: {} files", self.audio_count);
        println!("Documents: {} files", self.document_count);
        println!("Other: {} files", self.other_count);
        println!("NSFW content: {} files", self.nsfw_count);

        println!("\n=== Format Distribution ===");
        let mut formats: Vec<_> = self.files_by_type.iter().collect();
        formats.sort_by(|a, b| b.1.cmp(a.1));
        for (format, count) in formats {
            println!("{}: {} files", format, count);
        }

        if let Some(file) = &self.largest_file {
            println!("\n=== Largest File ===");
            println!("Name: {}", file.name);
            println!("URL: {}", file.url);
            println!("Size: {} bytes ({} MB)", file.size, file.size / 1_048_576);
            match &file.file_type {
                FileType::Image(format) => println!("Type: Image ({})", format),
                FileType::Video(format) => println!("Type: Video ({})", format),
                FileType::Audio(format) => println!("Type: Audio ({})", format),
                FileType::Document(format) => println!("Type: Document ({})", format),
                FileType::Other(ext) => println!("Type: Other ({})", ext),
                FileType::Directory => println!("Type: Directory"),
            }
        }

        if let Some((dir, size)) = &self.largest_directory {
            println!("\n=== Largest Directory ===");
            println!("Path: {}", dir);
            println!("Size: {} bytes ({} MB)", size, size / 1_048_576);
        }
    }

    fn save_to_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
        let mut file = fs::File::create(path)?;

        writeln!(file, "# Directory Scan Report")?;
        writeln!(file, "\n## General Statistics")?;
        writeln!(file, "- Total files: {}", self.total_files)?;
        writeln!(file, "- Total directories: {}", self.total_directories)?;
        writeln!(
            file,
            "- Total size: {} bytes ({} MB)",
            self.total_size,
            self.total_size / 1_048_576
        )?;

        writeln!(file, "\n## File Type Breakdown")?;
        writeln!(file, "- Images: {} files", self.image_count)?;
        writeln!(file, "- Videos: {} files", self.video_count)?;
        writeln!(file, "- Audio: {} files", self.audio_count)?;
        writeln!(file, "- Documents: {} files", self.document_count)?;
        writeln!(file, "- Other: {} files", self.other_count)?;
        writeln!(file, "- NSFW content: {} files", self.nsfw_count)?;

        writeln!(file, "\n## Format Distribution")?;
        let mut formats: Vec<_> = self.files_by_type.iter().collect();
        formats.sort_by(|a, b| b.1.cmp(a.1));
        for (format, count) in formats {
            writeln!(file, "- {}: {} files", format, count)?;
        }

        if let Some(file_info) = &self.largest_file {
            writeln!(file, "\n## Largest File")?;
            writeln!(file, "- Name: {}", file_info.name)?;
            writeln!(file, "- URL: {}", file_info.url)?;
            writeln!(
                file,
                "- Size: {} bytes ({} MB)",
                file_info.size,
                file_info.size / 1_048_576
            )?;
            match &file_info.file_type {
                FileType::Image(format) => writeln!(file, "- Type: Image ({})", format)?,
                FileType::Video(format) => writeln!(file, "- Type: Video ({})", format)?,
                FileType::Audio(format) => writeln!(file, "- Type: Audio ({})", format)?,
                FileType::Document(format) => writeln!(file, "- Type: Document ({})", format)?,
                FileType::Other(ext) => writeln!(file, "- Type: Other ({})", ext)?,
                FileType::Directory => writeln!(file, "- Type: Directory")?,
            }
        }

        if let Some((dir, size)) = &self.largest_directory {
            writeln!(file, "\n## Largest Directory")?;
            writeln!(file, "- Path: {}", dir)?;
            writeln!(file, "- Size: {} bytes ({} MB)", size, size / 1_048_576)?;
        }

        Ok(())
    }
}

fn get_file_type(filename: &str) -> FileType {
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        // Images
        "jpg" | "jpeg" => FileType::Image("JPEG".to_string()),
        "png" => FileType::Image("PNG".to_string()),
        "gif" => FileType::Image("GIF".to_string()),
        "webp" => FileType::Image("WebP".to_string()),
        "bmp" => FileType::Image("BMP".to_string()),
        "svg" => FileType::Image("SVG".to_string()),
        "tiff" | "tif" => FileType::Image("TIFF".to_string()),

        // Videos
        "mp4" => FileType::Video("MP4".to_string()),
        "webm" => FileType::Video("WebM".to_string()),
        "avi" => FileType::Video("AVI".to_string()),
        "mov" => FileType::Video("QuickTime".to_string()),
        "mkv" => FileType::Video("Matroska".to_string()),
        "flv" => FileType::Video("Flash".to_string()),
        "wmv" => FileType::Video("Windows Media".to_string()),

        // Audio
        "mp3" => FileType::Audio("MP3".to_string()),
        "wav" => FileType::Audio("WAV".to_string()),
        "ogg" => FileType::Audio("OGG".to_string()),
        "flac" => FileType::Audio("FLAC".to_string()),
        "aac" => FileType::Audio("AAC".to_string()),

        // Documents
        "pdf" => FileType::Document("PDF".to_string()),
        "doc" | "docx" => FileType::Document("Word".to_string()),
        "xls" | "xlsx" => FileType::Document("Excel".to_string()),
        "ppt" | "pptx" => FileType::Document("PowerPoint".to_string()),
        "txt" => FileType::Document("Text".to_string()),
        "md" => FileType::Document("Markdown".to_string()),

        // Other
        "" => FileType::Directory,
        _ => FileType::Other(extension),
    }
}

fn parse_size(size_str: &str) -> u64 {
    // Parse size strings like "1.5K", "2.3M", "4G", etc.
    let size_str = size_str.trim();

    if size_str.is_empty() {
        return 0;
    }

    // For directory entries in some servers, there might not be a size
    if size_str == "-" || size_str == "Directory" {
        return 0;
    }

    // Try to parse as a plain number first
    if let Ok(size) = size_str.parse::<u64>() {
        return size;
    }

    // Handle suffixes
    let chars = size_str.chars();
    let mut number_part = String::new();
    let mut suffix_part = String::new();

    for c in chars {
        if c.is_ascii_digit() || c == '.' {
            number_part.push(c);
        } else {
            suffix_part.push(c);
        }
    }

    let number = number_part.parse::<f64>().unwrap_or(0.0);
    match suffix_part.to_uppercase().as_str() {
        "K" => (number * 1_024.0) as u64,
        "M" => (number * 1_048_576.0) as u64,
        "G" => (number * 1_073_741_824.0) as u64,
        "T" => (number * 1_099_511_627_776.0) as u64,
        _ => number as u64,
    }
}

fn scan_directory(
    url: &str,
    depth: u32,
    max_depth: u32,
    client: &Client,
    nsfw_detector: &NSFWDetector,
) -> Result<(DirectoryReport, Vec<FileInfo>), Box<dyn Error>> {
    if depth > max_depth {
        return Ok((DirectoryReport::new(), vec![]));
    }

    println!("Scanning directory: {} (depth: {})", url, depth);

    let mut report = DirectoryReport::new();
    let mut all_files = Vec::new();

    let response = client.get(url).send()?;
    if !response.status().is_success() {
        return Err(format!("Failed to fetch {}: {}", url, response.status()).into());
    }

    let body = response.text()?;
    let document = Html::parse_document(&body);

    // This selector pattern might need adjustment based on the specific server's HTML structure
    let selector = Selector::parse("a").unwrap();

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(href) => href,
            None => continue,
        };

        // Skip parent directory links
        if href == "../" || href == ".." || href == "/" || href.starts_with("?") {
            continue;
        }

        let file_url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{}{}", url.trim_end_matches('/'), href)
        };

        let name = href.trim_end_matches('/');
        let is_directory = href.ends_with('/');

        // Try to find file size in the HTML
        // This is very server-specific, might need adjustment
        let size_text = element
            .parent()
            .and_then(|parent| {
                let parent_element = scraper::ElementRef::wrap(parent)?;
                parent_element
                    .select(&Selector::parse("td:nth-child(2)").unwrap())
                    .next()
            })
            .map(|size_element| size_element.text().collect::<String>())
            .unwrap_or_default();

        let size = parse_size(&size_text);

        let file_type = if is_directory {
            FileType::Directory
        } else {
            get_file_type(name)
        };

        // Check if image is NSFW
        let mut is_nsfw = None;
        if matches!(file_type, FileType::Image(_)) {
            // Only check images for NSFW content
            match nsfw_detector.is_nsfw(&file_url, client) {
                Ok(nsfw) => {
                    is_nsfw = Some(nsfw);
                    if nsfw {
                        report.nsfw_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to check NSFW for {}: {}", file_url, e);
                }
            }
        }

        let file_info = FileInfo {
            name: name.to_string(),
            url: file_url.clone(),
            size,
            file_type: file_type.clone(),
            is_nsfw,
        };

        // Update report statistics
        if is_directory {
            report.total_directories += 1;

            // Recursively scan subdirectory
            if depth < max_depth {
                match scan_directory(&file_url, depth + 1, max_depth, client, nsfw_detector) {
                    Ok((subdir_report, subdir_files)) => {
                        let subdir_size = subdir_report.total_size;

                        // Update largest directory if this is larger
                        if report.largest_directory.is_none()
                            || report.largest_directory.as_ref().unwrap().1 < subdir_size
                        {
                            report.largest_directory = Some((file_url, subdir_size));
                        }

                        // Update overall statistics
                        report.total_files += subdir_report.total_files;
                        report.total_directories += subdir_report.total_directories;
                        report.total_size += subdir_report.total_size;
                        report.image_count += subdir_report.image_count;
                        report.video_count += subdir_report.video_count;
                        report.audio_count += subdir_report.audio_count;
                        report.document_count += subdir_report.document_count;
                        report.other_count += subdir_report.other_count;
                        report.nsfw_count += subdir_report.nsfw_count;

                        // Update file type counts
                        for (format, count) in subdir_report.files_by_type {
                            *report.files_by_type.entry(format).or_insert(0) += count;
                        }

                        // Check if largest file is in this subdirectory
                        if let Some(subdir_largest_file) = subdir_report.largest_file {
                            if report.largest_file.is_none()
                                || report.largest_file.as_ref().unwrap().size
                                    < subdir_largest_file.size
                            {
                                report.largest_file = Some(subdir_largest_file.clone());
                            }
                        }

                        // Add all subdirectory files to our collection
                        all_files.extend(subdir_files);
                    }
                    Err(e) => eprintln!("Error scanning subdirectory {}: {}", file_url, e),
                }
            }
        } else {
            report.total_files += 1;
            report.total_size += size;

            // Update largest file if this is larger
            if report.largest_file.is_none() || report.largest_file.as_ref().unwrap().size < size {
                report.largest_file = Some(file_info.clone());
            }

            // Update file type statistics
            match &file_type {
                FileType::Image(format) => {
                    report.image_count += 1;
                    *report.files_by_type.entry(format.clone()).or_insert(0) += 1;
                }
                FileType::Video(format) => {
                    report.video_count += 1;
                    *report.files_by_type.entry(format.clone()).or_insert(0) += 1;
                }
                FileType::Audio(format) => {
                    report.audio_count += 1;
                    *report.files_by_type.entry(format.clone()).or_insert(0) += 1;
                }
                FileType::Document(format) => {
                    report.document_count += 1;
                    *report.files_by_type.entry(format.clone()).or_insert(0) += 1;
                }
                FileType::Other(ext) => {
                    report.other_count += 1;
                    *report.files_by_type.entry(ext.clone()).or_insert(0) += 1;
                }
                FileType::Directory => {}
            }
        }

        all_files.push(file_info);
    }

    Ok((report, all_files))
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("Open Directory Scanner")
        .version("1.0")
        .author("TigerdieKatze")
        .about("Scans open web directories and creates reports")
        .arg(
            Arg::new("url")
                .help("The URL of the directory to scan")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("depth")
                .short('d')
                .long("depth")
                .help("Maximum directory depth to scan")
                .value_name("DEPTH")
                .default_value("3"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file path for the report")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("timeout")
                .short('t')
                .long("timeout")
                .help("Request timeout in seconds")
                .value_name("SECONDS")
                .default_value("30"),
        )
        .get_matches();

    let url = matches.get_one::<String>("url").unwrap();
    let max_depth = matches
        .get_one::<String>("depth")
        .unwrap()
        .parse::<u32>()
        .unwrap_or(3);
    let timeout = matches
        .get_one::<String>("timeout")
        .unwrap()
        .parse::<u64>()
        .unwrap_or(30);

    let client = Client::builder()
        .timeout(Duration::from_secs(timeout))
        .build()?;

    let nsfw_detector = NSFWDetector::new()?;

    println!("Starting scan of {} with max depth {}", url, max_depth);
    let (report, _) = scan_directory(url, 0, max_depth, &client, &nsfw_detector)?;

    println!("\n=== Scan Complete ===");
    report.print();

    if let Some(output_path) = matches.get_one::<String>("output") {
        report.save_to_file(output_path)?;
        println!("\nReport saved to: {}", output_path);
    }

    Ok(())
}
