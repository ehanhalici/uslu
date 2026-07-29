// src/image.rs
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use image::{DynamicImage, GenericImageView, ImageFormat, RgbaImage};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use uuid::Uuid;

const DEFAULT_ZOOM_LEVEL: f32 = 1.0;
const DEFAULT_OFFSET_X: f32 = 0.0;
const DEFAULT_OFFSET_Y: f32 = 0.0;

pub const CROPPER_CANVAS_SIZE: f32 = 240.0;
pub const BASE_IMAGE_SIZE: f32 = 200.0;
const TARGET_CROPPED_DIMENSION: u32 = 128;

#[cfg(target_os = "linux")]
const FILE_DIALOG_BINARY: &str = "zenity";
#[cfg(target_os = "linux")]
const FILE_DIALOG_TITLE_ARG: &str = "--title=Resim Seç";
#[cfg(target_os = "linux")]
const FILE_DIALOG_FILTER_ARG: &str =
    "--file-filter=Resimler (png, jpg, jpeg, webp) | *.png *.jpg *.jpeg *.webp";

const MARKDOWN_HEADER_TITLE: &str = "# Images Storage\n";
const MARKDOWN_SECTION_PREFIX: &str = "## ";
const MARKDOWN_DATA_PREFIX: &str = "- data: \"";
const QUOTE_CHAR: char = '"';

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
        let min_zoom = CROPPER_CANVAS_SIZE / BASE_IMAGE_SIZE;
        let initial_zoom = DEFAULT_ZOOM_LEVEL.max(min_zoom);

        let mut state = Self {
            original_image: img,
            image_handle,
            zoom: initial_zoom,
            offset_x: DEFAULT_OFFSET_X,
            offset_y: DEFAULT_OFFSET_Y,
        };
        state.clamp_pan();
        state
    }    

    pub fn clamp_pan(&mut self) {
        let (img_w, img_h) = calculate_scaled_image_dimensions(&self.original_image, self.zoom);

        let max_offset_x = ((img_w - CROPPER_CANVAS_SIZE) / 2.0).max(0.0);
        let max_offset_y = ((img_h - CROPPER_CANVAS_SIZE) / 2.0).max(0.0);

        self.offset_x = self.offset_x.clamp(-max_offset_x, max_offset_x);
        self.offset_y = self.offset_y.clamp(-max_offset_y, max_offset_y);
    }

    pub fn crop_to_base64(&self) -> Result<String, String> {
        let (orig_w, orig_h) = (
            self.original_image.width() as f32,
            self.original_image.height() as f32,
        );
        let (current_w, current_h) =
            calculate_scaled_image_dimensions(&self.original_image, self.zoom);

        let scale_x = orig_w / current_w;
        let scale_y = orig_h / current_h;

        let img_center_x = (CROPPER_CANVAS_SIZE / 2.0) + self.offset_x;
        let img_center_y = (CROPPER_CANVAS_SIZE / 2.0) + self.offset_y;

        let img_left = img_center_x - (current_w / 2.0);
        let img_top = img_center_y - (current_h / 2.0);

        let final_canvas = generate_cropped_rgba_canvas(
            &self.original_image,
            current_w,
            current_h,
            scale_x,
            scale_y,
            img_left,
            img_top,
        );

        encode_rgba_image_to_base64(&final_canvas)
    }
}

pub struct ImageManager;

impl ImageManager {
    pub async fn pick_image_file() -> Option<(DynamicImage, Vec<u8>)> {
        tokio::task::spawn_blocking(execute_file_picker_dialog)
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

        write_images_map_to_file(images_file_path, &images_map)
    }

    pub fn load_all_images(images_file_path: &str) -> Result<HashMap<String, String>, String> {
        if !Path::new(images_file_path).exists() {
            return Ok(HashMap::new());
        }

        let content = read_file_to_string(images_file_path)?;
        parse_images_map_from_content(&content)
    }

    pub fn base64_to_handle(base64_str: &str) -> Option<iced::widget::image::Handle> {
        let bytes = BASE64.decode(base64_str).ok()?;
        Some(iced::widget::image::Handle::from_bytes(bytes))
    }
}

fn calculate_base_image_dimensions(img: &DynamicImage) -> (f32, f32) {
    let (orig_w, orig_h) = (img.width() as f32, img.height() as f32);
    let aspect_ratio = orig_w / orig_h;

    if aspect_ratio >= 1.0 {
        (BASE_IMAGE_SIZE * aspect_ratio, BASE_IMAGE_SIZE)
    } else {
        (BASE_IMAGE_SIZE, BASE_IMAGE_SIZE / aspect_ratio)
    }
}

fn calculate_scaled_image_dimensions(img: &DynamicImage, zoom: f32) -> (f32, f32) {
    let (base_w, base_h) = calculate_base_image_dimensions(img);
    (base_w * zoom, base_h * zoom)
}

fn generate_cropped_rgba_canvas(
    img: &DynamicImage,
    current_w: f32,
    current_h: f32,
    scale_x: f32,
    scale_y: f32,
    img_left: f32,
    img_top: f32,
) -> RgbaImage {
    let mut final_canvas = RgbaImage::new(TARGET_CROPPED_DIMENSION, TARGET_CROPPED_DIMENSION);
    let (orig_w, orig_h) = (img.width() as f32, img.height() as f32);

    for target_y in 0..TARGET_CROPPED_DIMENSION {
        for target_x in 0..TARGET_CROPPED_DIMENSION {
            let src_canvas_x =
                (target_x as f32 / TARGET_CROPPED_DIMENSION as f32) * CROPPER_CANVAS_SIZE;
            let src_canvas_y =
                (target_y as f32 / TARGET_CROPPED_DIMENSION as f32) * CROPPER_CANVAS_SIZE;

            let rel_x = src_canvas_x - img_left;
            let rel_y = src_canvas_y - img_top;

            if rel_x >= 0.0 && rel_x < current_w && rel_y >= 0.0 && rel_y < current_h {
                let orig_x = (rel_x * scale_x) as u32;
                let orig_y = (rel_y * scale_y) as u32;

                if orig_x < orig_w as u32 && orig_y < orig_h as u32 {
                    let pixel = img.get_pixel(orig_x, orig_y);
                    final_canvas.put_pixel(target_x, target_y, pixel);
                }
            }
        }
    }

    final_canvas
}

fn encode_rgba_image_to_base64(canvas: &RgbaImage) -> Result<String, String> {
    let mut png_bytes = Vec::new();
    let mut cursor = Cursor::new(&mut png_bytes);

    DynamicImage::ImageRgba8(canvas.clone())
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|err| format!("PNG Encode Hatası: {}", err))?;

    Ok(BASE64.encode(&png_bytes))
}

#[cfg(target_os = "linux")]
fn execute_file_picker_dialog() -> Option<(DynamicImage, Vec<u8>)> {
    let output = std::process::Command::new(FILE_DIALOG_BINARY)
        .arg("--file-selection")
        .arg(FILE_DIALOG_TITLE_ARG)
        .arg(FILE_DIALOG_FILTER_ARG)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path_str.is_empty() {
        return None;
    }

    load_image_from_path(&path_str)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn execute_file_picker_dialog() -> Option<(DynamicImage, Vec<u8>)> {
    let file_path = rfd::FileDialog::new()
        .add_filter("Resimler", &["png", "jpg", "jpeg", "webp"])
        .pick_file()?;

    let path_str = file_path.to_str()?;
    load_image_from_path(path_str)
}

fn load_image_from_path(path_str: &str) -> Option<(DynamicImage, Vec<u8>)> {
    let bytes = std::fs::read(path_str).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    Some((img, bytes))
}

fn read_file_to_string(path_str: &str) -> Result<String, String> {
    let mut file = File::open(path_str).map_err(|e| e.to_string())?;
    let mut content = String::new();
    file.read_to_string(&mut content)
        .map_err(|e| e.to_string())?;
    Ok(content)
}

pub fn write_images_map_to_file(
    file_path: &str,
    images_map: &HashMap<String, String>,
) -> Result<(), String> {
    let tmp_path = format!("{}.tmp", file_path);
    {
        let mut file = File::create(&tmp_path).map_err(|e| e.to_string())?;
        writeln!(file, "{}", MARKDOWN_HEADER_TITLE).map_err(|e| e.to_string())?;

        let mut sorted_keys: Vec<_> = images_map.keys().collect();
        sorted_keys.sort();

        for id in sorted_keys {
            if let Some(data) = images_map.get(id) {
                writeln!(file, "{}{}", MARKDOWN_SECTION_PREFIX, id).map_err(|e| e.to_string())?;
                writeln!(file, "{}{}\"\n", MARKDOWN_DATA_PREFIX, data).map_err(|e| e.to_string())?;
            }
        }
        file.flush().map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    std::fs::rename(tmp_path, file_path).map_err(|e| e.to_string())?;
    Ok(())
}

fn parse_images_map_from_content(content: &str) -> Result<HashMap<String, String>, String> {
    let mut images_map = HashMap::new();
    let mut current_id: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if let Some(id) = trimmed.strip_prefix(MARKDOWN_SECTION_PREFIX) {
            current_id = Some(id.trim().to_string());
        } else if let Some(data) = trimmed.strip_prefix(MARKDOWN_DATA_PREFIX) {
            if let Some(ref id) = current_id {
                let clean_data = data.trim_end_matches(QUOTE_CHAR);
                images_map.insert(id.clone(), clean_data.to_string());
            }
        }
    }

    Ok(images_map)
}
