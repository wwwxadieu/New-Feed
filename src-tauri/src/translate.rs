//! Dịch sang tiếng Việt bằng MyMemory.
//!
//! Không dùng mô hình ngôn ngữ lớn và không cần khoá API. MyMemory là dịch
//! vụ dịch máy có tài liệu công khai, dùng miễn phí ở mức 5.000 ký tự mỗi
//! ngày cho mỗi địa chỉ IP, hoặc 50.000 nếu khai báo một địa chỉ email.

use serde::Deserialize;

/// MyMemory giới hạn độ dài mỗi lần gọi, nên đoạn dài phải cắt nhỏ.
const MAX_CHUNK_CHARS: usize = 450;

#[derive(Debug)]
pub enum TranslateError {
    /// Hết hạn mức miễn phí trong ngày.
    QuotaExhausted,
    /// Lỗi mạng hoặc dịch vụ trả về không hợp lệ.
    Unavailable(String),
}

impl std::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::QuotaExhausted => write!(
                f,
                "Đã dùng hết hạn mức dịch miễn phí trong ngày. Khai báo email trong phần Nguồn tin để nâng hạn mức, hoặc thử lại vào ngày mai."
            ),
            Self::Unavailable(reason) => write!(f, "Không dịch được: {reason}"),
        }
    }
}

#[derive(Deserialize)]
struct MyMemoryResponse {
    #[serde(rename = "responseData")]
    response_data: ResponseData,
    #[serde(rename = "quotaFinished", default)]
    quota_finished: bool,
}

#[derive(Deserialize)]
struct ResponseData {
    #[serde(rename = "translatedText")]
    translated_text: String,
}

/// Cắt văn bản thành các đoạn vừa hạn mức, ưu tiên cắt ở ranh giới câu để
/// bản dịch không bị đứt giữa mệnh đề.
fn split_into_chunks(text: &str) -> Vec<String> {
    if text.chars().count() <= MAX_CHUNK_CHARS {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut current = String::new();

    for sentence in text.split_inclusive(['.', '!', '?', ';']) {
        if current.chars().count() + sentence.chars().count() > MAX_CHUNK_CHARS && !current.is_empty() {
            chunks.push(std::mem::take(&mut current));
        }
        // Một câu dài hơn cả hạn mức thì đành cắt cứng theo ký tự.
        if sentence.chars().count() > MAX_CHUNK_CHARS {
            let mut buffer = String::new();
            for ch in sentence.chars() {
                buffer.push(ch);
                if buffer.chars().count() >= MAX_CHUNK_CHARS {
                    chunks.push(std::mem::take(&mut buffer));
                }
            }
            current.push_str(&buffer);
        } else {
            current.push_str(sentence);
        }
    }

    if !current.trim().is_empty() {
        chunks.push(current);
    }
    chunks
}

async fn translate_chunk(
    client: &reqwest::Client,
    chunk: &str,
    email: Option<&str>,
) -> Result<String, TranslateError> {
    let mut query: Vec<(&str, &str)> = vec![("q", chunk), ("langpair", "Autodetect|vi")];
    if let Some(address) = email.filter(|value| value.contains('@')) {
        query.push(("de", address));
    }

    let res = client
        .get("https://api.mymemory.translated.net/get")
        .query(&query)
        .send()
        .await
        .map_err(|e| TranslateError::Unavailable(e.to_string()))?;

    let body = res
        .text()
        .await
        .map_err(|e| TranslateError::Unavailable(e.to_string()))?;

    let parsed: MyMemoryResponse =
        serde_json::from_str(&body).map_err(|e| TranslateError::Unavailable(e.to_string()))?;

    // Khi hết hạn mức, dịch vụ vẫn trả mã 200 nhưng nhét lời cảnh báo vào
    // đúng ô chứa bản dịch, nên phải bắt ở đây thay vì dựa vào mã HTTP.
    let text = parsed.response_data.translated_text;
    if parsed.quota_finished || text.starts_with("MYMEMORY WARNING") || text.contains("YOU USED ALL AVAILABLE FREE TRANSLATIONS") {
        return Err(TranslateError::QuotaExhausted);
    }
    if text.trim().is_empty() {
        return Err(TranslateError::Unavailable("dịch vụ trả về chuỗi rỗng".into()));
    }
    Ok(text)
}

/// Dịch vụ hay viết hoa toàn bộ đơn vị đo khi gặp "12M" hay "$1.5B".
/// Trả chúng về chữ thường cho khớp với phần còn lại của câu.
fn normalise_casing(text: &str) -> String {
    let mut out = text.to_string();
    for (upper, lower) in [
        (" TRIỆU", " triệu"),
        (" NGHÌN", " nghìn"),
        (" TỶ", " tỷ"),
        (" TỈ", " tỉ"),
    ] {
        out = out.replace(upper, lower);
    }
    out
}

/// Dịch một đoạn văn bản sang tiếng Việt.
pub async fn to_vietnamese(
    client: &reqwest::Client,
    text: &str,
    email: Option<&str>,
) -> Result<String, TranslateError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let mut out = String::with_capacity(trimmed.len() + 32);
    for chunk in split_into_chunks(trimmed) {
        let piece = translate_chunk(client, &chunk, email).await?;
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(piece.trim());
    }
    Ok(normalise_casing(&out))
}

/// Dấu riêng của chữ quốc ngữ. Chữ tiếng Anh không có những ký tự này.
const VIETNAMESE_MARKERS: &str = "ăâđêôơưàáảãạằắẳẵặầấẩẫậèéẻẽẹềếểễệìíỉĩịòóỏõọồốổỗộờớởỡợùúủũụừứửữựỳýỷỹỵ";

/// Đoán xem một nguồn có phải tiếng Việt không, dựa trên tiêu đề của nó.
///
/// Chỉ cần phân biệt "tiếng Việt" với "không phải tiếng Việt", vì đích dịch
/// luôn là tiếng Việt và MyMemory tự nhận ngôn ngữ nguồn.
pub fn is_vietnamese(samples: &[String]) -> bool {
    let considered: Vec<&String> = samples.iter().filter(|s| s.chars().count() >= 12).collect();
    if considered.is_empty() {
        return false;
    }
    let hits = considered
        .iter()
        .filter(|s| s.to_lowercase().chars().any(|c| VIETNAMESE_MARKERS.contains(c)))
        .count();
    hits * 10 >= considered.len() * 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nhan_dien_tieng_viet() {
        let vi = vec![
            "Apple ra mắt Mac mini mới với chip M6".to_string(),
            "Lỗ hổng bảo mật nghiêm trọng trong thư viện nén".to_string(),
            "Giá bán dự kiến từ 24,9 triệu đồng".to_string(),
        ];
        assert!(is_vietnamese(&vi));

        let en = vec![
            "Apple releases new Mac mini with M6 chip".to_string(),
            "Critical vulnerability found in compression library".to_string(),
            "IBM's new Granite models ride the wave of local LLMs".to_string(),
        ];
        assert!(!is_vietnamese(&en));
    }

    #[test]
    fn khong_doan_bua_khi_khong_du_du_lieu() {
        assert!(!is_vietnamese(&[]));
        assert!(!is_vietnamese(&["Ngắn".to_string()]));
    }

    #[test]
    fn chuan_hoa_chu_hoa_don_vi() {
        assert_eq!(
            normalise_casing("Gọi vốn 12 TRIỆU $ từ quỹ đầu tư"),
            "Gọi vốn 12 triệu $ từ quỹ đầu tư"
        );
        assert_eq!(normalise_casing("Định giá 1,5 TỶ USD"), "Định giá 1,5 tỷ USD");
        // Không đụng tới chữ vốn đã đúng.
        assert_eq!(normalise_casing("doanh thu 3 triệu USD"), "doanh thu 3 triệu USD");
    }

    #[test]
    fn cat_doan_dai_theo_ranh_gioi_cau() {
        let short = "Một câu ngắn.";
        assert_eq!(split_into_chunks(short), vec![short.to_string()]);

        let long = "Câu thứ nhất khá dài. ".repeat(60);
        let chunks = split_into_chunks(&long);
        assert!(chunks.len() > 1, "đoạn dài phải được cắt nhỏ");
        assert!(
            chunks.iter().all(|c| c.chars().count() <= MAX_CHUNK_CHARS + 1),
            "mỗi đoạn phải nằm trong hạn mức"
        );
        // Ghép lại không được mất chữ.
        assert_eq!(chunks.concat().chars().count(), long.chars().count());
    }
}
