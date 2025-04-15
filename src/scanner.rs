use crate::nsfwcheck::NSFWDetector;
use crate::report::{DirectoryReport, FileInfo, FileType};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use std::error::Error;
use std::path::Path;

fn get_file_type(filename: &str) -> FileType {
    let extension = Path::new(filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();

    match extension.as_str() {
        "jpg" | "jpeg" => FileType::Image("JPEG".to_string()),
        "png" => FileType::Image("PNG".to_string()),
        "gif" => FileType::Image("GIF".to_string()),
        "webp" => FileType::Image("WebP".to_string()),
        "bmp" => FileType::Image("BMP".to_string()),
        "svg" => FileType::Image("SVG".to_string()),
        "tiff" | "tif" => FileType::Image("TIFF".to_string()),
        "mp4" => FileType::Video("MP4".to_string()),
        "webm" => FileType::Video("WebM".to_string()),
        "avi" => FileType::Video("AVI".to_string()),
        "mov" => FileType::Video("QuickTime".to_string()),
        "mkv" => FileType::Video("Matroska".to_string()),
        "flv" => FileType::Video("Flash".to_string()),
        "wmv" => FileType::Video("Windows Media".to_string()),
        "mp3" => FileType::Audio("MP3".to_string()),
        "wav" => FileType::Audio("WAV".to_string()),
        "ogg" => FileType::Audio("OGG".to_string()),
        "flac" => FileType::Audio("FLAC".to_string()),
        "aac" => FileType::Audio("AAC".to_string()),
        "pdf" => FileType::Document("PDF".to_string()),
        "doc" | "docx" => FileType::Document("Word".to_string()),
        "xls" | "xlsx" => FileType::Document("Excel".to_string()),
        "ppt" | "pptx" => FileType::Document("PowerPoint".to_string()),
        "txt" => FileType::Document("Text".to_string()),
        "md" => FileType::Document("Markdown".to_string()),
        "" => FileType::Directory,
        _ => FileType::Other(extension),
    }
}

fn parse_size(size_str: &str) -> u64 {
    let size_str = size_str.trim();
    if size_str.is_empty() || size_str == "-" || size_str == "Directory" {
        return 0;
    }
    if let Ok(size) = size_str.parse::<u64>() {
        return size;
    }
    let mut number_part = String::new();
    let mut suffix_part = String::new();
    for c in size_str.chars() {
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

pub fn scan_directory(
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
    let selector = Selector::parse("a").unwrap();

    for element in document.select(&selector) {
        let href = match element.value().attr("href") {
            Some(href) => href,
            None => continue,
        };

        if href == "../" || href == ".." || href == "/" || href.starts_with("?") {
            continue;
        }

        let clean_href = href.trim_end_matches('/');
        let file_url = if href.starts_with("http") {
            href.to_string()
        } else {
            let base_url = if url.ends_with('/') {
                url.to_string()
            } else {
                format!("{}/", url)
            };
            let path_part = href.trim_start_matches('/');
            format!("{}{}", base_url, path_part)
        };

        let name = clean_href;
        let is_directory = href.ends_with('/');

        // Size extraction is server-specific; fallback to 0
        let size = 0;

        let file_type = if is_directory {
            FileType::Directory
        } else {
            get_file_type(name)
        };

        let mut is_nsfw = None;
        if matches!(file_type, FileType::Image(_)) {
            let image_url = file_url.trim_end_matches('/').to_string();
            match nsfw_detector.is_nsfw(&image_url, client) {
                Ok(nsfw) => {
                    is_nsfw = Some(nsfw);
                    if nsfw {
                        report.nsfw_count += 1;
                        report.nsfw_files.push(image_url.clone());
                    }
                }
                Err(e) => {
                    eprintln!("Failed to check NSFW for {}: {}", image_url, e);
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

        if is_directory {
            report.total_directories += 1;
            if depth < max_depth {
                match scan_directory(&file_url, depth + 1, max_depth, client, nsfw_detector) {
                    Ok((subdir_report, subdir_files)) => {
                        let subdir_size = subdir_report.total_size;
                        if report.largest_directory.is_none()
                            || report.largest_directory.as_ref().unwrap().1 < subdir_size
                        {
                            report.largest_directory = Some((file_url.clone(), subdir_size));
                        }
                        report.total_files += subdir_report.total_files;
                        report.total_directories += subdir_report.total_directories;
                        report.total_size += subdir_report.total_size;
                        report.image_count += subdir_report.image_count;
                        report.video_count += subdir_report.video_count;
                        report.audio_count += subdir_report.audio_count;
                        report.document_count += subdir_report.document_count;
                        report.other_count += subdir_report.other_count;
                        report.nsfw_count += subdir_report.nsfw_count;
                        for (format, count) in subdir_report.files_by_type {
                            *report.files_by_type.entry(format).or_insert(0) += count;
                        }
                        if let Some(subdir_largest_file) = subdir_report.largest_file {
                            if report.largest_file.is_none()
                                || report.largest_file.as_ref().unwrap().size
                                    < subdir_largest_file.size
                            {
                                report.largest_file = Some(subdir_largest_file.clone());
                            }
                        }
                        all_files.extend(subdir_files);
                    }
                    Err(e) => eprintln!("Error scanning subdirectory {}: {}", file_url, e),
                }
            }
        } else {
            report.total_files += 1;
            report.total_size += size;
            if report.largest_file.is_none() || report.largest_file.as_ref().unwrap().size < size {
                report.largest_file = Some(file_info.clone());
            }
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
