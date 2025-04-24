use image::ImageReader;
use nsfw::{create_model, examine};
use reqwest::blocking::Client;
use serde_json::Value as JsonValue;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use toml::{Table, Value};

// Constants
const CONFIG_PATH: &str = "resources/config.toml";
const GITHUB_API_RELEASES: &str = "https://api.github.com/repos/Fyko/nsfw/releases/latest";
const DEFAULT_CONFIG: &str = r#"# Open Directory Scanner Configuration

# Model information
[model]
version = "0.0.0"  # Will be updated with the actual version from GitHub
last_updated = ""  # Will be populated when model is downloaded
url = "https://github.com/Fyko/nsfw/releases/latest/download/model.onnx"
path = "resources/model.onnx"

# NSFW detection thresholds
[thresholds]
porn = 0.5    # Porn classification threshold
hentai = 0.6  # Hentai classification threshold
sexy = 0.8    # Sexy classification threshold

# Scanner defaults
[scanner]
default_depth = 3
default_timeout = 30

# Report settings
[report]
include_nsfw_urls = true  # Whether to include URLs of NSFW content in reports"#;

pub struct NSFWDetector {
    model: nsfw::Model,
    config: Table,
}

impl NSFWDetector {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut config = Self::load_config()?;
        let model_data = Self::ensure_model(&mut config)?;
        let model = create_model(&model_data[..])?;

        let detector = NSFWDetector { model, config };
        detector.save_updated_config()?;

        Ok(detector)
    }

    fn load_config() -> Result<Table, Box<dyn Error>> {
        let config_path = Path::new(CONFIG_PATH);
        if !config_path.exists() {
            println!("Config file not found, creating default config...");
            if let Some(parent) = config_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(config_path, DEFAULT_CONFIG)?;
            println!("Default config file created at {}", CONFIG_PATH);
        }

        let config_str = fs::read_to_string(config_path)?;
        let config: Table = toml::from_str(&config_str)?;
        Ok(config)
    }

    fn get_latest_release_info() -> Result<(String, String), Box<dyn Error>> {
        println!("Checking for latest model version on GitHub...");

        let client = reqwest::blocking::Client::builder()
            .user_agent("OpenDirectoryScanner/0.1.0")
            .build()?;

        let response = client.get(GITHUB_API_RELEASES).send()?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to check GitHub releases: HTTP {}",
                response.status()
            )
            .into());
        }

        let release_info: JsonValue = response.json()?;

        // Extract the tag name (version) and download URL
        let version = release_info["tag_name"]
            .as_str()
            .ok_or("Invalid release info: missing tag_name")?
            .trim_start_matches('v')
            .to_string();

        // Find the model.onnx asset
        let assets = release_info["assets"]
            .as_array()
            .ok_or("Invalid release info: missing assets")?;

        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name == "model.onnx" {
                let download_url = asset["browser_download_url"]
                    .as_str()
                    .ok_or("Invalid asset info: missing download URL")?
                    .to_string();

                return Ok((version, download_url));
            }
        }

        // Fallback to the default URL if asset not found
        Ok((
            version,
            "https://github.com/Fyko/nsfw/releases/latest/download/model.onnx".to_string(),
        ))
    }

    fn save_updated_config(&self) -> Result<(), Box<dyn Error>> {
        let config_str = toml::to_string(&self.config)?;
        fs::write(CONFIG_PATH, config_str)?;
        Ok(())
    }

    fn ensure_model(config: &mut Table) -> Result<Vec<u8>, Box<dyn Error>> {
        // Get model config section or create it
        let model_config = match config.get_mut("model") {
            Some(Value::Table(table)) => table,
            _ => {
                let mut table = Table::new();
                table.insert("version".to_string(), Value::String("0.0.0".to_string()));
                table.insert("last_updated".to_string(), Value::String("".to_string()));
                table.insert(
                    "url".to_string(),
                    Value::String(
                        "https://github.com/Fyko/nsfw/releases/latest/download/model.onnx"
                            .to_string(),
                    ),
                );
                table.insert(
                    "path".to_string(),
                    Value::String("resources/model.onnx".to_string()),
                );
                config.insert("model".to_string(), Value::Table(table));
                config.get_mut("model").unwrap().as_table_mut().unwrap()
            }
        };

        let model_path = model_config
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("resources/model.onnx")
            .to_string();
        let current_version = model_config
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0");

        let path = Path::new(&model_path);

        // Check GitHub for latest release
        let (latest_version, download_url) = Self::get_latest_release_info()?;

        // Check if model exists and if version is current
        let needs_download = if !path.exists() {
            println!(
                "NSFW model not found, downloading version {}...",
                latest_version
            );
            true
        } else if latest_version != current_version {
            println!(
                "NSFW model update available: {} -> {}",
                current_version, latest_version
            );
            true
        } else {
            println!("NSFW model is up to date (version {})", current_version);
            false
        };

        if needs_download {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            println!("Downloading model from {}", download_url);
            let mut resp = reqwest::blocking::get(&download_url)?;
            if !resp.status().is_success() {
                return Err(format!("Failed to download model: HTTP {}", resp.status()).into());
            }

            let mut out = fs::File::create(path)?;
            let mut buf = Vec::new();
            resp.read_to_end(&mut buf)?;
            out.write_all(&buf)?;

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs()
                .to_string();

            model_config.insert("version".to_string(), Value::String(latest_version.clone()));
            model_config.insert("last_updated".to_string(), Value::String(timestamp));
            model_config.insert("url".to_string(), Value::String(download_url));

            println!(
                "Model version {} downloaded to {}",
                latest_version, model_path
            );
            Ok(buf)
        } else {
            let buf = fs::read(path)?;
            Ok(buf)
        }
    }

    pub fn is_nsfw(&self, image_url: &str, client: &Client) -> Result<bool, Box<dyn Error>> {
        let response = client.get(image_url).send()?;
        if !response.status().is_success() {
            return Err(format!("Failed to download image: {}", response.status()).into());
        }

        let image_data = response.bytes()?;
        let img = ImageReader::new(Cursor::new(image_data))
            .with_guessed_format()?
            .decode()?;

        let rgba_img = img.to_rgba8();
        let result = examine(&self.model, &rgba_img)?;

        // Get thresholds from config
        let thresholds = match self.config.get("thresholds").and_then(|v| v.as_table()) {
            Some(table) => table,
            None => {
                // If thresholds not found in config, use these default values
                static DEFAULT_PORN: f64 = 0.5;
                static DEFAULT_HENTAI: f64 = 0.6;
                static DEFAULT_SEXY: f64 = 0.8;
                
                return Ok(result
                    .iter()
                    .any(|classification| match classification.metric {
                        nsfw::model::Metric::Hentai => classification.score > DEFAULT_HENTAI as f32,
                        nsfw::model::Metric::Porn => classification.score > DEFAULT_PORN as f32,
                        nsfw::model::Metric::Sexy => classification.score > DEFAULT_SEXY as f32,
                        _ => false,
                    }));
            }
        };

        let porn_threshold = thresholds
            .get("porn")
            .and_then(|v| v.as_float())
            .unwrap_or(0.5) as f32;
        let hentai_threshold = thresholds
            .get("hentai")
            .and_then(|v| v.as_float())
            .unwrap_or(0.6) as f32;
        let sexy_threshold = thresholds
            .get("sexy")
            .and_then(|v| v.as_float())
            .unwrap_or(0.8) as f32;

        println!("NSFW results for {}: {:?}", image_url, result);

        let check_results = result
            .iter()
            .any(|classification| match classification.metric {
                nsfw::model::Metric::Hentai => classification.score > hentai_threshold,
                nsfw::model::Metric::Porn => classification.score > porn_threshold,
                nsfw::model::Metric::Sexy => classification.score > sexy_threshold,
                _ => false,
            });

        Ok(check_results)
    }
}
