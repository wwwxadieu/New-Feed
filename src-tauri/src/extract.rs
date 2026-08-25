//! Bóc tách nội dung bài viết: loại quảng cáo/popup/script theo dõi, giữ lại chữ và ảnh.

use crate::model::{Block, CleanedArticle};
use scraper::{ElementRef, Html, Selector};
use url::Url;

/// Lớp/id thường gặp của khối quảng cáo.
const AD_PATTERNS: &[&str] = &[
    "advert", "adsbygoogle", "googlead", "doubleclick", "taboola", "outbrain",
    "sponsor", "promo", "banner", "quang-cao", "quangcao", "qc-box",
    "ad-slot", "ad-unit", "ad-container", "ad-wrapper", "ads-", "-ads", "adbox",
];

/// Lớp/id của popup, tường thu phí và mời đăng ký.
const POPUP_PATTERNS: &[&str] = &[
    "popup", "modal", "overlay", "interstitial", "newsletter", "subscribe",
    "paywall", "cookie", "gdpr", "consent", "notification-bar", "app-download",
];

/// Khối không thuộc nội dung bài.
const CHROME_PATTERNS: &[&str] = &[
    "related", "recommend", "trending", "most-read", "share", "social",
    "comment", "breadcrumb", "sidebar", "widget", "tag-list", "author-box",
];

const TRACKER_TAGS: &[&str] = &["script", "noscript", "iframe", "embed", "object"];
const CHROME_TAGS: &[&str] = &["style", "nav", "aside", "footer", "form", "button", "svg", "figcaption"];

fn absolutize(base: &Url, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() || href.starts_with("data:") {
        return None;
    }
    base.join(href).ok().map(|u| u.to_string())
}

fn meta_content(doc: &Html, selector: &str) -> Option<String> {
    let sel = Selector::parse(selector).ok()?;
    doc.select(&sel)
        .find_map(|el| el.value().attr("content"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Lấy nguồn ảnh thật, có tính tới lazy-load và srcset.
fn image_src(el: &scraper::ElementRef, base: &Url) -> Option<String> {
    let v = el.value();
    for attr in ["src", "data-src", "data-original", "data-lazy-src", "data-echo"] {
        if let Some(raw) = v.attr(attr) {
            if let Some(abs) = absolutize(base, raw) {
                return Some(abs);
            }
        }
    }
    if let Some(srcset) = v.attr("srcset").or_else(|| v.attr("data-srcset")) {
        // Lấy ứng viên cuối cùng, thường là bản độ phân giải cao nhất.
        if let Some(last) = srcset.split(',').next_back() {
            if let Some(candidate) = last.trim().split_whitespace().next() {
                return absolutize(base, candidate);
            }
        }
    }
    None
}

pub fn extract(html_src: &str, base: &Url) -> CleanedArticle {
    let mut doc = Html::parse_document(html_src);

    let lead_image = meta_content(&doc, r#"meta[property="og:image"]"#)
        .or_else(|| meta_content(&doc, r#"meta[name="twitter:image"]"#))
        .and_then(|raw| absolutize(base, &raw));
    let byline = meta_content(&doc, r#"meta[name="author"]"#)
        .or_else(|| meta_content(&doc, r#"meta[property="article:author"]"#));

    let mut removed_ads = 0usize;
    let mut removed_popups = 0usize;
    let mut removed_trackers = 0usize;
    let mut doomed = Vec::new();

    for tag in TRACKER_TAGS {
        if let Ok(sel) = Selector::parse(tag) {
            for el in doc.select(&sel) {
                doomed.push(el.id());
                removed_trackers += 1;
            }
        }
    }
    for tag in CHROME_TAGS {
        if let Ok(sel) = Selector::parse(tag) {
            for el in doc.select(&sel) {
                doomed.push(el.id());
            }
        }
    }

    if let Ok(all) = Selector::parse("*") {
        for el in doc.select(&all) {
            let value = el.value();
            let class = value.attr("class").unwrap_or_default();
            let id = value.attr("id").unwrap_or_default();
            if class.is_empty() && id.is_empty() {
                continue;
            }
            let ident = format!("{class} {id}").to_lowercase();
            if AD_PATTERNS.iter().any(|p| ident.contains(p)) {
                doomed.push(el.id());
                removed_ads += 1;
            } else if POPUP_PATTERNS.iter().any(|p| ident.contains(p)) {
                doomed.push(el.id());
                removed_popups += 1;
            } else if CHROME_PATTERNS.iter().any(|p| ident.contains(p)) {
                doomed.push(el.id());
            }
        }
    }

    for id in doomed {
        if let Some(mut node) = doc.tree.get_mut(id) {
            node.detach();
        }
    }

    // Chọn khối chứa nội dung: khối có nhiều chữ trong <p> nhất, trừ đi phần chữ nằm trong link.
    let para_sel = Selector::parse("p").expect("selector p");
    let link_sel = Selector::parse("a").expect("selector a");
    let candidate_sel = Selector::parse(
        "article, main, [itemprop=articleBody], [class*=article], [class*=content], [class*=post], [class*=entry], [class*=detail], div, section",
    )
    .expect("candidate selector");

    let mut best: Option<(ElementRef, isize)> = None;
    for el in doc.select(&candidate_sel) {
        let mut text_len: isize = 0;
        for p in el.select(&para_sel) {
            let len = p.text().map(str::len).sum::<usize>() as isize;
            if len > 40 {
                text_len += len;
            }
        }
        if text_len == 0 {
            continue;
        }
        let link_len: isize = el.select(&link_sel).map(|a| a.text().map(str::len).sum::<usize>() as isize).sum();
        let score = text_len - link_len / 2;
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((el, score));
        }
    }

    let mut blocks: Vec<Block> = Vec::new();
    let mut images: Vec<String> = Vec::new();

    if let Some((root, _)) = best {
        {
            let content_sel = Selector::parse("p, h2, h3, h4, blockquote, li, img").expect("content selector");
            for el in root.select(&content_sel) {
                let tag = el.value().name();
                if tag == "img" {
                    if let Some(src) = image_src(&el, base) {
                        if !images.contains(&src) {
                            images.push(src.clone());
                            blocks.push(Block::Image { src });
                        }
                    }
                    continue;
                }
                let text = el.text().collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ");
                if text.chars().count() < 25 {
                    continue;
                }
                let block = match tag {
                    "h2" | "h3" | "h4" => Block::Heading { text },
                    "blockquote" => Block::Quote { text },
                    _ => Block::Paragraph { text },
                };
                // Bỏ đoạn trùng lặp thường sinh ra do markup lồng nhau.
                let duplicated = blocks.iter().any(|b| match (b, &block) {
                    (Block::Paragraph { text: a }, Block::Paragraph { text: b })
                    | (Block::Heading { text: a }, Block::Heading { text: b })
                    | (Block::Quote { text: a }, Block::Quote { text: b }) => a == b,
                    _ => false,
                });
                if !duplicated {
                    blocks.push(block);
                }
            }
        }
    }

    if let Some(lead) = &lead_image {
        if !images.contains(lead) {
            images.insert(0, lead.clone());
        }
    }

    let word_count: usize = blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph { text } | Block::Heading { text } | Block::Quote { text } => {
                text.split_whitespace().count()
            }
            Block::Image { .. } => 0,
        })
        .sum();

    CleanedArticle {
        blocks,
        images,
        lead_image,
        byline,
        word_count,
        read_minutes: (word_count / 200).max(1),
        removed_ads,
        removed_popups,
        removed_trackers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r##"
    <html><head>
      <meta property="og:image" content="/hinh/anh-bia.jpg">
      <meta name="author" content="Ban Công nghệ">
    </head><body>
      <script>var tracker = 1;</script>
      <nav class="menu"><a href="/">Trang chủ</a></nav>
      <div class="ad-slot">Quảng cáo hiển thị ở đây</div>
      <div id="newsletter-popup">Đăng ký nhận bản tin</div>
      <article class="article-body">
        <p>Đoạn mở đầu của bài viết nói về một sự kiện công nghệ vừa diễn ra sáng nay.</p>
        <div class="advert-inline">Quảng cáo chèn giữa bài, cần bị loại bỏ hoàn toàn.</div>
        <p>Đoạn thứ hai giải thích chi tiết kỹ thuật và bối cảnh của sự kiện được nhắc tới.</p>
        <h2>Một tiêu đề phụ đủ dài để không bị lọc</h2>
        <img src="/hinh/minh-hoa.jpg">
        <blockquote>Một trích dẫn đủ dài để vượt qua ngưỡng lọc ký tự tối thiểu.</blockquote>
      </article>
      <div class="related-posts"><p>Bài viết liên quan mà chúng ta không muốn lấy vào.</p></div>
      <iframe src="https://quang-cao.example/frame"></iframe>
    </body></html>
    "##;

    fn cleaned() -> CleanedArticle {
        let base = Url::parse("https://bao.example/tin/bai-viet").unwrap();
        extract(PAGE, &base)
    }

    #[test]
    fn dem_dung_so_khoi_quang_cao_va_popup() {
        let result = cleaned();
        assert_eq!(result.removed_ads, 2, "phải bắt được cả ad-slot lẫn advert-inline");
        assert_eq!(result.removed_popups, 1, "phải bắt được newsletter-popup");
        assert!(result.removed_trackers >= 2, "script và iframe đều phải bị loại");
    }

    #[test]
    fn giu_lai_noi_dung_va_loai_bo_chu_quang_cao() {
        let text = cleaned()
            .blocks
            .iter()
            .map(|b| match b {
                Block::Paragraph { text } | Block::Heading { text } | Block::Quote { text } => text.clone(),
                Block::Image { .. } => String::new(),
            })
            .collect::<Vec<_>>()
            .join(" ");

        assert!(text.contains("Đoạn mở đầu"), "thân bài phải được giữ");
        assert!(text.contains("Đoạn thứ hai"), "thân bài phải được giữ");
        assert!(!text.contains("Quảng cáo"), "không được sót chữ trong khối quảng cáo");
        assert!(!text.contains("liên quan"), "khối bài liên quan phải bị loại");
    }

    #[test]
    fn quy_anh_ve_dia_chi_tuyet_doi() {
        let result = cleaned();
        assert_eq!(result.lead_image.as_deref(), Some("https://bao.example/hinh/anh-bia.jpg"));
        assert!(result
            .images
            .iter()
            .any(|src| src == "https://bao.example/hinh/minh-hoa.jpg"));
    }

    #[test]
    fn nhan_dien_cac_loai_khoi() {
        let blocks = cleaned().blocks;
        assert!(blocks.iter().any(|b| matches!(b, Block::Heading { .. })));
        assert!(blocks.iter().any(|b| matches!(b, Block::Quote { .. })));
        assert!(blocks.iter().any(|b| matches!(b, Block::Image { .. })));
    }
}
