use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Directory,
    Image(String),
    Video(String),
    Audio(String),
    Document(String),
    Other(String),
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub file_type: FileType,
    pub is_nsfw: Option<bool>,
}

pub struct DirectoryReport {
    pub total_files: usize,
    pub total_directories: usize,
    pub total_size: u64,
    pub files_by_type: HashMap<String, usize>,
    pub image_count: usize,
    pub video_count: usize,
    pub audio_count: usize,
    pub document_count: usize,
    pub other_count: usize,
    pub nsfw_count: usize,
    pub nsfw_files: Vec<String>,
    pub largest_file: Option<FileInfo>,
    pub largest_directory: Option<(String, u64)>,
}

impl DirectoryReport {
    pub fn new() -> Self {
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
            nsfw_files: Vec::new(),
            largest_file: None,
            largest_directory: None,
        }
    }

    pub fn print(&self) {
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

        if self.nsfw_count > 0 {
            println!("\n=== NSFW Files ===");
            for file in &self.nsfw_files {
                println!("{}", file);
            }
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn Error>> {
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
