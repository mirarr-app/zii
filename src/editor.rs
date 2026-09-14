use std::path::{Path, PathBuf};
use anyhow::Context;
use image::{imageops, DynamicImage, GenericImageView, ImageFormat};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Adjustments {
    pub brightness: i32,  // -100 to +100
    pub contrast: f32,    // -100.0 to +100.0 (0.0 is neutral, positive increases, negative decreases)
    pub saturation: i32,  // -100 to +100
}

impl Default for Adjustments {
    fn default() -> Self {
        Self {
            brightness: 0,
            contrast: 0.0,
            saturation: 0,
        }
    }
}

pub fn read_exif_orientation(path: &Path) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).ok()?;
    let orientation = exif
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|f| f.value.get_uint(0))?;
    Some(orientation)
}

pub fn apply_orientation(img: DynamicImage, orientation: u32) -> DynamicImage {
    match orientation {
        2 => img.fliph(),
        3 => img.rotate180(),
        4 => img.flipv(),
        5 => img.rotate90().fliph(),
        6 => img.rotate90(),
        7 => img.rotate270().fliph(),
        8 => img.rotate270(),
        _ => img,
    }
}

pub struct ImageEditor {
    pub original_path: PathBuf,
    pub base_image: Option<DynamicImage>,
    pub adjustment_base: Option<DynamicImage>,
    pub undo_stack: Vec<DynamicImage>,
    pub redo_stack: Vec<DynamicImage>,
    pub current_image: Option<DynamicImage>,
    pub cache_dir: PathBuf,
    pub revision: usize,
}

impl ImageEditor {
    pub fn new() -> Self {
        let pid = std::process::id();
        let cache_dir = std::env::temp_dir().join(format!("zii_preview_cache_{}", pid));
        let _ = std::fs::create_dir_all(&cache_dir);
        Self {
            original_path: PathBuf::new(),
            base_image: None,
            adjustment_base: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_image: None,
            cache_dir,
            revision: 0,
        }
    }

    pub fn cleanup(&self) {
        let _ = std::fs::remove_dir_all(&self.cache_dir);
        let default_cache = std::env::temp_dir().join("zii_preview_cache");
        if default_cache.exists() && default_cache != self.cache_dir {
            let _ = std::fs::remove_dir_all(&default_cache);
        }
    }

    pub fn open(&mut self, path: &Path) -> anyhow::Result<PathBuf> {
        self.original_path = path.to_path_buf();
        let mut img = image::open(path)
            .with_context(|| format!("Failed to open image at {:?}", path))?;
        if let Some(orientation) = read_exif_orientation(path) {
            img = apply_orientation(img, orientation);
        }
        self.base_image = Some(img.clone());
        self.current_image = Some(img);
        self.adjustment_base = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.revision += 1;
        self.generate_preview()
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.current_image
            .as_ref()
            .map(|img| img.dimensions())
            .unwrap_or((0, 0))
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn commit_adjustments(&mut self) {
        if let Some(base) = self.adjustment_base.take() {
            self.push_undo(base);
        }
    }

    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> anyhow::Result<PathBuf> {
        self.commit_adjustments();
        let img = match &self.current_image {
            Some(i) => i.clone(),
            None => anyhow::bail!("No image currently loaded in editor"),
        };

        let (img_w, img_h) = img.dimensions();
        if width == 0 || height == 0 || x >= img_w || y >= img_h {
            anyhow::bail!("Invalid crop bounds");
        }

        let clamp_w = width.min(img_w.saturating_sub(x));
        let clamp_h = height.min(img_h.saturating_sub(y));

        let cropped = img.crop_imm(x, y, clamp_w, clamp_h);

        self.push_undo(img);
        self.current_image = Some(cropped);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn rotate(&mut self, degrees: i32) -> anyhow::Result<PathBuf> {
        self.commit_adjustments();
        let img = match &self.current_image {
            Some(i) => i.clone(),
            None => anyhow::bail!("No image currently loaded in editor"),
        };

        let rotated = match degrees % 360 {
            90 | -270 => img.rotate90(),
            180 | -180 => img.rotate180(),
            270 | -90 => img.rotate270(),
            _ => img.clone(),
        };

        self.push_undo(img);
        self.current_image = Some(rotated);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn flip(&mut self, horizontal: bool, vertical: bool) -> anyhow::Result<PathBuf> {
        self.commit_adjustments();
        let mut img = match &self.current_image {
            Some(i) => i.clone(),
            None => anyhow::bail!("No image currently loaded in editor"),
        };

        let previous = img.clone();
        if horizontal {
            img = img.fliph();
        }
        if vertical {
            img = img.flipv();
        }

        self.push_undo(previous);
        self.current_image = Some(img);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<PathBuf> {
        self.commit_adjustments();
        let img = match &self.current_image {
            Some(i) => i.clone(),
            None => anyhow::bail!("No image currently loaded in editor"),
        };

        if width == 0 || height == 0 {
            anyhow::bail!("Invalid resize dimensions");
        }

        let resized = img.resize_exact(width, height, imageops::FilterType::Lanczos3);

        self.push_undo(img);
        self.current_image = Some(resized);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn adjust(&mut self, brightness: i32, contrast: f32, saturation: i32) -> anyhow::Result<PathBuf> {
        if self.adjustment_base.is_none() {
            match &self.current_image {
                Some(i) => self.adjustment_base = Some(i.clone()),
                None => anyhow::bail!("No image currently loaded in editor"),
            }
        }

        let base = self.adjustment_base.as_ref().unwrap();
        let mut adjusted = base.clone();

        if brightness != 0 {
            adjusted = adjusted.brighten(brightness);
        }
        if contrast != 0.0 {
            adjusted = adjusted.adjust_contrast(contrast);
        }
        if saturation != 0 {
            let factor = 1.0 + (saturation as f32 / 100.0).clamp(-1.0, 3.0);
            match &mut adjusted {
                DynamicImage::ImageRgb8(rgb) => {
                    for pixel in rgb.pixels_mut() {
                        let r = pixel[0] as f32;
                        let g = pixel[1] as f32;
                        let b = pixel[2] as f32;
                        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        pixel[0] = (l + (r - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[1] = (l + (g - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[2] = (l + (b - l) * factor).clamp(0.0, 255.0).round() as u8;
                    }
                }
                DynamicImage::ImageRgba8(rgba) => {
                    for pixel in rgba.pixels_mut() {
                        let r = pixel[0] as f32;
                        let g = pixel[1] as f32;
                        let b = pixel[2] as f32;
                        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        pixel[0] = (l + (r - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[1] = (l + (g - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[2] = (l + (b - l) * factor).clamp(0.0, 255.0).round() as u8;
                    }
                }
                _ => {
                    let mut rgba = adjusted.to_rgba8();
                    for pixel in rgba.pixels_mut() {
                        let r = pixel[0] as f32;
                        let g = pixel[1] as f32;
                        let b = pixel[2] as f32;
                        let l = 0.2126 * r + 0.7152 * g + 0.0722 * b;
                        pixel[0] = (l + (r - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[1] = (l + (g - l) * factor).clamp(0.0, 255.0).round() as u8;
                        pixel[2] = (l + (b - l) * factor).clamp(0.0, 255.0).round() as u8;
                    }
                    adjusted = DynamicImage::ImageRgba8(rgba);
                }
            }
        }

        self.current_image = Some(adjusted);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn undo(&mut self) -> anyhow::Result<Option<PathBuf>> {
        self.commit_adjustments();
        if let Some(prev) = self.undo_stack.pop() {
            if let Some(curr) = self.current_image.take() {
                self.redo_stack.push(curr);
            }
            self.current_image = Some(prev);
            self.revision += 1;
            return Ok(Some(self.generate_preview()?));
        }
        Ok(None)
    }

    pub fn redo(&mut self) -> anyhow::Result<Option<PathBuf>> {
        self.commit_adjustments();
        if let Some(next) = self.redo_stack.pop() {
            if let Some(curr) = self.current_image.take() {
                self.undo_stack.push(curr);
            }
            self.current_image = Some(next);
            self.revision += 1;
            return Ok(Some(self.generate_preview()?));
        }
        Ok(None)
    }

    pub fn save(&self, overwrite: bool, new_path: Option<&Path>) -> anyhow::Result<PathBuf> {
        let img = match &self.current_image {
            Some(i) => i,
            None => anyhow::bail!("No image to save"),
        };

        let target_path = if let Some(p) = new_path {
            p.to_path_buf()
        } else if overwrite {
            self.original_path.clone()
        } else {
            Self::generate_copy_path(&self.original_path)
        };

        // Determine format according to extension
        let ext = target_path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase());

        let format = match ext.as_deref() {
            Some("jpg") | Some("jpeg") => ImageFormat::Jpeg,
            Some("png") => ImageFormat::Png,
            Some("webp") => ImageFormat::WebP,
            Some("bmp") => ImageFormat::Bmp,
            _ => ImageFormat::from_path(&target_path).unwrap_or(ImageFormat::Png),
        };

        // Atomic save using temporary file in the same directory
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        let mut temp_file = tempfile::Builder::new()
            .prefix(".zii_save_")
            .tempfile_in(parent)?;

        if format == ImageFormat::Jpeg {
            let mut file = std::io::BufWriter::new(&mut temp_file);
            let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, 95);
            img.write_with_encoder(encoder)
                .with_context(|| format!("Failed to encode JPEG image to {:?}", target_path))?;
            use std::io::Write;
            file.flush()
                .with_context(|| format!("Failed to flush encoded JPEG image to {:?}", target_path))?;
        } else {
            img.save_with_format(temp_file.path(), format)
                .with_context(|| format!("Failed to encode image to {:?}", target_path))?;
        }

        temp_file.persist(&target_path)
            .with_context(|| format!("Failed to persist saved file to {:?}", target_path))?;

        Ok(target_path)
    }

    fn push_undo(&mut self, img: DynamicImage) {
        self.undo_stack.push(img);
        self.redo_stack.clear();
        // Limit undo stack to 30 steps to preserve memory
        if self.undo_stack.len() > 30 {
            self.undo_stack.remove(0);
        }
    }

    fn generate_preview(&self) -> anyhow::Result<PathBuf> {
        let img = match &self.current_image {
            Some(i) => i,
            None => anyhow::bail!("No current image to preview"),
        };

        let _ = std::fs::create_dir_all(&self.cache_dir);
        let preview_path = self.cache_dir.join(format!("preview_{}.png", self.revision));
        img.save_with_format(&preview_path, ImageFormat::Png)?;

        // Delete previous revision preview files so cache does not accumulate images
        if let Ok(entries) = std::fs::read_dir(&self.cache_dir) {
            let current_filename = format!("preview_{}.png", self.revision);
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("preview_") && name_str.ends_with(".png") && name_str != current_filename {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }

        Ok(preview_path)
    }

    pub fn generate_copy_path(original: &Path) -> PathBuf {
        let parent = original.parent().unwrap_or_else(|| Path::new("."));
        let stem = original
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image");
        let ext = original
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png");

        let mut counter = 1;
        loop {
            let candidate_name = format!("{}_edited_{}.{}", stem, counter, ext);
            let candidate_path = parent.join(candidate_name);
            if !candidate_path.exists() {
                return candidate_path;
            }
            counter += 1;
        }
    }
}

impl Drop for ImageEditor {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_cleanup() {
        let mut editor = ImageEditor::new();
        let unique_cache = std::env::temp_dir().join(format!("zii_test_cleanup_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let _ = std::fs::create_dir_all(&unique_cache);
        editor.cache_dir = unique_cache.clone();
        assert!(unique_cache.exists());
        editor.cleanup();
        assert!(!unique_cache.exists());
    }

    #[test]
    fn test_apply_orientation() {
        // Create 20x10 image with top-left pixel black (0,0) and bottom-right pixel white (19,9)
        let mut img = DynamicImage::new_rgba8(20, 10);
        img.as_mut_rgba8().unwrap().put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));

        // Orientation 1: unchanged
        let o1 = apply_orientation(img.clone(), 1);
        assert_eq!(o1.dimensions(), (20, 10));
        assert_eq!(o1.as_rgba8().unwrap().get_pixel(0, 0)[0], 255);

        // Orientation 3: rotate 180 (red pixel moves to 19, 9)
        let o3 = apply_orientation(img.clone(), 3);
        assert_eq!(o3.dimensions(), (20, 10));
        assert_eq!(o3.as_rgba8().unwrap().get_pixel(19, 9)[0], 255);

        // Orientation 6: rotate 90 CW (dimensions swap to 10, 20)
        let o6 = apply_orientation(img.clone(), 6);
        assert_eq!(o6.dimensions(), (10, 20));
        assert_eq!(o6.as_rgba8().unwrap().get_pixel(9, 0)[0], 255);

        // Orientation 2: flip horizontal (red pixel moves to 19, 0)
        let o2 = apply_orientation(img.clone(), 2);
        assert_eq!(o2.dimensions(), (20, 10));
        assert_eq!(o2.as_rgba8().unwrap().get_pixel(19, 0)[0], 255);

        // Orientation 4: flip vertical (red pixel moves to 0, 9)
        let o4 = apply_orientation(img.clone(), 4);
        assert_eq!(o4.dimensions(), (20, 10));
        assert_eq!(o4.as_rgba8().unwrap().get_pixel(0, 9)[0], 255);

        // Orientation 5: transpose (dimensions swap to 10, 20, red pixel at 0, 0)
        let o5 = apply_orientation(img.clone(), 5);
        assert_eq!(o5.dimensions(), (10, 20));
        assert_eq!(o5.as_rgba8().unwrap().get_pixel(0, 0)[0], 255);

        // Orientation 7: transverse (dimensions swap to 10, 20, red pixel at 9, 19)
        let o7 = apply_orientation(img.clone(), 7);
        assert_eq!(o7.dimensions(), (10, 20));
        assert_eq!(o7.as_rgba8().unwrap().get_pixel(9, 19)[0], 255);

        // Orientation 8: rotate 270 CW (dimensions swap to 10, 20)
        let o8 = apply_orientation(img.clone(), 8);
        assert_eq!(o8.dimensions(), (10, 20));
        assert_eq!(o8.as_rgba8().unwrap().get_pixel(0, 19)[0], 255);
    }

    #[test]
    fn test_saturation_adjustment() {
        let mut editor = ImageEditor::new();
        editor.cache_dir = tempfile::tempdir().unwrap().keep();
        // Create an image with a pure red pixel (255, 0, 0)
        let mut img = DynamicImage::new_rgb8(2, 2);
        img.as_mut_rgb8().unwrap().put_pixel(0, 0, image::Rgb([255, 0, 0]));
        editor.current_image = Some(img);

        // Desaturate completely: saturation = -100
        editor.adjust(0, 0.0, -100).unwrap();
        let adjusted = editor.current_image.as_ref().unwrap();
        let p = adjusted.get_pixel(0, 0);
        // L = 0.2126 * 255 = 54.213 -> rounds to 54
        assert_eq!(p[0], 54);
        assert_eq!(p[1], 54);
        assert_eq!(p[2], 54);

        // Boost saturation: saturation = 50 on neutral base
        editor.adjustment_base = None; // reset base
        let mut img2 = DynamicImage::new_rgb8(2, 2);
        img2.as_mut_rgb8().unwrap().put_pixel(0, 0, image::Rgb([200, 100, 50]));
        editor.current_image = Some(img2);
        editor.adjust(0, 0.0, 50).unwrap();
        let adjusted2 = editor.current_image.as_ref().unwrap();
        let p2 = adjusted2.get_pixel(0, 0);
        // Factor is 1.5, colors should be pushed further from luminance
        assert!(p2[0] > 200);
        assert!(p2[2] < 50);
    }

    #[test]
    fn test_save_formats_and_jpeg_quality() {
        let dir = tempfile::tempdir().unwrap();
        let mut editor = ImageEditor::new();
        editor.cache_dir = tempfile::tempdir().unwrap().keep();
        let img = DynamicImage::new_rgb8(40, 30);
        editor.current_image = Some(img);

        // Test JPEG save (high quality 95)
        let jpg_path = dir.path().join("output.jpg");
        let saved_jpg = editor.save(false, Some(&jpg_path)).unwrap();
        assert_eq!(saved_jpg, jpg_path);
        assert!(jpg_path.exists());
        let loaded_jpg = image::open(&jpg_path).unwrap();
        assert_eq!(loaded_jpg.dimensions(), (40, 30));

        // Test PNG save
        let png_path = dir.path().join("output.png");
        let saved_png = editor.save(false, Some(&png_path)).unwrap();
        assert_eq!(saved_png, png_path);
        assert!(png_path.exists());
        let loaded_png = image::open(&png_path).unwrap();
        assert_eq!(loaded_png.dimensions(), (40, 30));

        // Test WebP save
        let webp_path = dir.path().join("output.webp");
        let saved_webp = editor.save(false, Some(&webp_path)).unwrap();
        assert_eq!(saved_webp, webp_path);
        assert!(webp_path.exists());
        let loaded_webp = image::open(&webp_path).unwrap();
        assert_eq!(loaded_webp.dimensions(), (40, 30));

        // Test BMP save
        let bmp_path = dir.path().join("output.bmp");
        let saved_bmp = editor.save(false, Some(&bmp_path)).unwrap();
        assert_eq!(saved_bmp, bmp_path);
        assert!(bmp_path.exists());
        let loaded_bmp = image::open(&bmp_path).unwrap();
        assert_eq!(loaded_bmp.dimensions(), (40, 30));
    }

    #[test]
    fn test_preview_cache_cleanup() {
        let mut editor = ImageEditor::new();
        editor.cache_dir = tempfile::tempdir().unwrap().keep();
        let img = DynamicImage::new_rgb8(10, 10);
        editor.current_image = Some(img);

        // Generate revisions 1, 2, 3
        editor.revision = 1;
        let p1 = editor.generate_preview().unwrap();
        assert!(p1.exists());

        editor.revision = 2;
        let p2 = editor.generate_preview().unwrap();
        assert!(p2.exists());
        // p1 should now be cleaned up
        assert!(!p1.exists());

        editor.revision = 3;
        let p3 = editor.generate_preview().unwrap();
        assert!(p3.exists());
        // p2 should now be cleaned up
        assert!(!p2.exists());

        // Cache dir should contain exactly 1 file (preview_3.png)
        let entries: Vec<_> = std::fs::read_dir(&editor.cache_dir)
            .unwrap()
            .flatten()
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name(), "preview_3.png");
    }
}

