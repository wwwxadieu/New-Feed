//! Lưu trạng thái ứng dụng ra đĩa dưới dạng JSON trong thư mục cấu hình của người dùng.

use crate::model::AppData;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn state_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Không xác định được thư mục cấu hình: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Không tạo được thư mục cấu hình: {e}"))?;
    Ok(dir.join("state.json"))
}

pub fn load(app: &AppHandle) -> AppData {
    let Ok(path) = state_path(app) else {
        return AppData::default();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return AppData::default();
    };
    // Nếu file hỏng thì bắt đầu lại từ đầu thay vì làm sập ứng dụng.
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(app: &AppHandle, data: &AppData) -> Result<(), String> {
    let path = state_path(app)?;
    let raw = serde_json::to_string_pretty(data).map_err(|e| format!("Không tuần tự hoá được dữ liệu: {e}"))?;
    // Ghi ra file tạm rồi đổi tên: mất điện giữa chừng không làm hỏng state.json.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| format!("Không ghi được dữ liệu: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Không lưu được dữ liệu: {e}"))?;
    Ok(())
}

/// Vài nguồn công nghệ có sẵn để lần chạy đầu tiên không phải màn hình trống.
pub fn default_sources() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("VnExpress Số hóa", "https://vnexpress.net/so-hoa", "https://vnexpress.net/rss/so-hoa.rss"),
        ("Genk", "https://genk.vn", "https://genk.vn/rss/home.rss"),
        ("VietnamNet Công nghệ", "https://vietnamnet.vn/cong-nghe", "https://vietnamnet.vn/rss/cong-nghe.rss"),
        ("TechCrunch", "https://techcrunch.com", "https://techcrunch.com/feed/"),
        ("The Verge", "https://www.theverge.com", "https://www.theverge.com/rss/index.xml"),
        ("Ars Technica", "https://arstechnica.com", "https://feeds.arstechnica.com/arstechnica/index"),
    ]
}
