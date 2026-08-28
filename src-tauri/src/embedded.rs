//! Lấy thân bài từ khối JSON mà trang nhúng sẵn trong HTML.
//!
//! Có những trang dựng nội dung bằng JavaScript: HTML trả về gần như không có
//! thẻ văn bản nào nên bộ bóc tách theo mật độ chữ không tìm được gì. Nhưng
//! khung Next.js vẫn phải gửi kèm dữ liệu để dựng trang, đặt trong thẻ
//! <script id="__NEXT_DATA__">. Thân bài nằm nguyên trong đó, lấy được bằng
//! một lượt tải bình thường mà không cần trình duyệt không giao diện.

use serde_json::Value;

/// Khoá có thể chứa tiêu đề bài trong JSON.
const TITLE_KEYS: &[&str] = &["title", "thread_title", "headline", "post_title", "name"];

/// Khoá có thể chứa thân bài. Ưu tiên tên có hậu tố html vì bản đó giữ được
/// ảnh và tiêu đề phụ, còn bản thuần chữ đã mất hết cấu trúc.
const BODY_KEYS: &[&str] = &[
    "post_body_html", "body_html", "content_html", "articlebody", "article_body",
    "content", "body", "html",
];

/// Trần cho khối JSON. Trang thật nặng khoảng 1 MB; quá mức này thì gần như
/// chắc chắn không phải trang bài và không đáng bỏ công phân tích.
const MAX_JSON_BYTES: usize = 8 * 1024 * 1024;

/// Trần độ sâu khi duyệt. serde_json vốn đã chặn ở 128 mức lúc phân tích,
/// đây là lớp chặn thứ hai: dữ liệu lấy từ Internet mà đệ quy không đáy thì
/// tràn ngăn xếp, và tràn ngăn xếp thì catch_unwind không đỡ được.
const MAX_DEPTH: usize = 64;

/// Số ký tự tối thiểu để hai tiêu đề được coi là đủ đặc trưng mà đem so.
const MIN_TITLE_CHARS: usize = 20;

/// Tìm thân bài trong khối JSON nhúng, neo theo tiêu đề của chính trang đó.
///
/// Neo theo tiêu đề là phần quan trọng nhất. Khối JSON của một trang bài còn
/// chứa bài liên quan, tin nổi bật và sự kiện, và những bài đó thường dài hơn
/// bài đang đọc — đo trên tinhte.vn thì chuỗi HTML dài nhất trong JSON là một
/// bài sự kiện 58 KB chẳng liên quan gì. Nên không được chọn theo độ dài, mà
/// phải tìm đúng nút mang tiêu đề của bài rồi mới lấy thân bài trong nút đó.
pub fn article_body(html: &str) -> Option<String> {
    let page_title = page_title(html)?;
    let raw = next_data(html)?;
    if raw.len() > MAX_JSON_BYTES {
        return None;
    }
    let value: Value = serde_json::from_str(raw).ok()?;

    let mut best: Option<String> = None;
    collect(&value, &page_title, 0, &mut best);
    best
}

/// Duyệt tìm nút có tiêu đề khớp, rồi lấy thân bài dài nhất bên trong nút đó.
fn collect(node: &Value, page_title: &str, depth: usize, best: &mut Option<String>) {
    if depth > MAX_DEPTH {
        return;
    }
    match node {
        Value::Object(map) => {
            let titled = TITLE_KEYS.iter().any(|k| {
                map.get(*k)
                    .and_then(Value::as_str)
                    .is_some_and(|t| titles_match(&normalise(t), page_title))
            });
            if titled {
                if let Some(body) = longest_body(node, 0) {
                    if body.len() > best.as_ref().map(String::len).unwrap_or(0) {
                        *best = Some(body);
                    }
                }
            }
            for v in map.values() {
                collect(v, page_title, depth + 1, best);
            }
        }
        Value::Array(items) => {
            for v in items {
                collect(v, page_title, depth + 1, best);
            }
        }
        _ => {}
    }
}

/// Chuỗi HTML dài nhất nằm dưới một khoá thân bài, trong cả cây con.
fn longest_body(node: &Value, depth: usize) -> Option<String> {
    if depth > MAX_DEPTH {
        return None;
    }
    let mut best: Option<&str> = None;

    fn walk<'a>(node: &'a Value, depth: usize, best: &mut Option<&'a str>) {
        if depth > MAX_DEPTH {
            return;
        }
        match node {
            Value::Object(map) => {
                for (k, v) in map {
                    if let Some(text) = v.as_str() {
                        let key = k.to_ascii_lowercase();
                        // Phải có thẻ, nếu không thì đây là bản thuần chữ.
                        if BODY_KEYS.contains(&key.as_str())
                            && text.contains('<')
                            && text.len() > best.map(str::len).unwrap_or(0)
                        {
                            *best = Some(text);
                        }
                    } else {
                        walk(v, depth + 1, best);
                    }
                }
            }
            Value::Array(items) => {
                for v in items {
                    walk(v, depth + 1, best);
                }
            }
            _ => {}
        }
    }

    walk(node, depth, &mut best);
    best.map(str::to_string)
}

/// Tiêu đề của trang, đã chuẩn hoá để đem so.
fn page_title(html: &str) -> Option<String> {
    let from_meta = meta_property(html, "og:title");
    let title = from_meta.or_else(|| tag_text(html, "title"))?;
    let normalised = normalise(&title);
    (normalised.chars().count() >= MIN_TITLE_CHARS).then_some(normalised)
}

/// Hai tiêu đề coi là cùng một bài khi cái ngắn là phần đầu của cái dài.
///
/// Không so bằng nhau được: og:title của tinhte.vn có thêm đuôi tác giả
/// ("... | Viết bởi ittus") mà tiêu đề trong JSON thì không.
fn titles_match(a: &str, b: &str) -> bool {
    if a.chars().count() < MIN_TITLE_CHARS || b.chars().count() < MIN_TITLE_CHARS {
        return false;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    long.starts_with(short)
}

fn normalise(input: &str) -> String {
    crate::fetcher::decode_entities(input)
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Nội dung thẻ <script id="__NEXT_DATA__" ...>.
fn next_data(html: &str) -> Option<&str> {
    let anchor = html.find("__NEXT_DATA__")?;
    let open_end = anchor + html[anchor..].find('>')?;
    let start = open_end + 1;
    let end = start + html[start..].find("</script>")?;
    Some(html[start..end].trim())
}

fn meta_property(html: &str, property: &str) -> Option<String> {
    let needle = format!("property=\"{property}\"");
    let at = html.find(&needle)?;
    // Thẻ meta có thể đặt content trước hoặc sau property.
    let open = html[..at].rfind('<')?;
    let close = at + html[at..].find('>')?;
    let tag = &html[open..close];
    let idx = tag.find("content=\"")? + "content=\"".len();
    let rest = &tag[idx..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn tag_text(html: &str, tag: &str) -> Option<String> {
    let open = html.find(&format!("<{tag}"))?;
    let start = open + html[open..].find('>')? + 1;
    let end = start + html[start..].find(&format!("</{tag}>"))?;
    Some(html[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(next_data: &str, title: &str) -> String {
        format!(
            r#"<html><head><meta property="og:title" content="{title}"/>
            <script id="__NEXT_DATA__" type="application/json">{next_data}</script>
            </head><body></body></html>"#
        )
    }

    /// Bài đang đọc phải thắng, kể cả khi bài liên quan trong cùng khối JSON
    /// dài hơn hẳn. Đây đúng là tình huống gặp trên tinhte.vn.
    #[test]
    fn khong_lay_nham_bai_lien_quan_dai_hon() {
        let json = r#"{
            "props": {
                "thread": {
                    "thread_title": "Microsoft số hoá đĩa game Xbox Series X/S",
                    "posts": [{"post_body_html": "<span>Thân bài đúng của bài Xbox.</span>"}]
                },
                "related": [{
                    "title": "Một bài hoàn toàn khác nhưng dài hơn nhiều lần",
                    "first_post": {"post_body_html": "<span>AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA</span>"}
                }]
            }
        }"#;
        let html = page(json, "Microsoft số hoá đĩa game Xbox Series X/S | Viết bởi ai đó");
        let body = article_body(&html).expect("phải tìm được thân bài");
        assert!(body.contains("Thân bài đúng"), "lấy nhầm bài: {body}");
    }

    /// Không khớp được tiêu đề thì thà không trả gì còn hơn đoán bừa.
    #[test]
    fn khong_khop_tieu_de_thi_tra_ve_none() {
        let json = r#"{"props": {"thread": {"title": "Một tiêu đề chẳng liên quan gì cả",
            "body_html": "<p>nội dung</p>"}}}"#;
        let html = page(json, "Tiêu đề của trang thì hoàn toàn khác hẳn");
        assert!(article_body(&html).is_none());
    }

    #[test]
    fn khong_co_next_data_thi_tra_ve_none() {
        let html = r#"<html><head><title>Một tiêu đề bài viết đủ dài để xét</title></head>
            <body><p>nội dung thường</p></body></html>"#;
        assert!(article_body(html).is_none());
    }

    /// Bản thuần chữ không được thắng bản có thẻ, vì bản kia giữ được ảnh.
    #[test]
    fn uu_tien_ban_co_the_hon_ban_thuan_chu() {
        let json = r#"{"props": {"thread": {
            "thread_title": "Tiêu đề bài viết dùng để kiểm thử",
            "post_body_plain_text": "chữ không thẻ dài hơn rất nhiều lần so với bản kia",
            "post_body_html": "<p>ngắn</p>"}}}"#;
        let html = page(json, "Tiêu đề bài viết dùng để kiểm thử");
        assert_eq!(article_body(&html).as_deref(), Some("<p>ngắn</p>"));
    }
}
