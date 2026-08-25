//! Tải nguồn tin: tự dò feed từ địa chỉ trang chủ, đọc feed, tải trang bài viết.

use crate::model::{stable_id, Article, CleanedArticle, Source};
use chrono::{SecondsFormat, Utc};
use scraper::{Html, Selector};
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "NewsFeed/0.1 (ung dung doc tin ca nhan)";
const COMMON_FEED_PATHS: &[&str] = &["/feed", "/rss", "/rss.xml", "/feed.xml", "/atom.xml", "/index.xml", "/feeds/posts/default"];

pub fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(20))
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
        return Ok(make_source(title, base.as_str(), base.as_str()));
    }

    let page = scan_html(&body, &base);
    let title = page.site_title.clone().unwrap_or_else(|| host_label(&base));

    // Trường hợp 2: trang khai báo feed trong thẻ <link rel="alternate">.
    if let Some(feed_url) = &page.declared_feed {
        return Ok(make_source(title, base.as_str(), feed_url.as_str()));
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
            return Ok(make_source(title, base.as_str(), candidate.as_str()));
        }
    }

    // Trường hợp 4: theo các liên kết RSS xuất hiện trong nội dung trang.
    for candidate in page.linked_feeds.iter().take(4) {
        if let Some(found) = try_feed(client, candidate).await {
            let title = page.site_title.clone().or(found).unwrap_or_else(|| host_label(&base));
            return Ok(make_source(title, base.as_str(), candidate.as_str()));
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

fn make_source(title: String, home_url: &str, feed_url: &str) -> Source {
    Source {
        id: stable_id(feed_url),
        title: title.chars().take(80).collect(),
        home_url: home_url.to_string(),
        feed_url: feed_url.to_string(),
        enabled: true,
        added_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        last_fetched: None,
        last_error: None,
        article_count: 0,
    }
}

/// Đọc feed của một nguồn và trả về danh sách bài.
pub async fn fetch_source(client: &reqwest::Client, source: &Source, limit: usize) -> Result<Vec<Article>, String> {
    let body = get_text(client, &source.feed_url).await?;
    let feed = feed_rs::parser::parse(body.as_bytes())
        .map_err(|e| format!("Không đọc được feed của {}: {e}", source.title))?;

    let mut out = Vec::new();
    for entry in feed.entries.into_iter().take(limit) {
        let Some(link) = entry.links.into_iter().map(|l| l.href).find(|h| h.starts_with("http")) else {
            continue;
        };
        let title = entry
            .title
            .map(|t| t.content.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        let summary_html = entry
            .summary
            .map(|s| s.content)
            .or_else(|| entry.content.and_then(|c| c.body))
            .unwrap_or_default();
        let summary = strip_tags(&summary_html);
        let published = entry
            .published
            .or(entry.updated)
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Secs, true);
        let image = entry
            .media
            .into_iter()
            .flat_map(|m| m.content)
            .find_map(|c| c.url.map(|u| u.to_string()));

        out.push(Article {
            id: stable_id(&link),
            source_id: source.id.clone(),
            source_title: source.title.clone(),
            title,
            url: link,
            summary: summary.chars().take(400).collect(),
            published,
            image,
        });
    }
    Ok(out)
}

/// Tải trang bài viết và trả về nội dung đã làm sạch.
pub async fn fetch_article(client: &reqwest::Client, article_url: &str) -> Result<CleanedArticle, String> {
    let base = Url::parse(article_url).map_err(|_| format!("Địa chỉ bài không hợp lệ: {article_url}"))?;
    let body = get_text(client, article_url).await?;
    Ok(crate::extract::extract(&body, &base))
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

            let articles = fetch_source(&client, &source, 5).await.expect("đọc được feed");
            assert!(!articles.is_empty(), "feed của {site} phải có bài");

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
