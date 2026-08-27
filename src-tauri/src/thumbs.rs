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
/// Chặn trên cho kích thước ảnh sau khi giải nén.
///
/// Một tệp PNG 137 KB có thể khai báo 12000x12000 pixel và bung ra 412 MB.
/// Không đặt trần thì một tấm ảnh như vậy đủ làm cạn bộ nhớ và giết cả ứng
/// dụng — ảnh lấy từ Internet nên phải coi là dữ liệu không tin được.
/// Chặn theo tổng số điểm ảnh, không chỉ theo từng chiều: một tấm 8000x8000
/// lọt qua mọi giới hạn chiều thông thường nhưng vẫn ngốn 183 MB khi chuyển
/// sang RGB. Ảnh minh hoạ của báo thực tế hiếm khi vượt 12 triệu điểm ảnh.
const MAX_PIXELS: u64 = 24_000_000;
const MAX_DIMENSION: u32 = 8_000;
const MAX_DECODE_BYTES: u64 = 96 * 1024 * 1024;
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

/// Giải mã ảnh trong giới hạn an toàn.
///
/// Trả về None thay vì panic hay ngốn hết bộ nhớ khi gặp tệp dị dạng hoặc
/// tệp cố tình khai báo kích thước khổng lồ.
fn decode_within_limits(bytes: &[u8]) -> Option<image::DynamicImage> {
    // Đọc kích thước từ phần đầu tệp trước, chưa cấp phát vùng điểm ảnh nào.
    let (width, height) = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?
        .into_dimensions()
        .ok()?;

    if width > MAX_DIMENSION || height > MAX_DIMENSION {
        return None;
    }
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return None;
    }

    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_DIMENSION);
    limits.max_image_height = Some(MAX_DIMENSION);
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    reader.limits(limits);
    reader.decode().ok()
}

fn shrink_to_file(bytes: &[u8], path: &std::path::Path) -> Option<()> {
    let decoded = decode_within_limits(bytes)?;
    let resized = if decoded.width() > THUMB_WIDTH {
        let height = (decoded.height() as u64 * THUMB_WIDTH as u64 / decoded.width().max(1) as u64)
            .clamp(1, MAX_DIMENSION as u64) as u32;
        decoded.resize_exact(THUMB_WIDTH, height, FilterType::Triangle)
    } else {
        decoded
    };
    // Bỏ kênh trong suốt vì lưu ở dạng JPEG.
    resized.to_rgb8().save_with_format(path, image::ImageFormat::Jpeg).ok()
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
        // Bộ giải mã ảnh có thể panic với tệp dị dạng. Chặn ngay tại đây để
        // một tấm ảnh hỏng không kéo đổ cả ứng dụng.
        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            shrink_to_file(&bytes, &path)
        }));
        match done {
            Ok(Some(())) => Some(path.to_string_lossy().to_string()),
            _ => None,
        }
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


#[cfg(test)]
mod tests {
    use super::*;

    /// Dựng một tệp PNG nhỏ nhưng khai báo kích thước khổng lồ.
    fn decompression_bomb(width: u32, height: u32) -> Vec<u8> {
        use std::io::Write;

        fn chunk(tag: &[u8], data: &[u8]) -> Vec<u8> {
            let mut out = Vec::new();
            out.extend_from_slice(&(data.len() as u32).to_be_bytes());
            let mut body = tag.to_vec();
            body.extend_from_slice(data);
            let crc = crc32(&body);
            out.extend_from_slice(&body);
            out.extend_from_slice(&crc.to_be_bytes());
            out
        }

        fn crc32(data: &[u8]) -> u32 {
            let mut table = [0u32; 256];
            for (i, entry) in table.iter_mut().enumerate() {
                let mut c = i as u32;
                for _ in 0..8 {
                    c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
                }
                *entry = c;
            }
            let mut c = 0xFFFF_FFFFu32;
            for byte in data {
                c = table[((c ^ *byte as u32) & 0xFF) as usize] ^ (c >> 8);
            }
            c ^ 0xFFFF_FFFF
        }

        let mut header = Vec::new();
        header.extend_from_slice(&width.to_be_bytes());
        header.extend_from_slice(&height.to_be_bytes());
        header.extend_from_slice(&[8, 0, 0, 0, 0]); // 8 bit, thang xám

        // Dữ liệu điểm ảnh toàn số 0 nén lại rất nhỏ.
        let raw = vec![0u8; (width as usize + 1) * height as usize];
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();

        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&chunk(b"IHDR", &header));
        png.extend_from_slice(&chunk(b"IDAT", &compressed));
        png.extend_from_slice(&chunk(b"IEND", b""));
        png
    }

    #[test]
    fn tu_choi_anh_khai_bao_kich_thuoc_khong_lo() {
        // Vượt giới hạn từng chiều.
        let huge = decompression_bomb(12_000, 12_000);
        assert!(huge.len() < 1024 * 1024, "tệp mồi phải nhỏ, thực tế {} byte", huge.len());
        assert!(decode_within_limits(&huge).is_none(), "phải từ chối ảnh quá khổ");

        // Đủ nhỏ ở mỗi chiều nhưng tổng số điểm ảnh vẫn quá lớn: 8000x8000 là
        // 64 triệu điểm, chuyển sang RGB mất 183 MB.
        let wide = decompression_bomb(8_000, 8_000);
        assert!(
            decode_within_limits(&wide).is_none(),
            "phải từ chối cả ảnh đủ nhỏ mỗi chiều nhưng tổng điểm ảnh quá lớn"
        );

        // Ảnh khổ thật của báo vẫn phải qua được.
        let realistic = decompression_bomb(3_000, 2_000);
        assert!(decode_within_limits(&realistic).is_some(), "ảnh khổ thường phải giải mã được");
    }

    /// Ghi lại vì sao phải đặt giới hạn: API không giới hạn của thư viện vẫn
    /// giải mã trọn tấm ảnh mồi. Bỏ qua ở lần chạy thường vì tốn hàng trăm MB.
    /// Chạy bằng: cargo test -- --ignored khong_gioi_han
    #[test]
    #[ignore]
    fn khong_gioi_han_thi_giai_ma_het_anh_moi() {
        let bomb = decompression_bomb(8_000, 8_000);
        println!("tệp mồi: {} KB", bomb.len() / 1024);
        let decoded = image::load_from_memory(&bomb).expect("API không giới hạn vẫn giải mã");
        println!(
            "giải mã ra {}x{} = {} MB dạng RGB",
            decoded.width(),
            decoded.height(),
            decoded.width() as u64 * decoded.height() as u64 * 3 / 1024 / 1024
        );
        assert_eq!(decoded.width(), 8_000);
        // Cùng tấm đó, đường đi có giới hạn thì từ chối.
        assert!(decode_within_limits(&bomb).is_none());
    }

    #[test]
    fn tu_choi_du_lieu_khong_phai_anh() {
        assert!(decode_within_limits(b"day khong phai anh").is_none());
        assert!(decode_within_limits(&[]).is_none());
        // Tệp PNG cụt giữa chừng.
        let mut truncated = decompression_bomb(20, 20);
        truncated.truncate(truncated.len() / 2);
        assert!(decode_within_limits(&truncated).is_none());
    }

    #[test]
    fn van_giai_ma_duoc_anh_binh_thuong() {
        let small = decompression_bomb(64, 48);
        let decoded = decode_within_limits(&small).expect("ảnh hợp lệ phải giải mã được");
        assert_eq!((decoded.width(), decoded.height()), (64, 48));
    }
}
