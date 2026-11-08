use crate::privacy;
use crate::settings::Settings;
use crate::storage::{ItemKind, NewItem, Store};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub static PAUSED: AtomicBool = AtomicBool::new(false);

pub enum Captured {
    Text(String),
    Image {
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

pub fn preview_of(text: &str) -> String {
    let single: String = text
        .chars()
        .take(160)
        .collect::<String>()
        .replace(['\n', '\r', '\t'], " ");
    single
}

pub fn capture_text(
    store: &Arc<Store>,
    settings: &Settings,
    text: String,
) -> Result<Option<i64>, String> {
    if text.is_empty() || text.len() > 10_000_000 {
        return Ok(None);
    }
    let sensitive = privacy::is_sensitive(&text);
    if sensitive && !settings.allow_sensitive {
        let hash = hash_bytes(text.as_bytes());
        let item = NewItem {
            hash,
            kind: ItemKind::Redacted,
            text: Some(privacy::redact_placeholder().to_string()),
            preview: privacy::redact_placeholder().to_string(),
            byte_len: 0,
            source_app: None,
            sensitive: true,
            blob_path: None,
            width: None,
            height: None,
        };
        let id = store.insert_item(&item)?;
        if id.is_some() {
            store.log_activity(
                "redacted",
                "Sensitive clipboard content was withheld, hash kept only",
            );
        }
        return Ok(id);
    }
    let hash = hash_bytes(text.as_bytes());
    let preview = preview_of(&text);
    let item = NewItem {
        hash,
        kind: ItemKind::Text,
        text: Some(text.clone()),
        preview,
        byte_len: text.len() as u64,
        source_app: None,
        sensitive,
        blob_path: None,
        width: None,
        height: None,
    };
    let id = store.insert_item(&item)?;
    if id.is_some() {
        store.log_activity("capture", "Text clipboard item captured");
    }
    Ok(id)
}

pub fn capture_image(
    store: &Arc<Store>,
    settings: &Settings,
    width: u32,
    height: u32,
    rgba: Vec<u8>,
) -> Result<Option<i64>, String> {
    if !settings.capture_images || width == 0 || height == 0 || rgba.is_empty() {
        return Ok(None);
    }
    let png_bytes = encode_png(width, height, &rgba)?;
    let hash = hash_bytes(&png_bytes);
    let blob_path = store.write_blob(&hash, &png_bytes)?;
    let item = NewItem {
        hash,
        kind: ItemKind::Image,
        text: None,
        preview: format!("Image {}x{}", width, height),
        byte_len: png_bytes.len() as u64,
        source_app: None,
        sensitive: false,
        blob_path: Some(blob_path),
        width: Some(width),
        height: Some(height),
    };
    let id = store.insert_item(&item)?;
    if id.is_some() {
        store.log_activity("capture", "Image clipboard item captured");
    }
    Ok(id)
}

pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    writer.finish().map_err(|e| e.to_string())?;
    Ok(out)
}

pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => {
            let rgb = &buf[..info.buffer_size()];
            let mut out = Vec::with_capacity(rgb.len() / 3 * 4);
            let mut i = 0;
            while i + 2 < rgb.len() {
                out.extend_from_slice(&[rgb[i], rgb[i + 1], rgb[i + 2], 255]);
                i += 3;
            }
            out
        }
        _ => return Err("unsupported PNG color type for clipboard restore".to_string()),
    };
    Ok((info.width, info.height, rgba))
}

pub fn restore_to_clipboard(store: &Arc<Store>, id: i64) -> Result<String, String> {
    let item = store
        .get_item(id)?
        .ok_or_else(|| format!("no clipboard item with id {}", id))?;
    match item.kind {
        ItemKind::Text => {
            let text = item.text.ok_or_else(|| "item has no text".to_string())?;
            set_clipboard_text(&text)?;
            store.log_activity("restore", "Text item copied back to clipboard");
            Ok(format!("copied text item {} back to clipboard", id))
        }
        ItemKind::Image => {
            let rel = item
                .blob_path
                .ok_or_else(|| "image item has no stored data".to_string())?;
            let bytes = store.read_blob(&rel)?;
            let (w, h, rgba) = decode_png(&bytes)?;
            set_clipboard_image(w, h, rgba)?;
            store.log_activity("restore", "Image item copied back to clipboard");
            Ok(format!("copied image item {} back to clipboard", id))
        }
        ItemKind::Redacted => {
            Err("redacted item was never stored and cannot be restored".to_string())
        }
    }
}

pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    let mut ctx = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    ctx.set_text(text).map_err(|e| e.to_string())
}

pub fn set_clipboard_image(width: u32, height: u32, rgba: Vec<u8>) -> Result<(), String> {
    let mut ctx = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let img = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: rgba.into(),
    };
    ctx.set_image(img).map_err(|e| e.to_string())
}

pub fn get_clipboard_snapshot() -> Option<Captured> {
    let mut ctx = arboard::Clipboard::new().ok()?;
    if let Ok(img) = ctx.get_image() {
        let bytes: Vec<u8> = img.bytes.to_vec();
        return Some(Captured::Image {
            width: img.width as u32,
            height: img.height as u32,
            rgba: bytes,
        });
    }
    if let Ok(text) = ctx.get_text() {
        return Some(Captured::Text(text));
    }
    None
}

pub fn start_monitor(store: Arc<Store>, settings: Settings) {
    std::thread::Builder::new()
        .name("clipboard-monitor".to_string())
        .spawn(move || {
            let mut last_hash = String::new();
            loop {
                if !PAUSED.load(Ordering::Relaxed) && settings.monitoring_enabled {
                    if let Some(snap) = get_clipboard_snapshot() {
                        match snap {
                            Captured::Text(text) => {
                                let hash = hash_bytes(text.as_bytes());
                                if hash != last_hash {
                                    last_hash = hash;
                                    if settings.capture_text {
                                        let _ = capture_text(&store, &settings, text);
                                    }
                                }
                            }
                            Captured::Image {
                                width,
                                height,
                                rgba,
                            } => {
                                let hash = hash_bytes(&rgba);
                                if hash != last_hash {
                                    last_hash = hash;
                                    let _ = capture_image(&store, &settings, width, height, rgba);
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(settings.poll_interval_ms.max(200)));
            }
        })
        .ok();
}
