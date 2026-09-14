use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    pub path: PathBuf,
    pub filename: String,
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub format: String,
}

pub struct DirectoryScanner {
    pub current_dir: PathBuf,
    pub entries: Vec<ImageEntry>,
    pub current_index: usize,
}

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg", "ico",
];

impl DirectoryScanner {
    pub fn new(target_path: &Path) -> anyhow::Result<Self> {
        let (dir, initial_file) = if target_path.is_dir() {
            (target_path.to_path_buf(), None)
        } else {
            let parent = target_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            (parent, Some(target_path.canonicalize().unwrap_or_else(|_| target_path.to_path_buf())))
        };

        let mut scanner = Self {
            current_dir: dir.clone(),
            entries: Vec::new(),
            current_index: 0,
        };

        scanner.rescan()?;

        if let Some(target) = initial_file {
            if let Some(pos) = scanner.entries.iter().position(|e| e.path == target) {
                scanner.current_index = pos;
            }
        }

        Ok(scanner)
    }

    pub fn rescan(&mut self) -> anyhow::Result<()> {
        let mut found_paths = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let lower_ext = ext.to_ascii_lowercase();
                        if SUPPORTED_EXTENSIONS.contains(&lower_ext.as_str()) {
                            found_paths.push(path);
                        }
                    }
                }
            }
        }

        // Natural sort by filename
        found_paths.sort_by(|a, b| {
            let name_a = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let name_b = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            natord::compare(name_a, name_b)
        });

        self.entries = found_paths
            .into_iter()
            .filter_map(|p| Self::probe_image(&p))
            .collect();

        if self.current_index >= self.entries.len() && !self.entries.is_empty() {
            self.current_index = self.entries.len() - 1;
        }

        Ok(())
    }

    pub fn current(&self) -> Option<&ImageEntry> {
        self.entries.get(self.current_index)
    }

    pub fn next(&mut self) -> Option<&ImageEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.entries.len();
        self.current()
    }

    pub fn prev(&mut self) -> Option<&ImageEntry> {
        if self.entries.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.entries.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.current()
    }

    pub fn first(&mut self) -> Option<&ImageEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.current_index = 0;
        self.current()
    }

    pub fn last(&mut self) -> Option<&ImageEntry> {
        if self.entries.is_empty() {
            return None;
        }
        self.current_index = self.entries.len() - 1;
        self.current()
    }

    pub fn go_to(&mut self, index: usize) -> Option<&ImageEntry> {
        if self.entries.is_empty() {
            return None;
        }
        if index < self.entries.len() {
            self.current_index = index;
        }
        self.current()
    }

    pub fn remove_current(&mut self) -> Option<PathBuf> {
        if self.entries.is_empty() {
            return None;
        }
        let removed = self.entries.remove(self.current_index);
        if self.current_index >= self.entries.len() && !self.entries.is_empty() {
            self.current_index = self.entries.len() - 1;
        }
        Some(removed.path)
    }

    fn probe_image(path: &Path) -> Option<ImageEntry> {
        let filename = path.file_name()?.to_string_lossy().to_string();
        let metadata = std::fs::metadata(path).ok()?;
        let file_size = metadata.len();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let (width, height) = if ext == "svg" {
            // Read dimensions from SVG if possible or default to 1024x1024
            Self::probe_svg_dimensions(path).unwrap_or((1024, 1024))
        } else {
            match image::image_dimensions(path) {
                Ok((w, h)) => (w, h),
                Err(_) => (0, 0),
            }
        };

        Some(ImageEntry {
            path: path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            filename,
            width,
            height,
            file_size,
            format: ext.to_uppercase(),
        })
    }

    fn probe_svg_dimensions(path: &Path) -> Option<(u32, u32)> {
        let content = std::fs::read_to_string(path).ok()?;
        // Simple search for width and height in <svg ...>
        let w = Self::extract_svg_attr(&content, "width")?;
        let h = Self::extract_svg_attr(&content, "height")?;
        Some((w, h))
    }

    fn extract_svg_attr(svg: &str, attr: &str) -> Option<u32> {
        let pattern = format!("{}=\"", attr);
        if let Some(pos) = svg.find(&pattern) {
            let rest = &svg[pos + pattern.len()..];
            let val_str: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(val) = val_str.parse::<f32>() {
                return Some(val.round() as u32);
            }
        }
        None
    }
}

// Minimal natural order sorting helper so external crate isn't strictly required
mod natord {
    use std::cmp::Ordering;

    pub fn compare(a: &str, b: &str) -> Ordering {
        let mut ca = a.chars().peekable();
        let mut cb = b.chars().peekable();

        loop {
            match (ca.peek(), cb.peek()) {
                (None, None) => return Ordering::Equal,
                (None, Some(_)) => return Ordering::Less,
                (Some(_), None) => return Ordering::Greater,
                (Some(c1), Some(c2)) if c1.is_ascii_digit() && c2.is_ascii_digit() => {
                    let mut num_a: u64 = 0;
                    while let Some(d) = ca.peek() {
                        if d.is_ascii_digit() {
                            num_a = num_a.saturating_mul(10).saturating_add(d.to_digit(10).unwrap() as u64);
                            ca.next();
                        } else {
                            break;
                        }
                    }

                    let mut num_b: u64 = 0;
                    while let Some(d) = cb.peek() {
                        if d.is_ascii_digit() {
                            num_b = num_b.saturating_mul(10).saturating_add(d.to_digit(10).unwrap() as u64);
                            cb.next();
                        } else {
                            break;
                        }
                    }

                    match num_a.cmp(&num_b) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                (Some(c1), Some(c2)) => {
                    let o = c1.to_ascii_lowercase().cmp(&c2.to_ascii_lowercase());
                    if o != Ordering::Equal {
                        return o;
                    }
                    ca.next();
                    cb.next();
                }
            }
        }
    }
}
