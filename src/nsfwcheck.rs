use image::ImageReader;
use nsfw::{create_model, examine};
use reqwest::blocking::Client;
use std::error::Error;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

const MODEL_URL: &str = "https://github.com/Fyko/nsfw/releases/latest/download/model.onnx";
const MODEL_PATH: &str = "resources/model.onnx";

pub struct NSFWDetector {
    model: nsfw::Model,
}

impl NSFWDetector {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let model_data = Self::ensure_model()?;
        let model = create_model(&model_data[..])?;
        Ok(NSFWDetector { model })
    }

    fn ensure_model() -> Result<Vec<u8>, Box<dyn Error>> {
        let path = Path::new(MODEL_PATH);
        if !path.exists() {
            println!("NSFW model not found, downloading...");
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut resp = reqwest::blocking::get(MODEL_URL)?;
            if !resp.status().is_success() {
                return Err(format!(
                    "Failed to download model: HTTP {}",
                    resp.status()
                )
                .into());
            }
            let mut out = fs::File::create(path)?;
            let mut buf = Vec::new();
            resp.read_to_end(&mut buf)?;
            out.write_all(&buf)?;
            println!("Model downloaded to {}", MODEL_PATH);
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

        println!("NSFW results for {}: {:?}", image_url, result);

        let check_results = result.iter().any(|classification| match classification.metric {
            nsfw::model::Metric::Hentai => classification.score > 0.6,
            nsfw::model::Metric::Porn => classification.score > 0.5,
            nsfw::model::Metric::Sexy => classification.score > 0.8,
            _ => false,
        });

        Ok(check_results)
    }
}
