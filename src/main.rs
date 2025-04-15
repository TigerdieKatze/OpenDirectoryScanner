mod nsfwcheck;
mod report;
mod scanner;

use clap::{Arg, Command};
use nsfwcheck::NSFWDetector;
use reqwest::blocking::Client;
use std::error::Error;
use std::time::Duration;

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
    let (report, _) = scanner::scan_directory(url, 0, max_depth, &client, &nsfw_detector)?;

    println!("\n=== Scan Complete ===");
    report.print();

    if let Some(output_path) = matches.get_one::<String>("output") {
        report.save_to_file(output_path)?;
        println!("\nReport saved to: {}", output_path);
    }

    Ok(())
}
