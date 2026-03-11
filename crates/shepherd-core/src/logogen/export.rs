use anyhow::{Context, Result};
use base64::Engine;
use image::imageops::FilterType;
use image::DynamicImage;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use crate::logogen::ExportedFile;
use crate::logogen::IconExport;

/// Standard PNG icon sizes for export.
pub const PNG_SIZES: &[(u32, &str)] = &[
    (1024, "icon-1024.png"),
    (512, "icon-512.png"),
    (192, "icon-192.png"),
    (64, "icon-64.png"),
];

/// Export icons in multiple formats from a base64-encoded PNG.
///
/// Creates PNG icons at standard sizes, favicon.ico, apple-touch-icon,
/// app.ico, app.icns (or placeholder), logo.svg placeholder, and manifest.json.
pub fn export_icons(
    png_base64: &str,
    output_dir: &Path,
    product_name: &str,
) -> Result<IconExport> {
    let png_bytes = base64::engine::general_purpose::STANDARD
        .decode(png_base64)
        .context("Failed to decode base64 PNG data")?;

    let img = image::load_from_memory(&png_bytes).context("Failed to load PNG image from decoded bytes")?;

    fs::create_dir_all(output_dir).context("Failed to create output directory")?;

    let mut files = Vec::new();

    // Export PNG sizes
    for &(size, filename) in PNG_SIZES {
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        let path = output_dir.join(filename);
        resized
            .save(&path)
            .with_context(|| format!("Failed to save {filename}"))?;
        let metadata = fs::metadata(&path)?;
        files.push(ExportedFile {
            path: path.to_string_lossy().to_string(),
            size_bytes: metadata.len(),
            format: "png".to_string(),
            dimensions: Some((size, size)),
        });
    }

    // Apple touch icon (180x180)
    let apple_icon = img.resize_exact(180, 180, FilterType::Lanczos3);
    let apple_path = output_dir.join("apple-touch-icon.png");
    apple_icon
        .save(&apple_path)
        .context("Failed to save apple-touch-icon.png")?;
    let apple_meta = fs::metadata(&apple_path)?;
    files.push(ExportedFile {
        path: apple_path.to_string_lossy().to_string(),
        size_bytes: apple_meta.len(),
        format: "png".to_string(),
        dimensions: Some((180, 180)),
    });

    // Favicon.ico (16, 32, 48)
    let favicon_path = output_dir.join("favicon.ico");
    export_ico(&img, &favicon_path, &[16, 32, 48])?;
    let favicon_meta = fs::metadata(&favicon_path)?;
    files.push(ExportedFile {
        path: favicon_path.to_string_lossy().to_string(),
        size_bytes: favicon_meta.len(),
        format: "ico".to_string(),
        dimensions: None,
    });

    // App.ico (16, 32, 48, 256)
    let app_ico_path = output_dir.join("app.ico");
    export_ico(&img, &app_ico_path, &[16, 32, 48, 256])?;
    let app_ico_meta = fs::metadata(&app_ico_path)?;
    files.push(ExportedFile {
        path: app_ico_path.to_string_lossy().to_string(),
        size_bytes: app_ico_meta.len(),
        format: "ico".to_string(),
        dimensions: None,
    });

    // App.icns (macOS icon)
    let icns_path = output_dir.join("app.icns");
    export_icns(&img, &icns_path)?;
    let icns_meta = fs::metadata(&icns_path)?;
    files.push(ExportedFile {
        path: icns_path.to_string_lossy().to_string(),
        size_bytes: icns_meta.len(),
        format: "icns".to_string(),
        dimensions: None,
    });

    // Logo SVG placeholder
    let svg_path = output_dir.join("logo.svg");
    let svg_content = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 512 512\" width=\"512\" height=\"512\">\n\
         \x20\x20<!-- Placeholder SVG for {product_name} - replace with vector version -->\n\
         \x20\x20<rect width=\"512\" height=\"512\" fill=\"#f0f0f0\" rx=\"64\"/>\n\
         \x20\x20<text x=\"256\" y=\"280\" text-anchor=\"middle\" font-family=\"system-ui, sans-serif\" \
         font-size=\"48\" fill=\"#333\">{product_name}</text>\n\
         </svg>"
    );
    fs::write(&svg_path, &svg_content).context("Failed to write logo.svg")?;
    let svg_meta = fs::metadata(&svg_path)?;
    files.push(ExportedFile {
        path: svg_path.to_string_lossy().to_string(),
        size_bytes: svg_meta.len(),
        format: "svg".to_string(),
        dimensions: Some((512, 512)),
    });

    // Manifest JSON
    let manifest_path = output_dir.join("manifest.json");
    let manifest = generate_manifest_json(product_name);
    fs::write(&manifest_path, &manifest).context("Failed to write manifest.json")?;
    let manifest_meta = fs::metadata(&manifest_path)?;
    files.push(ExportedFile {
        path: manifest_path.to_string_lossy().to_string(),
        size_bytes: manifest_meta.len(),
        format: "json".to_string(),
        dimensions: None,
    });

    Ok(IconExport { files })
}

/// Write an ICO file containing PNG-encoded images at the specified sizes.
///
/// Uses the ICO format with PNG-compressed directory entries.
pub fn export_ico(img: &DynamicImage, path: &Path, sizes: &[u32]) -> Result<()> {
    let mut file = fs::File::create(path).context("Failed to create ICO file")?;

    // ICO Header: reserved (2), type=1 (2), count (2)
    let count = sizes.len() as u16;
    file.write_all(&[0, 0])?; // reserved
    file.write_all(&1u16.to_le_bytes())?; // type: icon
    file.write_all(&count.to_le_bytes())?; // image count

    // Prepare PNG blobs for each size
    let mut png_blobs: Vec<Vec<u8>> = Vec::new();
    for &size in sizes {
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        let mut buf = std::io::Cursor::new(Vec::new());
        resized
            .write_to(&mut buf, image::ImageFormat::Png)
            .with_context(|| format!("Failed to encode {size}x{size} PNG for ICO"))?;
        png_blobs.push(buf.into_inner());
    }

    // Calculate data offset: header (6) + directory entries (16 each)
    let dir_size = 6 + (sizes.len() as u32) * 16;
    let mut offset = dir_size;

    // Write directory entries
    for (i, &size) in sizes.iter().enumerate() {
        let w: u8 = if size >= 256 { 0 } else { size as u8 };
        let h: u8 = w;
        file.write_all(&[w, h])?; // width, height (0 = 256)
        file.write_all(&[0])?; // color palette count
        file.write_all(&[0])?; // reserved
        file.write_all(&1u16.to_le_bytes())?; // color planes
        file.write_all(&32u16.to_le_bytes())?; // bits per pixel
        let blob_size = png_blobs[i].len() as u32;
        file.write_all(&blob_size.to_le_bytes())?; // image data size
        file.write_all(&offset.to_le_bytes())?; // offset to image data
        offset += blob_size;
    }

    // Write image data
    for blob in &png_blobs {
        file.write_all(blob)?;
    }

    Ok(())
}

/// Create a macOS .icns file using iconutil, or write a placeholder if unavailable.
///
/// Creates a temporary .iconset directory with standard sizes and @2x variants,
/// then attempts to convert via `iconutil --convert icns`. Falls back to a
/// placeholder file if iconutil is not available.
pub fn export_icns(img: &DynamicImage, path: &Path) -> Result<()> {
    // Standard iconset sizes: name => (size, is_retina)
    let iconset_entries: &[(&str, u32)] = &[
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ];

    // Create temporary .iconset directory next to output path
    let iconset_dir = path.with_extension("iconset");
    fs::create_dir_all(&iconset_dir).context("Failed to create .iconset directory")?;

    for &(name, size) in iconset_entries {
        let resized = img.resize_exact(size, size, FilterType::Lanczos3);
        let entry_path = iconset_dir.join(name);
        resized
            .save(&entry_path)
            .with_context(|| format!("Failed to save iconset entry {name}"))?;
    }

    // Try to use iconutil (macOS only)
    let result = Command::new("iconutil")
        .arg("--convert")
        .arg("icns")
        .arg("--output")
        .arg(path)
        .arg(&iconset_dir)
        .output();

    // Clean up iconset directory
    let _ = fs::remove_dir_all(&iconset_dir);

    match result {
        Ok(output) if output.status.success() => Ok(()),
        _ => {
            // Write a placeholder ICNS file
            // ICNS magic: 'icns' + total file size (u32 BE)
            let placeholder = b"icns\x00\x00\x00\x08";
            fs::write(path, placeholder).context("Failed to write placeholder .icns")?;
            Ok(())
        }
    }
}

/// Generate a PWA manifest.json with icon entries.
pub fn generate_manifest_json(name: &str) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": name,
        "short_name": name,
        "icons": [
            {
                "src": "icon-192.png",
                "sizes": "192x192",
                "type": "image/png"
            },
            {
                "src": "icon-512.png",
                "sizes": "512x512",
                "type": "image/png"
            },
            {
                "src": "apple-touch-icon.png",
                "sizes": "180x180",
                "type": "image/png"
            }
        ],
        "theme_color": "#ffffff",
        "background_color": "#ffffff",
        "display": "standalone"
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_manifest_json_test() {
        let manifest = generate_manifest_json("TestApp");
        let parsed: serde_json::Value = serde_json::from_str(&manifest).unwrap();

        assert_eq!(parsed["name"], "TestApp");
        assert_eq!(parsed["short_name"], "TestApp");
        assert_eq!(parsed["display"], "standalone");

        let icons = parsed["icons"].as_array().unwrap();
        assert_eq!(icons.len(), 3);

        // Check that expected sizes are present
        let sizes: Vec<&str> = icons
            .iter()
            .map(|i| i["sizes"].as_str().unwrap())
            .collect();
        assert!(sizes.contains(&"192x192"));
        assert!(sizes.contains(&"512x512"));
        assert!(sizes.contains(&"180x180"));

        // All icons should be image/png
        for icon in icons {
            assert_eq!(icon["type"], "image/png");
        }
    }

    #[test]
    fn png_sizes_correct() {
        assert_eq!(PNG_SIZES.len(), 4);

        let sizes: Vec<u32> = PNG_SIZES.iter().map(|(s, _)| *s).collect();
        assert!(sizes.contains(&1024));
        assert!(sizes.contains(&512));
        assert!(sizes.contains(&192));
        assert!(sizes.contains(&64));

        // Sizes should be in descending order
        for window in sizes.windows(2) {
            assert!(
                window[0] > window[1],
                "PNG_SIZES should be in descending order"
            );
        }

        // Filenames should match sizes
        for &(size, name) in PNG_SIZES {
            assert!(
                name.contains(&size.to_string()),
                "Filename {name} should contain size {size}"
            );
            assert!(name.ends_with(".png"), "Filename {name} should end with .png");
        }
    }

    #[test]
    fn export_icons_invalid_base64() {
        let tmp = tempfile::tempdir().unwrap();
        let result = export_icons("not-valid-base64!!!", tmp.path(), "Test");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("base64") || err.contains("decode") || err.contains("Base64"),
            "Error should mention base64 decoding: {err}"
        );
    }
}
