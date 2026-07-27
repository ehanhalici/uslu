// src/image.rs
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::process::Command;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ImageCropperState {
    pub original_image: DynamicImage,
    pub image_handle: iced::widget::image::Handle,
    pub zoom: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl ImageCropperState {
    pub fn new(img: DynamicImage, raw_bytes: Vec<u8>) -> Self {
        let image_handle = iced::widget::image::Handle::from_bytes(raw_bytes);
        Self {
            original_image: img,
            image_handle,
            zoom: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }

    /// WhatsApp tarzı profil fotoğrafı kırpma davranışı: resmi ne kadar
    /// sürüklerseniz sürükleyin, kırpma kutusunun içinde asla boş alan
    /// kalmaz. Bu metodu offset_x / offset_y güncellendiği her yerde
    /// (pan delta uygulandıktan sonra VE zoom değiştikten sonra) çağırın.
    pub fn clamp_pan(&mut self) {
        let canvas_size = 240.0_f32;
        let (orig_w, orig_h) = (
            self.original_image.width() as f32,
            self.original_image.height() as f32,
        );
        let aspect_ratio = orig_w / orig_h;
        let base_size = 200.0_f32;
        let (base_img_w, base_img_h) = if aspect_ratio >= 1.0 {
            (base_size * aspect_ratio, base_size)
        } else {
            (base_size, base_size / aspect_ratio)
        };

        let img_w = base_img_w * self.zoom;
        let img_h = base_img_h * self.zoom;

        // Resmin kenarı en fazla kutunun kenarına kadar kayabilir; bunun
        // ötesi kutunun içinde boşluk (siyah alan) göstermek demektir.
        let max_offset_x = ((img_w - canvas_size) / 2.0).max(0.0);
        let max_offset_y = ((img_h - canvas_size) / 2.0).max(0.0);

        self.offset_x = self.offset_x.clamp(-max_offset_x, max_offset_x);
        self.offset_y = self.offset_y.clamp(-max_offset_y, max_offset_y);
    }

    // src/image.rs içindeki crop_to_base64 metodu:

    pub fn crop_to_base64(&self) -> Result<String, String> {
        let canvas_size = 240.0;
        let (orig_w, orig_h) = (
            self.original_image.width() as f32,
            self.original_image.height() as f32,
        );

        let aspect_ratio = orig_w / orig_h;
        let base_size = 200.0;
        let (base_img_w, base_img_h) = if aspect_ratio >= 1.0 {
            (base_size * aspect_ratio, base_size)
        } else {
            (base_size, base_size / aspect_ratio)
        };

        let current_w = base_img_w * self.zoom;
        let current_h = base_img_h * self.zoom;

        let scale_x = orig_w / current_w;
        let scale_y = orig_h / current_h;

        let img_center_x = (canvas_size / 2.0) + self.offset_x;
        let img_center_y = (canvas_size / 2.0) + self.offset_y;

        let img_left = img_center_x - (current_w / 2.0);
        let img_top = img_center_y - (current_h / 2.0);

        let mut final_canvas = image::RgbaImage::new(128, 128);

        // Siyah ekranın (240x240) içindeki alanı 128x128 piksele eşleyerek kırp
        for target_y in 0..128 {
            for target_x in 0..128 {
                let src_canvas_x = (target_x as f32 / 128.0) * canvas_size;
                let src_canvas_y = (target_y as f32 / 128.0) * canvas_size;

                let rel_x = src_canvas_x - img_left;
                let rel_y = src_canvas_y - img_top;

                if rel_x >= 0.0 && rel_x < current_w && rel_y >= 0.0 && rel_y < current_h {
                    let orig_x = (rel_x * scale_x) as u32;
                    let orig_y = (rel_y * scale_y) as u32;

                    if orig_x < orig_w as u32 && orig_y < orig_h as u32 {
                        let pixel = self.original_image.get_pixel(orig_x, orig_y);
                        final_canvas.put_pixel(target_x, target_y, pixel);
                    }
                }
            }
        }

        let mut png_bytes = Vec::new();
        let mut cursor = Cursor::new(&mut png_bytes);
        DynamicImage::ImageRgba8(final_canvas)
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(|e| format!("PNG Encode Hatası: {}", e))?;

        Ok(BASE64.encode(&png_bytes))
    }
}

pub struct ImageManager;

impl ImageManager {
    /// Zenity çağrısıyla dosya seçer
    pub async fn pick_image_file() -> Option<(DynamicImage, Vec<u8>)> {
        tokio::task::spawn_blocking(|| {
            let output = Command::new("zenity")
                .arg("--file-selection")
                .arg("--title=Resim Seç")
                .arg("--file-filter=Resimler (png, jpg, jpeg, webp) | *.png *.jpg *.jpeg *.webp")
                .output()
                .ok()?;

            if !output.status.success() {
                return None;
            }

            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if path_str.is_empty() {
                return None;
            }

            let bytes = std::fs::read(&path_str).ok()?;
            let img = image::load_from_memory(&bytes).ok()?;
            Some((img, bytes))
        })
        .await
        .ok()?
    }

    pub fn save_image_to_md(
        images_file_path: &str,
        image_id: Uuid,
        base64_data: &str,
    ) -> Result<(), String> {
        let mut images_map = Self::load_all_images(images_file_path).unwrap_or_default();
        images_map.insert(image_id.to_string(), base64_data.to_string());

        let mut file = File::create(images_file_path).map_err(|e| e.to_string())?;
        writeln!(file, "# Images Storage\n").map_err(|e| e.to_string())?;

        for (id, data) in images_map {
            writeln!(file, "## {}", id).map_err(|e| e.to_string())?;
            writeln!(file, "- data: \"{}\"\n", data).map_err(|e| e.to_string())?;
        }

        Ok(())
    }

    pub fn load_all_images(images_file_path: &str) -> Result<HashMap<String, String>, String> {
        if !std::path::Path::new(images_file_path).exists() {
            return Ok(HashMap::new());
        }

        let mut file = File::open(images_file_path).map_err(|e| e.to_string())?;
        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| e.to_string())?;

        let mut map = HashMap::new();
        let mut current_id: Option<String> = None;

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(id) = trimmed.strip_prefix("## ") {
                current_id = Some(id.trim().to_string());
            } else if let Some(data) = trimmed.strip_prefix("- data: \"") {
                if let Some(ref id) = current_id {
                    let clean_data = data.trim_end_matches('"');
                    map.insert(id.clone(), clean_data.to_string());
                }
            }
        }

        Ok(map)
    }

    pub fn base64_to_handle(base64_str: &str) -> Option<iced::widget::image::Handle> {
        let bytes = BASE64.decode(base64_str).ok()?;
        Some(iced::widget::image::Handle::from_bytes(bytes))
    }
}
