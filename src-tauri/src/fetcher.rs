//! Tải nguồn tin: tự dò feed từ địa chỉ trang chủ, đọc feed, tải trang bài viết.

use crate::model::{stable_id, Article, CleanedArticle, Source};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{SecondsFormat, Utc};
use scraper::{Html, Selector};
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "NewsFeed/0.1 (ung dung doc tin ca nhan)";
/// Logo lớn hơn mức này thì bỏ qua — huy hiệu nguồn chỉ hiển thị ở 20–28px.
const MAX_LOGO_BYTES: usize = 120_000;
const COMMON_FEED_PATHS: &[&str] = &["/feed", "/rss", "/rss.xml", "/feed.xml", "/atom.xml", "/index.xml", "/feeds/posts/default"];

pub fn client() -> Result<reqwest::Client, String> {
    // Không khai báo Accept thì có CDN trả thẳng 404 cho chính địa chỉ feed
    // mà trình duyệt mở được — Engadget là một trường hợp như vậy.
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "application/rss+xml, application/atom+xml, application/xml;q=0.9, text/html;q=0.8, */*;q=0.7",
        ),
    );

    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(headers)
        // Trần chờ cho cả lượt gọi, và một trần riêng ngắn hơn cho khâu bắt
        // tay: máy chủ chết thì nhả chỗ sớm cho nguồn khác thay vì giữ đủ 20s.
        .timeout(Duration::from_secs(20))
        .connect_timeout(Duration::from_secs(8))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| format!("Không khởi tạo được HTTP client: {e}"))
}

/// Trả về địa chỉ cuối cùng sau khi đi hết chuyển hướng, kèm nội dung.
/// Địa chỉ cuối mới là mốc đúng để ghép đường dẫn tương đối — nhiều báo
/// đổi tên chuyên mục và chuyển hướng địa chỉ cũ sang địa chỉ mới.
async fn get_page(client: &reqwest::Client, url: &str) -> Result<(Url, String), String> {
    let res = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Không kết nối được tới {url}: {e}"))?;
    if !res.status().is_success() {
        return Err(format!("{url} trả về mã {}", res.status().as_u16()));
    }
    let final_url = res.url().clone();
    let body = res
        .text()
        .await
        .map_err(|e| format!("Không đọc được nội dung từ {url}: {e}"))?;
    Ok((final_url, body))
}

async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    get_page(client, url).await.map(|(_, body)| body)
}

fn looks_like_feed(body: &str) -> bool {
    let head: String = body.chars().take(600).collect::<String>().to_lowercase();
    head.contains("<rss") || head.contains("<feed") || head.contains("<rdf:rdf")
}

/// Nhận vào địa chỉ bất kỳ (trang chủ, trang chuyên mục hoặc feed) và trả về
/// nguồn đã xác định được.
pub async fn discover(client: &reqwest::Client, input: &str) -> Result<Source, String> {
    let raw = input.trim();
    let normalized = if raw.starts_with("http://") || raw.starts_with("https://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    let entry = Url::parse(&normalized).map_err(|_| format!("Địa chỉ không hợp lệ: {input}"))?;

    let (base, body) = get_page(client, entry.as_str()).await?;

    // Trường hợp 1: người dùng dán thẳng địa chỉ feed.
    if looks_like_feed(&body) {
        let title = feed_title(&body).unwrap_or_else(|| host_label(&base));
        let mut source = make_source(title, base.as_str(), base.as_str());
        source.logo = fetch_logo(client, base.as_str()).await;
        return Ok(source);
    }

    let page = scan_html(&body, &base);
    let title = page.site_title.clone().unwrap_or_else(|| host_label(&base));

    // Trường hợp 2: trang khai báo feed trong thẻ <link rel="alternate">.
    if let Some(feed_url) = &page.declared_feed {
        let mut source = make_source(title, base.as_str(), feed_url.as_str());
        source.logo = fetch_logo(client, base.as_str()).await;
        return Ok(source);
    }

    // Trường hợp 3: đoán đường dẫn feed. Nhiều báo Việt Nam đặt feed theo
    // chuyên mục, ví dụ /khoa-hoc-cong-nghe → /rss/khoa-hoc-cong-nghe.rss
    let mut candidates: Vec<String> = Vec::new();
    if let Some(section) = section_slug(&base) {
        candidates.push(format!("/rss/{section}.rss"));
        candidates.push(format!("/{section}.rss"));
        candidates.push(format!("/{section}/feed"));
    }
    candidates.extend(COMMON_FEED_PATHS.iter().map(|p| (*p).to_string()));

    for path in candidates {
        let Ok(candidate) = base.join(&path) else { continue };
        if let Some(found) = try_feed(client, &candidate).await {
            let title = page.site_title.clone().or(found).unwrap_or_else(|| host_label(&base));
            let mut source = make_source(title, base.as_str(), candidate.as_str());
            source.logo = fetch_logo(client, base.as_str()).await;
            return Ok(source);
        }
    }

    // Trường hợp 4: theo các liên kết RSS xuất hiện trong nội dung trang.
    for candidate in page.linked_feeds.iter().take(4) {
        if let Some(found) = try_feed(client, candidate).await {
            let title = page.site_title.clone().or(found).unwrap_or_else(|| host_label(&base));
            let mut source = make_source(title, base.as_str(), candidate.as_str());
            source.logo = fetch_logo(client, base.as_str()).await;
            return Ok(source);
        }
    }

    Err(format!(
        "Không tìm thấy feed cho {}. Hãy thử dán trực tiếp địa chỉ RSS của trang.",
        host_label(&base)
    ))
}

/// Tải thử một địa chỉ; trả về Some(tên feed) nếu đúng là feed.
async fn try_feed(client: &reqwest::Client, url: &Url) -> Option<Option<String>> {
    let body = get_text(client, url.as_str()).await.ok()?;
    if looks_like_feed(&body) {
        Some(feed_title(&body))
    } else {
        None
    }
}

struct PageHints {
    site_title: Option<String>,
    declared_feed: Option<Url>,
    linked_feeds: Vec<Url>,
}

fn scan_html(body: &str, base: &Url) -> PageHints {
    let doc = Html::parse_document(body);

    let site_title = Selector::parse("title")
        .ok()
        .and_then(|sel| doc.select(&sel).next().map(|el| el.text().collect::<String>()))
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|t| !t.is_empty());

    let declared_feed = Selector::parse(
        r#"link[rel="alternate"][type="application/rss+xml"], link[rel="alternate"][type="application/atom+xml"], link[type="application/rss+xml"], link[type="application/atom+xml"]"#,
    )
    .ok()
    .and_then(|sel| {
        doc.select(&sel)
            .find_map(|el| el.value().attr("href"))
            .and_then(|href| base.join(href).ok())
    });

    let mut linked_feeds = Vec::new();
    if let Ok(sel) = Selector::parse("a[href]") {
        for el in doc.select(&sel) {
            let Some(href) = el.value().attr("href") else { continue };
            let lower = href.to_lowercase();
            if !(lower.ends_with(".rss") || lower.ends_with(".xml") || lower.contains("/rss") || lower.contains("/feed")) {
                continue;
            }
            if let Ok(url) = base.join(href) {
                if !linked_feeds.contains(&url) {
                    linked_feeds.push(url);
                }
            }
        }
    }

    PageHints { site_title, declared_feed, linked_feeds }
}

/// Đoạn cuối trong đường dẫn, dùng để đoán tên feed theo chuyên mục.
fn section_slug(url: &Url) -> Option<String> {
    url.path_segments()?
        .filter(|s| !s.is_empty())
        .next_back()
        .map(|s| s.trim_end_matches(".html").to_string())
        .filter(|s| !s.is_empty())
}

fn host_label(url: &Url) -> String {
    url.host_str().unwrap_or("Nguồn tin").trim_start_matches("www.").to_string()
}

fn feed_title(body: &str) -> Option<String> {
    let feed = feed_rs::parser::parse(body.as_bytes()).ok()?;
    feed.title.map(|t| t.content.trim().to_string()).filter(|t| !t.is_empty())
}

/// Rút tên thương hiệu từ thẻ <title>.
///
/// Thẻ title của báo thường là "Tên báo - khẩu hiệu dài" hoặc "khẩu hiệu dài
/// | Tên báo". Đoạn ngắn nhất gần như luôn là tên báo, bất kể nó nằm bên nào.
fn brand_name(raw: &str) -> String {
    let segments: Vec<&str> = raw
        .split(['|', '-', '–', '—', '·', '«', '»'])
        .map(str::trim)
        .filter(|part| part.chars().count() >= 3)
        .collect();

    let picked = segments
        .iter()
        .min_by_key(|part| part.chars().count())
        .copied()
        .unwrap_or(raw);

    picked.chars().take(60).collect()
}

fn make_source(title: String, home_url: &str, feed_url: &str) -> Source {
    Source {
        id: stable_id(feed_url),
        title: brand_name(&title),
        home_url: home_url.to_string(),
        feed_url: feed_url.to_string(),
        enabled: true,
        added_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        last_fetched: None,
        last_error: None,
        article_count: 0,
        logo: None,
        language: None,
    }
}

/// Tải logo của nguồn và trả về dạng data URI.
///
/// Ảnh được nhúng thẳng vào dữ liệu thay vì lưu địa chỉ, để huy hiệu nguồn
/// vẫn hiện khi không có mạng và không phải gọi ra ngoài mỗi lần vẽ lại.
pub async fn fetch_logo(client: &reqwest::Client, page_url: &str) -> Option<String> {
    let base = Url::parse(page_url).ok()?;

    let mut candidates: Vec<Url> = Vec::new();
    if let Ok((final_url, body)) = get_page(client, base.as_str()).await {
        candidates.extend(icon_links(&body, &final_url));
        if let Ok(root) = final_url.join("/favicon.ico") {
            candidates.push(root);
        }
    }
    if let Ok(root) = base.join("/favicon.ico") {
        if !candidates.contains(&root) {
            candidates.push(root);
        }
    }

    for candidate in candidates.into_iter().take(5) {
        if let Some(data_uri) = download_icon(client, &candidate).await {
            return Some(data_uri);
        }
    }
    None
}

/// Các thẻ <link> khai báo icon, xếp ảnh to lên trước vì nét hơn khi phóng.
fn icon_links(body: &str, base: &Url) -> Vec<Url> {
    let doc = Html::parse_document(body);
    let Ok(sel) = Selector::parse(r#"link[rel~="icon"], link[rel~="apple-touch-icon"], link[rel~="apple-touch-icon-precomposed"]"#)
    else {
        return Vec::new();
    };

    let mut found: Vec<(u32, Url)> = Vec::new();
    for el in doc.select(&sel) {
        let Some(href) = el.value().attr("href") else { continue };
        let Ok(url) = base.join(href) else { continue };
        let rel = el.value().attr("rel").unwrap_or_default().to_lowercase();
        // apple-touch-icon thường là PNG 180px, đẹp hơn favicon.ico 16px.
        let mut rank = if rel.contains("apple-touch-icon") { 400 } else { 0 };
        if let Some(sizes) = el.value().attr("sizes") {
            if let Some(px) = sizes.split(&['x', 'X'][..]).next().and_then(|v| v.trim().parse::<u32>().ok()) {
                rank = rank.max(px);
            }
        }
        if !found.iter().any(|(_, existing)| existing == &url) {
            found.push((rank, url));
        }
    }
    found.sort_by(|a, b| b.0.cmp(&a.0));
    found.into_iter().map(|(_, url)| url).collect()
}

async fn download_icon(client: &reqwest::Client, url: &Url) -> Option<String> {
    let res = client.get(url.as_str()).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    let mime = res
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.split(';').next().unwrap_or(v).trim().to_string())
        .unwrap_or_else(|| "image/x-icon".to_string());
    if !mime.starts_with("image/") {
        return None;
    }
    let bytes = res.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_LOGO_BYTES {
        return None;
    }
    Some(format!("data:{mime};base64,{}", BASE64.encode(&bytes)))
}

/// Đọc feed của một nguồn và trả về danh sách bài.
pub async fn fetch_source(client: &reqwest::Client, source: &Source, limit: usize) -> Result<Vec<Article>, String> {
    let body = get_text(client, &source.feed_url).await?;
    let feed = feed_rs::parser::parse(body.as_bytes())
        .map_err(|e| format!("Không đọc được feed của {}: {e}", source.title))?;

    let mut out = Vec::new();
    for entry in feed.entries.into_iter().take(limit) {
        let Some(link) = entry.links.iter().map(|l| l.href.clone()).find(|h| h.starts_with("http")) else {
            continue;
        };
        let Ok(article_base) = Url::parse(&link) else { continue };
        let title = entry
            .title
            .map(|t| decode_entities(&t.content).split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        let summary_html = entry
            .summary
            .map(|s| s.content)
            .or_else(|| entry.content.and_then(|c| c.body))
            .unwrap_or_default();
        let summary = decode_entities(&strip_tags(&summary_html));
        let published = entry
            .published
            .or(entry.updated)
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Secs, true);

        // Mỗi báo mang ảnh một kiểu: thẻ media chuẩn, enclosure, hoặc chỉ
        // nhét <img> vào phần mô tả. Thử lần lượt cả ba.
        let from_media = entry
            .media
            .iter()
            .flat_map(|m| m.content.iter())
            .find_map(|c| c.url.as_ref().map(|u| u.to_string()))
            .or_else(|| {
                entry
                    .media
                    .iter()
                    .flat_map(|m| m.thumbnails.iter())
                    .map(|t| t.image.uri.clone())
                    .next()
            });
        let from_enclosure = entry.links.iter().find_map(|l| {
            let is_image = l
                .media_type
                .as_deref()
                .is_some_and(|mime| mime.starts_with("image/"));
            (is_image && !looks_like_tracking_pixel(&l.href)).then(|| l.href.clone())
        });
        let image = from_media
            .or(from_enclosure)
            .or_else(|| image_from_html(&summary_html, &article_base));

        out.push(Article {
            id: stable_id(&link),
            source_id: source.id.clone(),
            source_title: source.title.clone(),
            title,
            url: link,
            summary: summary.chars().take(400).collect(),
            published,
            image,
            title_vi: None,
            summary_vi: None,
            thumb: None,
            image_checked: false,
            // Giữ lại bản HTML của feed: có nguồn dựng bài bằng JavaScript nên
            // trang gốc không chứa chữ, lúc đó đây là thứ duy nhất đọc được.
            content_html: Some(summary_html.chars().take(20_000).collect()),
        });
    }
    Ok(out)
}

/// Ảnh nhỏ hơn mức này gần như chắc chắn là điểm ảnh theo dõi, không phải ảnh bài.
const MIN_IMAGE_DIMENSION: u32 = 60;
/// Chỉ đọc phần đầu trang khi đi tìm og:image — thẻ meta luôn nằm trong <head>.
const OG_IMAGE_SCAN_BYTES: usize = 96_000;

fn looks_like_tracking_pixel(src: &str) -> bool {
    let lower = src.to_lowercase();
    ["1x1", "spacer", "blank.gif", "pixel", "beacon", "transparent"]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Ảnh đầu tiên trong một đoạn HTML, ví dụ phần mô tả của feed.
fn image_from_html(html: &str, base: &Url) -> Option<String> {
    let fragment = Html::parse_fragment(html);
    let sel = Selector::parse("img").ok()?;

    for el in fragment.select(&sel) {
        let value = el.value();

        // Bỏ qua ảnh khai báo kích thước quá nhỏ.
        let too_small = ["width", "height"].iter().any(|attr| {
            value
                .attr(attr)
                .and_then(|v| v.trim().parse::<u32>().ok())
                .is_some_and(|px| px < MIN_IMAGE_DIMENSION)
        });
        if too_small {
            continue;
        }

        for attr in ["src", "data-src", "data-original", "data-lazy-src"] {
            let Some(raw) = value.attr(attr) else { continue };
            if raw.trim().is_empty() || raw.starts_with("data:") || looks_like_tracking_pixel(raw) {
                continue;
            }
            if let Ok(url) = base.join(raw.trim()) {
                return Some(url.to_string());
            }
        }
    }
    None
}

/// Lấy og:image của một bài mà không tải cả trang.
///
/// Đọc theo từng khối và dừng ngay khi hết <head> hoặc vượt hạn mức, nên
/// việc bù ảnh cho hàng chục bài không kéo dài lượt làm mới.
pub async fn fetch_og_image(client: &reqwest::Client, article_url: &str) -> Option<String> {
    let base = Url::parse(article_url).ok()?;
    let mut res = client.get(article_url).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }

    let mut buffer: Vec<u8> = Vec::with_capacity(OG_IMAGE_SCAN_BYTES);
    while let Ok(Some(chunk)) = res.chunk().await {
        buffer.extend_from_slice(&chunk);
        let seen = String::from_utf8_lossy(&buffer);
        if seen.contains("</head>") || buffer.len() >= OG_IMAGE_SCAN_BYTES {
            break;
        }
    }

    let head = String::from_utf8_lossy(&buffer);
    let doc = Html::parse_document(&head);
    for selector in [
        r#"meta[property="og:image"]"#,
        r#"meta[name="og:image"]"#,
        r#"meta[name="twitter:image"]"#,
        r#"meta[property="twitter:image"]"#,
    ] {
        let Ok(sel) = Selector::parse(selector) else { continue };
        if let Some(raw) = doc.select(&sel).find_map(|el| el.value().attr("content")) {
            if raw.trim().is_empty() || raw.starts_with("data:") {
                continue;
            }
            if let Ok(url) = base.join(raw.trim()) {
                return Some(url.to_string());
            }
        }
    }
    None
}

/// Tải trang bài viết và trả về nội dung đã làm sạch.
/// Dưới ngưỡng này thì coi như chưa lấy được bài và đi tìm đường khác.
const THIN_ARTICLE_WORDS: usize = 120;

pub async fn fetch_article(client: &reqwest::Client, article_url: &str) -> Result<CleanedArticle, String> {
    let base = Url::parse(article_url).map_err(|_| format!("Địa chỉ bài không hợp lệ: {article_url}"))?;
    let body = get_text(client, article_url).await?;
    let cleaned = crate::extract::extract(&body, &base);
    if cleaned.word_count >= THIN_ARTICLE_WORDS {
        return Ok(cleaned);
    }

    // Bóc tách theo mật độ chữ không ra gì thì thử khối JSON trang nhúng sẵn.
    // Đây là các trang dựng nội dung bằng JavaScript: HTML gần như rỗng chữ
    // nhưng dữ liệu để dựng trang vẫn nằm ngay trong đó.
    let Some(embedded_html) = crate::embedded::article_body(&body) else {
        return Ok(cleaned);
    };
    let mut rich = crate::extract::from_body_html(&embedded_html, &base);
    if rich.word_count <= cleaned.word_count {
        return Ok(cleaned);
    }

    // Ảnh đại diện và tác giả vẫn lấy từ thẻ meta của trang, khối JSON không có.
    rich.lead_image = cleaned.lead_image;
    rich.byline = cleaned.byline;
    rich.removed_ads = cleaned.removed_ads;
    rich.removed_popups = cleaned.removed_popups;
    rich.removed_trackers = cleaned.removed_trackers;
    Ok(rich)
}

/// Giải mã thực thể HTML trong tiêu đề và mô tả của feed.
///
/// Nhiều feed đưa dấu nháy và gạch ngang dưới dạng &#8216; hay &mdash;.
/// Không giải mã thì tiêu đề hiện ra thô và bản dịch cũng lệch theo.
pub(crate) fn decode_entities(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }

    let named: &[(&str, &str)] = &[
        ("amp", "&"), ("lt", "<"), ("gt", ">"), ("quot", "\""), ("apos", "'"),
        ("nbsp", " "), ("hellip", "…"), ("mdash", "—"), ("ndash", "–"),
        ("lsquo", "\u{2018}"), ("rsquo", "\u{2019}"), ("ldquo", "\u{201C}"), ("rdquo", "\u{201D}"),
        ("laquo", "«"), ("raquo", "»"), ("times", "×"), ("middot", "·"), ("bull", "•"),
    ];

    let mut out = String::with_capacity(input.len());
    let mut rest = input;

    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];

        // Thực thể hợp lệ luôn kết thúc bằng ';' trong vòng vài ký tự.
        //
        // Cắt theo ranh giới ký tự chứ không theo byte. Chữ tiếng Việt chiếm
        // nhiều byte, nên mốc byte thứ 12 hay rơi vào giữa một chữ — và cắt
        // chuỗi giữa chừng một ký tự là chương trình dừng ngay tại đó.
        let window = tail.char_indices().nth(12).map_or(tail.len(), |(at, _)| at);
        let Some(end) = tail[..window].find(';') else {
            out.push('&');
            rest = &tail[1..];
            continue;
        };

        let body = &tail[1..end];
        let decoded = if let Some(digits) = body.strip_prefix("#x").or_else(|| body.strip_prefix("#X")) {
            u32::from_str_radix(digits, 16).ok().and_then(char::from_u32).map(String::from)
        } else if let Some(digits) = body.strip_prefix('#') {
            digits.parse::<u32>().ok().and_then(char::from_u32).map(String::from)
        } else {
            named.iter().find(|(name, _)| *name == body).map(|(_, value)| (*value).to_string())
        };

        match decoded {
            Some(text) => {
                out.push_str(&text);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn strip_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut inside = false;
    for ch in input.chars() {
        match ch {
            '<' => inside = true,
            '>' => inside = false,
            c if !inside => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Đo thời gian đọc từng nguồn mặc định trên đường truyền thật.
    ///
    /// Dùng khi tin về chậm hoặc không về: nó chỉ ra nguồn nào ngốn thời gian
    /// và nguồn nào chạm trần chờ, thay vì phải ngồi đoán.
    /// Chạy bằng: cargo test -- --ignored --nocapture do_thoi_gian_doc_nguon
    #[test]
    #[ignore]
    fn do_thoi_gian_doc_nguon_mac_dinh() {
        use futures::StreamExt;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = client().unwrap();
        let sources = crate::store::default_sources();

        let started = std::time::Instant::now();
        let mut rows: Vec<(u128, String, String)> = runtime.block_on(async {
            futures::stream::iter(sources)
                .map(|(title, home, feed)| {
                    let client = client.clone();
                    async move {
                        let source = make_source(title.to_string(), home, feed);
                        let at = std::time::Instant::now();
                        let outcome = fetch_source(&client, &source, 25).await;
                        let note = match outcome {
                            Ok(articles) => format!("{} bài", articles.len()),
                            Err(message) => message.chars().take(70).collect(),
                        };
                        (at.elapsed().as_millis(), title.to_string(), note)
                    }
                })
                .buffer_unordered(crate::FETCH_CONCURRENCY)
                .collect()
                .await
        });

        rows.sort_by(|a, b| b.0.cmp(&a.0));
        for (ms, title, note) in &rows {
            println!("{ms:>6} ms  {title:<24} {note}");
        }
        println!("---- tổng cả lượt: {} ms", started.elapsed().as_millis());
    }

    #[test]
    fn giai_ma_thuc_the_html() {
        assert_eq!(
            decode_entities("Rockstar responds to &#8216;heartbreaking&#8217; GTA 6 leaks"),
            "Rockstar responds to \u{2018}heartbreaking\u{2019} GTA 6 leaks"
        );
        assert_eq!(decode_entities("Q&amp;A v&#x1EDB;i CEO"), "Q&A với CEO");
        // Sau dấu & là chữ tiếng Việt nhiều byte: mốc cắt phải theo ký tự,
        // nếu tính theo byte thì rơi vào giữa chữ "ủ" và chương trình dừng.
        assert_eq!(
            decode_entities("&amp; Max của LG học thói quen sử dụng"),
            "& Max của LG học thói quen sử dụng"
        );
        // Dấu & đứng một mình ngay trước chữ có dấu cũng phải qua được.
        assert_eq!(decode_entities("Điều hoà & tủ lạnh thông minh"), "Điều hoà & tủ lạnh thông minh");
        assert_eq!(decode_entities("Giá 25 tri&#7879;u &mdash; r&#7867;"), "Giá 25 triệu — rẻ");
        // Chuỗi không phải thực thể thì giữ nguyên, không được nuốt mất.
        assert_eq!(decode_entities("Tom & Jerry"), "Tom & Jerry");
        assert_eq!(decode_entities("a &notarealentity; b"), "a &notarealentity; b");
        assert_eq!(decode_entities("không có gì"), "không có gì");
    }

    #[test]
    fn rut_gon_ten_bao_tu_the_title() {
        assert_eq!(brand_name("Tinhte.vn - MXH Hỏi đáp, Review, Thông tin công nghệ"), "Tinhte.vn");
        assert_eq!(
            brand_name("Ars Technica - Serving the Technologist since 1998. News, reviews, and analysis."),
            "Ars Technica"
        );
        assert_eq!(brand_name("Trang thông tin dành cho tín đồ công nghệ | GenK.vn"), "GenK.vn");
        assert_eq!(brand_name("Tổng hợp tin tức Khoa học công nghệ mới nhất | VnExpress"), "VnExpress");
        // Tên vốn đã gọn thì giữ nguyên.
        assert_eq!(brand_name("TechCrunch"), "TechCrunch");
    }

    /// Ảnh trong bài phải tải được thật.
    ///
    /// tinhte.vn đặt src là địa chỉ kèm token hết hạn, dùng nó thì ảnh hỏng
    /// hết mà không có lỗi nào báo ra. Chạm mạng thật nên bị bỏ qua thường lệ.
    #[tokio::test]
    #[ignore]
    async fn anh_trong_bai_tai_duoc_that() {
        let client = client().expect("tạo client");
        let cleaned = fetch_article(
            &client,
            "https://tinhte.vn/thread/review-macbook-pro-m3-max-16-inch-con-quai-vat-minh-dung-de-cay-viec-nang-suot-mot-nam-qua.4152062/",
        )
        .await
        .expect("đọc được bài");

        assert!(!cleaned.images.is_empty(), "bài này phải có ảnh");
        let mut ok = 0usize;
        for src in &cleaned.images {
            let good = match client.get(src).send().await {
                Ok(res) => {
                    let status = res.status().as_u16();
                    let bytes = res.bytes().await.map(|b| b.len()).unwrap_or(0);
                    println!("  {status} {bytes:>8} byte  {}", &src[..src.len().min(78)]);
                    status == 200 && bytes > 1000
                }
                Err(e) => {
                    println!("  lỗi {e}");
                    false
                }
            };
            if good {
                ok += 1;
            }
        }
        println!("  ảnh tải được: {ok}/{}", cleaned.images.len());
        assert_eq!(ok, cleaned.images.len(), "mọi ảnh trong bài đều phải tải được");
    }

    /// Trang dựng nội dung bằng JavaScript phải ra toàn văn, không phải tóm tắt.
    ///
    /// Chạm mạng thật nên bị bỏ qua ở lần chạy thường.
    /// Chạy bằng: `cargo test -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn lay_duoc_toan_van_tu_trang_dung_bang_javascript() {
        let client = client().expect("tạo client");
        let source = crate::model::Source {
            id: "tinhte".into(),
            title: "Tinhte.vn".into(),
            home_url: "https://tinhte.vn".into(),
            feed_url: "https://tinhte.vn/rss".into(),
            enabled: true,
            added_at: String::new(),
            last_fetched: None,
            last_error: None,
            article_count: 0,
            logo: None,
            language: None,
        };

        let articles = fetch_source(&client, &source, 6).await.expect("đọc được feed");
        assert!(!articles.is_empty(), "feed tinhte.vn phải có bài");

        let mut full = 0usize;
        for article in &articles {
            let words_in_feed = article
                .content_html
                .as_deref()
                .map(|h| crate::extract::from_fragment(h, &Url::parse(&article.url).unwrap()).word_count)
                .unwrap_or(0);
            let cleaned = fetch_article(&client, &article.url).await.expect("đọc được bài");
            println!(
                "  {:5} từ (feed cho {:4}) {:2} ảnh  {}",
                cleaned.word_count,
                words_in_feed,
                cleaned.images.len(),
                article.title.chars().take(52).collect::<String>()
            );
            if !cleaned.partial && cleaned.word_count > words_in_feed {
                full += 1;
            }
        }

        println!("  lấy được toàn văn: {full}/{}", articles.len());
        assert!(
            full * 2 > articles.len(),
            "quá nửa số bài phải ra toàn văn, hiện chỉ {full}/{}",
            articles.len()
        );
    }

    /// Kiểm thử chạm mạng thật, nên bị bỏ qua ở lần chạy thường.
    /// Chạy bằng: `cargo test -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn duong_ong_that_tu_trang_chu_den_bai_da_boc_tach() {
        let client = client().expect("tạo client");

        // Dò feed từ địa chỉ trang chủ, không phải địa chỉ RSS.
        for site in ["vnexpress.net/so-hoa", "genk.vn", "arstechnica.com"] {
            let source = discover(&client, site).await.expect("dò được feed");
            println!("\n{site} → {} ({})", source.title, source.feed_url);

            let articles = fetch_source(&client, &source, 10).await.expect("đọc được feed");
            assert!(!articles.is_empty(), "feed của {site} phải có bài");

            let from_feed = articles.iter().filter(|a| a.image.is_some()).count();
            // Bài nào feed không kèm ảnh thì bù bằng og:image của trang bài.
            let mut with_image = from_feed;
            for article in articles.iter().filter(|a| a.image.is_none()) {
                if fetch_og_image(&client, &article.url).await.is_some() {
                    with_image += 1;
                }
            }
            println!(
                "  ảnh: {}/{} lấy thẳng từ feed, {}/{} sau khi bù og:image",
                from_feed,
                articles.len(),
                with_image,
                articles.len()
            );
            assert!(
                with_image * 2 >= articles.len(),
                "{site}: quá nửa số bài phải có ảnh, hiện chỉ {with_image}/{}",
                articles.len()
            );

            let cleaned = fetch_article(&client, &articles[0].url).await.expect("bóc tách được bài");
            println!(
                "  {} từ · {} ảnh · loại bỏ {} quảng cáo / {} popup / {} script",
                cleaned.word_count,
                cleaned.images.len(),
                cleaned.removed_ads,
                cleaned.removed_popups,
                cleaned.removed_trackers
            );
            println!("  tiêu đề: {}", articles[0].title);
            assert!(cleaned.word_count > 80, "{site}: phải bóc được thân bài");
        }
    }
}
