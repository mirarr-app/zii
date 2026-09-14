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

pub struct ImageEditor {
    pub original_path: PathBuf,
    pub base_image: Option<DynamicImage>,
    pub undo_stack: Vec<DynamicImage>,
    pub redo_stack: Vec<DynamicImage>,
    pub current_image: Option<DynamicImage>,
    pub cache_dir: PathBuf,
    pub revision: usize,
}

impl ImageEditor {
    pub fn new() -> Self {
        let cache_dir = std::env::temp_dir().join("zii_preview_cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        Self {
            original_path: PathBuf::new(),
            base_image: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            current_image: None,
            cache_dir,
            revision: 0,
        }
    }

    pub fn open(&mut self, path: &Path) -> anyhow::Result<PathBuf> {
        self.original_path = path.to_path_buf();
        let img = image::open(path)
            .with_context(|| format!("Failed to open image at {:?}", path))?;
        self.base_image = Some(img.clone());
        self.current_image = Some(img);
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

    pub fn crop(&mut self, x: u32, y: u32, width: u32, height: u32) -> anyhow::Result<PathBuf> {
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

    pub fn adjust(&mut self, brightness: i32, contrast: f32) -> anyhow::Result<PathBuf> {
        let img = match &self.current_image {
            Some(i) => i.clone(),
            None => anyhow::bail!("No image currently loaded in editor"),
        };

        let previous = img.clone();
        let mut adjusted = img;

        if brightness != 0 {
            adjusted = adjusted.brighten(brightness);
        }
        if contrast != 0.0 {
            adjusted = adjusted.adjust_contrast(contrast);
        }

        self.push_undo(previous);
        self.current_image = Some(adjusted);
        self.revision += 1;
        self.generate_preview()
    }

    pub fn undo(&mut self) -> anyhow::Result<Option<PathBuf>> {
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

        // Determine format
        let format = ImageFormat::from_path(&target_path)
            .unwrap_or(ImageFormat::Png);

        // Atomic save using temporary file in the same directory
        let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_file = tempfile::Builder::new()
            .prefix(".zii_save_")
            .tempfile_in(parent)?;

        img.save_with_format(temp_file.path(), format)
            .with_context(|| format!("Failed to encode image to {:?}", target_path))?;

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

        let preview_path = self.cache_dir.join(format!("preview_{}.png", self.revision));
        img.save_with_format(&preview_path, ImageFormat::Png)?;
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
