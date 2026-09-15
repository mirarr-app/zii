use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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
    pub explicit_files: Option<Vec<PathBuf>>,
    pub probe_cache: HashMap<PathBuf, (SystemTime, u64, ImageEntry)>,
    #[cfg(test)]
    pub probe_count: std::cell::Cell<usize>,
}

const SUPPORTED_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff", "tif", "svg", "ico",
];

impl DirectoryScanner {
    pub fn new(target_path: &Path) -> anyhow::Result<Self> {
        let canonical = target_path.canonicalize().unwrap_or_else(|_| {
            if target_path.is_relative() {
                std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .join(target_path)
            } else {
                target_path.to_path_buf()
            }
        });

        let (dir, initial_file) = if canonical.is_dir() {
            (canonical, None)
        } else {
            let parent = canonical
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            (parent, Some(canonical))
        };

        let mut scanner = Self {
            current_dir: dir,
            entries: Vec::new(),
            current_index: 0,
            explicit_files: None,
            probe_cache: HashMap::new(),
            #[cfg(test)]
            probe_count: std::cell::Cell::new(0),
        };

        scanner.rescan()?;

        if let Some(target) = initial_file {
            if let Some(pos) = scanner.entries.iter().position(|e| e.path == target) {
                scanner.current_index = pos;
            }
        }

        Ok(scanner)
    }

    pub fn from_paths(paths: &[PathBuf]) -> anyhow::Result<Self> {
        if paths.is_empty() {
            return Self::new(Path::new("."));
        }
        if paths.len() == 1 {
            return Self::new(&paths[0]);
        }

        let mut explicit = Vec::new();

        for p in paths {
            let can = p.canonicalize().unwrap_or_else(|_| {
                if p.is_relative() {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(p)
                } else {
                    p.clone()
                }
            });
            // Only keep explicit files that look like images; otherwise a stray
            // non-image argument would show up as a 0x0 entry.
            if can.is_file() && Self::has_supported_extension(&can) {
                explicit.push(can);
            }
        }

        if explicit.is_empty() {
            let first_can = paths[0].canonicalize().unwrap_or_else(|_| {
                if paths[0].is_relative() {
                    std::env::current_dir()
                        .unwrap_or_else(|_| PathBuf::from("."))
                        .join(&paths[0])
                } else {
                    paths[0].clone()
                }
            });
            let dir = first_can
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or_else(|| Path::new("."));
            return Self::new(dir);
        }

        let current_dir = explicit[0]
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        let mut scanner = Self {
            current_dir,
            entries: Vec::new(),
            current_index: 0,
            explicit_files: Some(explicit),
            probe_cache: HashMap::new(),
            #[cfg(test)]
            probe_count: std::cell::Cell::new(0),
        };

        scanner.rescan()?;
        Ok(scanner)
    }

    pub fn rescan(&mut self) -> anyhow::Result<()> {
        let candidate_paths: Vec<PathBuf> = if let Some(ref explicit) = self.explicit_files {
            explicit.clone()
        } else {
            let mut found_paths = Vec::new();
            if let Ok(read_dir) = std::fs::read_dir(&self.current_dir) {
                for entry in read_dir.flatten() {
                    let path = entry.path();
                    if path.is_file() && Self::has_supported_extension(&path) {
                        found_paths.push(path);
                    }
                }
            }

            // Natural sort by filename
            found_paths.sort_by(|a, b| {
                let name_a = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let name_b = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
                natord::compare(name_a, name_b)
            });

            found_paths
        };

        let mut current_paths_set = std::collections::HashSet::new();
        let mut new_entries = Vec::new();

        for path in candidate_paths {
            if let Ok(meta) = std::fs::metadata(&path) {
                if !meta.is_file() {
                    continue;
                }
                current_paths_set.insert(path.clone());
                let mtime = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                let len = meta.len();

                if let Some((cached_mtime, cached_len, cached_entry)) = self.probe_cache.get(&path)
                {
                    if *cached_mtime == mtime && *cached_len == len {
                        new_entries.push(cached_entry.clone());
                        continue;
                    }
                }

                if let Some(entry) = self.probe_image(&path, &meta) {
                    self.probe_cache
                        .insert(path.clone(), (mtime, len, entry.clone()));
                    new_entries.push(entry);
                }
            }
        }

        // Drop cache entries for paths no longer present
        self.probe_cache
            .retain(|p, _| current_paths_set.contains(p));

        self.entries = new_entries;

        if self.current_index >= self.entries.len() && !self.entries.is_empty() {
            self.current_index = self.entries.len() - 1;
        }

        Ok(())
    }

    pub fn start_watcher(
        watch_dir: PathBuf,
        tx: tokio::sync::mpsc::Sender<()>,
    ) -> anyhow::Result<()> {
        if !watch_dir.exists() {
            return Ok(());
        }

        tokio::task::spawn_blocking(move || {
            let (notify_tx, notify_rx) = std::sync::mpsc::channel();
            let mut watcher = match notify::recommended_watcher(
                move |res: Result<notify::Event, notify::Error>| {
                    if let Ok(evt) = res {
                        use notify::event::EventKind;
                        match evt.kind {
                            EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                                let _ = notify_tx.send(());
                            }
                            _ => {}
                        }
                    }
                },
            ) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Directory watcher error: {:?}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&watch_dir, notify::RecursiveMode::NonRecursive) {
                eprintln!("Failed to watch directory {:?}: {:?}", watch_dir, e);
                return;
            }

            while let Ok(()) = notify_rx.recv() {
                // Debounce events by ~200ms
                std::thread::sleep(std::time::Duration::from_millis(200));
                while notify_rx.try_recv().is_ok() {}

                let _ = tx.blocking_send(());
            }
        });

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
        if let Some(ref mut explicit) = self.explicit_files {
            explicit.retain(|p| p != &removed.path);
        }
        if self.current_index >= self.entries.len() && !self.entries.is_empty() {
            self.current_index = self.entries.len() - 1;
        }
        Some(removed.path)
    }

    fn has_supported_extension(path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
    }

    fn probe_image(&self, path: &Path, metadata: &std::fs::Metadata) -> Option<ImageEntry> {
        #[cfg(test)]
        self.probe_count.set(self.probe_count.get() + 1);

        let filename = path.file_name()?.to_string_lossy().to_string();
        let file_size = metadata.len();
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        let (mut width, mut height) = if ext == "svg" {
            // Read dimensions from SVG if possible or default to 1024x1024
            Self::probe_svg_dimensions(path).unwrap_or((1024, 1024))
        } else {
            match image::image_dimensions(path) {
                Ok((w, h)) => (w, h),
                Err(_) => (0, 0),
            }
        };

        if let Some(orientation) = crate::editor::read_exif_orientation(path) {
            if (5..=8).contains(&orientation) {
                std::mem::swap(&mut width, &mut height);
            }
        }

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
                            num_a = num_a
                                .saturating_mul(10)
                                .saturating_add(d.to_digit(10).unwrap() as u64);
                            ca.next();
                        } else {
                            break;
                        }
                    }

                    let mut num_b: u64 = 0;
                    while let Some(d) = cb.peek() {
                        if d.is_ascii_digit() {
                            num_b = num_b
                                .saturating_mul(10)
                                .saturating_add(d.to_digit(10).unwrap() as u64);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_sample() {
        let sample = Path::new("tests/samples/sample_2.jpg");
        let scanner = DirectoryScanner::new(sample).unwrap();
        assert!(!scanner.entries.is_empty());
        assert_eq!(scanner.current().unwrap().filename, "sample_2.jpg");
    }

    #[test]
    fn test_from_paths_multiple() {
        let paths = vec![
            PathBuf::from("tests/samples/sample_1.png"),
            PathBuf::from("tests/samples/sample_2.jpg"),
        ];
        let scanner = DirectoryScanner::from_paths(&paths).unwrap();
        assert_eq!(scanner.entries.len(), 2);
        assert_eq!(scanner.entries[0].filename, "sample_1.png");
        assert_eq!(scanner.entries[1].filename, "sample_2.jpg");
        assert!(scanner.explicit_files.is_some());
    }

    #[test]
    fn test_from_paths_ignores_non_images() {
        let dir = tempfile::tempdir().unwrap();
        let txt = dir.path().join("notes.txt");
        let png = dir.path().join("pic.png");
        std::fs::write(&txt, "hello").unwrap();
        std::fs::copy("tests/samples/sample_1.png", &png).unwrap();

        // Mixed: the .txt is dropped, only the image is kept as an explicit entry.
        let scanner = DirectoryScanner::from_paths(&[txt.clone(), png.clone()]).unwrap();
        assert_eq!(scanner.entries.len(), 1);
        assert_eq!(scanner.entries[0].filename, "pic.png");
        assert!(scanner.explicit_files.is_some());

        // Only non-images: fall back to scanning the first argument's directory.
        let other_txt = dir.path().join("more.txt");
        std::fs::write(&other_txt, "x").unwrap();
        let scanner = DirectoryScanner::from_paths(&[txt, other_txt]).unwrap();
        assert!(scanner.explicit_files.is_none());
        assert_eq!(scanner.entries.len(), 1);
        assert_eq!(scanner.entries[0].filename, "pic.png");
    }

    #[test]
    fn test_bare_filename_relative() {
        // Test that a path without directory components like "sample_1.png" resolves cleanly
        let path = Path::new("tests/samples/sample_1.png");
        let scanner = DirectoryScanner::new(path).unwrap();
        assert!(!scanner.entries.is_empty());
        assert!(!scanner.current_dir.as_os_str().is_empty());
        assert_eq!(scanner.current().unwrap().filename, "sample_1.png");
    }

    #[test]
    fn test_scanner_probe_cache_rescan_reuses() {
        let path = Path::new("tests/samples");
        let mut scanner = DirectoryScanner::new(path).unwrap();
        let first_count = scanner.probe_count.get();
        assert!(first_count > 0);

        scanner.rescan().unwrap();
        let second_count = scanner.probe_count.get();
        assert_eq!(first_count, second_count);
    }
}
