use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ExifMetadata {
    pub make: Option<String>,
    pub model: Option<String>,
    pub lens_model: Option<String>,
    pub shutter_speed: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub exposure_bias: Option<String>,
    pub flash: Option<String>,
    pub date_time: Option<String>,
    pub dimensions: String,
    pub megapixels: Option<String>,
    pub file_size: String,
    pub format: String,
    pub path: String,
    pub filename: String,
}

fn clean_str(s: &str) -> String {
    let trimmed = s.trim();
    if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
        && trimmed.len() >= 2
    {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn extract_metadata(
    path: &Path,
    width: u32,
    height: u32,
    file_size_bytes: u64,
    format_name: &str,
) -> ExifMetadata {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy().to_string();
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let (dimensions, megapixels) = if width > 0 && height > 0 {
        let mp = (width as f64 * height as f64) / 1_000_000.0;
        (
            format!("{} × {} ({:.1} MP)", width, height, mp),
            Some(format!("{:.1} MP", mp)),
        )
    } else {
        ("Unknown".to_string(), None)
    };

    let file_size = format_file_size(file_size_bytes);
    let format = format_name.to_uppercase();

    let mut meta = ExifMetadata {
        make: None,
        model: None,
        lens_model: None,
        shutter_speed: None,
        f_number: None,
        iso: None,
        focal_length: None,
        exposure_bias: None,
        flash: None,
        date_time: None,
        dimensions,
        megapixels,
        file_size,
        format,
        path: path_str,
        filename,
    };

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return meta,
    };

    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = match exifreader.read_from_container(&mut bufreader) {
        Ok(e) => e,
        Err(_) => return meta,
    };

    // Camera Make & Model
    if let Some(f) = exif.get_field(exif::Tag::Make, exif::In::PRIMARY) {
        let val = clean_str(&f.display_value().to_string());
        if !val.is_empty() {
            meta.make = Some(val);
        }
    }

    if let Some(f) = exif.get_field(exif::Tag::Model, exif::In::PRIMARY) {
        let val = clean_str(&f.display_value().to_string());
        if !val.is_empty() {
            meta.model = Some(val);
        }
    }

    // Lens Model
    if let Some(f) = exif.get_field(exif::Tag::LensModel, exif::In::PRIMARY) {
        let val = clean_str(&f.display_value().to_string());
        if !val.is_empty() {
            meta.lens_model = Some(val);
        }
    }

    // Shutter Speed / Exposure Time
    if let Some(f) = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        let val = raw.trim_end_matches(" s").trim_end_matches('s').trim();
        meta.shutter_speed = Some(format!("{}s", val));
    } else if let Some(f) = exif.get_field(exif::Tag::ShutterSpeedValue, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        meta.shutter_speed = Some(raw);
    }

    // F-Number / Aperture
    if let Some(f) = exif.get_field(exif::Tag::FNumber, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        if raw.starts_with("f/") || raw.starts_with("F/") {
            meta.f_number = Some(raw);
        } else {
            meta.f_number = Some(format!("f/{}", raw));
        }
    } else if let Some(f) = exif.get_field(exif::Tag::ApertureValue, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        if raw.starts_with("f/") || raw.starts_with("F/") {
            meta.f_number = Some(raw);
        } else {
            meta.f_number = Some(format!("f/{}", raw));
        }
    }

    // ISO
    if let Some(f) = exif.get_field(exif::Tag::PhotographicSensitivity, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        if raw.to_uppercase().starts_with("ISO") {
            meta.iso = Some(raw);
        } else {
            meta.iso = Some(format!("ISO {}", raw));
        }
    } else if let Some(f) = exif.get_field(exif::Tag::ISOSpeed, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        if raw.to_uppercase().starts_with("ISO") {
            meta.iso = Some(raw);
        } else {
            meta.iso = Some(format!("ISO {}", raw));
        }
    }

    // Focal Length
    if let Some(f) = exif.get_field(exif::Tag::FocalLength, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        let val = raw.trim_end_matches(" mm").trim_end_matches("mm").trim();
        meta.focal_length = Some(format!("{}mm", val));
    }

    // Exposure Bias
    if let Some(f) = exif.get_field(exif::Tag::ExposureBiasValue, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        meta.exposure_bias = Some(raw);
    }

    // Flash status
    if let Some(f) = exif.get_field(exif::Tag::Flash, exif::In::PRIMARY) {
        let raw = clean_str(&f.display_value().to_string());
        meta.flash = Some(raw);
    }

    // Date & Time taken
    let date_field = exif
        .get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)
        .or_else(|| exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
        .or_else(|| exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY));

    if let Some(f) = date_field {
        let raw = clean_str(&f.display_value().to_string());
        meta.date_time = Some(format_exif_date(&raw));
    }

    meta
}

pub fn format_exif_date(raw: &str) -> String {
    let bytes = raw.as_bytes();
    if raw.len() >= 10
        && raw.is_char_boundary(4)
        && raw.is_char_boundary(5)
        && raw.is_char_boundary(7)
        && raw.is_char_boundary(8)
        && bytes.get(4) == Some(&b':')
        && bytes.get(7) == Some(&b':')
    {
        format!("{}-{}-{}", &raw[0..4], &raw[5..7], &raw[8..])
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(500), "500 B");
        assert_eq!(format_file_size(2048), "2.0 KB");
        assert_eq!(format_file_size(1048576 * 3), "3.0 MB");
        assert_eq!(format_file_size(1073741824 * 2), "2.00 GB");
    }

    #[test]
    fn test_format_exif_date_safe_with_non_ascii() {
        // Standard ASCII date format
        assert_eq!(
            format_exif_date("2023:05:14 15:30:00"),
            "2023-05-14 15:30:00"
        );

        // Non-ASCII string where byte 4 or 7 is not a char boundary
        // 'П' (2 bytes), 'р' (2 bytes) -> byte 4 is start of 'и' (2 bytes)
        // A direct slice &s[4..5] panics on "Привет:мир"
        let non_ascii = "Привет:мир";
        assert_eq!(format_exif_date(non_ascii), non_ascii);

        // Japanese date with multi-byte characters
        let cjk = "2023年05月14日";
        assert_eq!(format_exif_date(cjk), cjk);

        // Short string
        assert_eq!(format_exif_date("2023:01"), "2023:01");
    }

    #[test]
    fn test_extract_metadata_sample_fallback() {
        let path = Path::new("tests/samples/sample_1.png");
        let meta = extract_metadata(path, 100, 100, 2048, "PNG");
        assert_eq!(meta.format, "PNG");
        assert_eq!(meta.dimensions, "100 × 100 (0.0 MP)");
        assert_eq!(meta.filename, "sample_1.png");
    }

    #[test]
    fn test_extract_metadata_sample_jpeg() {
        let path = Path::new("tests/samples/sample_2.jpg");
        let meta = extract_metadata(path, 200, 200, 5000, "JPEG");
        assert_eq!(meta.format, "JPEG");
        assert_eq!(meta.filename, "sample_2.jpg");
    }
}
