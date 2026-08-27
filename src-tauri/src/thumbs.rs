//! Tải trước và thu nhỏ ảnh đại diện, lưu vào thư mục đệm trên máy.
//!
//! Nếu để giao diện tự tải ảnh từ máy chủ của báo mỗi lần vẽ lại, mỗi thẻ
//! tin là một lượt gọi mạng riêng và ảnh gốc thường lớn gấp nhiều lần khung
//! hiển thị. Tải sẵn ngay trong lượt làm mới — lúc người dùng vốn đã phải
//! chờ — rồi thu về đúng cỡ cần dùng thì thẻ tin hiện ảnh tức thì.

use futures::stream::{self, StreamExt};
use image::imageops::FilterType;
use std::collections::HashSet;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Bề ngang tối đa của ảnh đệm. Khung lớn nhất là thẻ dẫn 220px, nhân đôi
/// cho màn hình mật độ cao rồi làm tròn lên.
const THUMB_WIDTH: u32 = 480;
/// Bỏ qua ảnh gốc lớn bất thường để một tấm hỏng không làm nghẽn lượt tải.
const MAX_SOURCE_BYTES: usize = 8 * 1024 * 1024;
/// Số ảnh tải song song.
const CONCURRENCY: usize = 8;

pub fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Không xác định được thư mục cấu hình: {e}"))?
        .join("thumbs");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Không tạo được thư mục ảnh: {e}"))?;
    Ok(dir)
}

async fn download_and_shrink(
    client: &reqwest::Client,
    url: &str,
    destination: PathBuf,
) -> Option<String> {
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_SOURCE_BYTES {
        return None;
    }

    // Giải mã và thu nhỏ tốn CPU, đưa sang luồng chặn để không giữ vòng lặp async.
    let path = destination.clone();
    tokio::task::spawn_blocking(move || {
        let decoded = image::load_from_memory(&bytes).ok()?;
        let resized = if decoded.width() > THUMB_WIDTH {
            decoded.resize(THUMB_WIDTH, u32::MAX, FilterType::Triangle)
        } else {
            decoded
        };
        // Bỏ kênh trong suốt vì lưu ở dạng JPEG.
        resized.to_rgb8().save_with_format(&path, image::ImageFormat::Jpeg).ok()?;
        Some(path.to_string_lossy().to_string())
    })
    .await
    .ok()?
}

/// Tải ảnh cho những bài chưa có ảnh đệm. Trả về cặp (id bài, đường dẫn).
pub async fn ensure(
    client: &reqwest::Client,
    dir: &PathBuf,
    targets: Vec<(String, String)>,
) -> Vec<(String, String)> {
    stream::iter(targets)
        .map(|(id, url)| {
            let client = client.clone();
            let destination = dir.join(format!("{id}.jpg"));
            async move {
                // Đã có sẵn trên đĩa thì dùng lại, không tải lại.
                if destination.is_file() {
                    return Some((id, destination.to_string_lossy().to_string()));
                }
                download_and_shrink(&client, &url, destination)
                    .await
                    .map(|path| (id, path))
            }
        })
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Xoá ảnh đệm của những bài không còn trong kho tin.
pub fn prune(dir: &PathBuf, keep: &HashSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let still_needed = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| keep.contains(stem));
        if !still_needed {
            let _ = std::fs::remove_file(path);
        }
    }
}
