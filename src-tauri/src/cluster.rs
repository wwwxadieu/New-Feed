//! Gộp các bài viết nói về cùng một sự kiện thành cụm, và phân loại chủ đề.

use crate::model::{stable_id, Article, Cluster};
use chrono::{DateTime, Utc};
use std::collections::{HashMap, HashSet};

const STOPWORDS: &[&str] = &[
    "và", "của", "cho", "với", "trong", "các", "những", "được", "là", "có", "khi", "từ", "sau",
    "trước", "một", "người", "này", "đó", "đã", "sẽ", "tại", "về", "ra", "vào", "trên", "dưới",
    "theo", "để", "không", "cũng", "như", "thì", "mà", "nhưng", "hay", "hoặc", "bị", "làm", "nên",
    "the", "a", "an", "of", "for", "and", "to", "in", "on", "with", "is", "are", "at", "by",
    "from", "new", "its", "it", "as", "that", "this", "you", "your", "has", "have", "will",
];

/// Ngưỡng tương đồng để hai bài được coi là cùng một sự kiện.
const SIMILARITY_THRESHOLD: f32 = 0.45;
/// Số từ chung tối thiểu, để một tiêu đề ngắn không vô tình khớp với tiêu đề dài.
const MIN_SHARED_TOKENS: usize = 2;
/// Hai bài cách nhau quá số giờ này thì không gộp, dù tiêu đề giống nhau.
const MAX_GAP_HOURS: i64 = 72;

fn tokenize(title: &str) -> HashSet<String> {
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.chars().count() >= 2)
        .filter(|t| !STOPWORDS.contains(t))
        .map(str::to_string)
        .collect()
}

fn parse_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

/// Hệ số chồng lấn có trọng số.
///
/// Dùng `min(tổng a, tổng b)` làm mẫu số thay vì hợp của hai tập: hai bản tin
/// về cùng sự kiện thường khác nhau đáng kể về độ dài tiêu đề, và Jaccard
/// phạt sự chênh lệch đó quá nặng nên hay tách nhầm thành hai cụm.
fn similarity(a: &HashSet<String>, b: &HashSet<String>, weights: &HashMap<String, f32>) -> f32 {
    let weight_of = |t: &String| *weights.get(t).unwrap_or(&1.0);
    let shared: Vec<&String> = a.intersection(b).collect();
    if shared.len() < MIN_SHARED_TOKENS {
        return 0.0;
    }
    let intersection: f32 = shared.into_iter().map(weight_of).sum();
    let total_a: f32 = a.iter().map(weight_of).sum();
    let total_b: f32 = b.iter().map(weight_of).sum();
    let smaller = total_a.min(total_b);
    if smaller <= 0.0 {
        return 0.0;
    }
    intersection / smaller
}

pub fn classify(title: &str, summary: &str) -> &'static str {
    // Thêm khoảng trắng hai đầu để mẫu như " ai " khớp được cả khi từ đứng đầu câu.
    let text = format!(" {} {} ", title.to_lowercase(), summary.to_lowercase());
    let has = |keys: &[&str]| keys.iter().any(|k| text.contains(k));

    // Thứ tự quan trọng. Gần như mọi tin công nghệ bây giờ đều nhắc tới AI, nên
    // các chủ đề có dấu hiệu cụ thể hơn phải được xét trước, nếu không mọi thứ
    // sẽ rơi hết vào nhóm AI.
    if has(&[
        "lỗ hổng", "bảo mật", "vulnerability", "mã độc", "malware", "ransomware", "cve-",
        "tấn công mạng", "rò rỉ dữ liệu", "data breach", "phishing", "lừa đảo", "hacker",
        "bản vá", "bị hack", "chiếm đoạt tài khoản", "an ninh mạng", "mã hoá đầu cuối",
        "zero-day", "exploit", "cyberattack", "cybersecurity", "botnet", "spyware",
        "infostealer", "backdoor", "ddos", "patch tuesday", "threat actor", "stolen credentials",
        "supply chain attack", "phần mềm gián điệp", "tống tiền dữ liệu",
    ]) {
        return "security";
    }
    // Xét trước nhóm thiết bị nên tránh những chữ dùng chung với tên sản phẩm:
    // "galaxy" là điện thoại Samsung, "eclipse" là bộ công cụ lập trình, và
    // "rocket" không kèm gì thì kéo cả Rocket League sang đây.
    if has(&[
        "vệ tinh", "không gian", "tên lửa", "nasa", "spacex", "quỹ đạo", "satellite",
        "vũ trụ", "sao hoả", "mặt trăng", "thiên hà", "kính viễn vọng",
        "asteroid", "astronaut", "spacecraft", "telescope", "orbital", "in orbit",
        "rocket launch", "rocket lab", "blue origin", "starship", "starlink", "artemis",
        "james webb", "solar eclipse", "lunar", "comet", "meteor", " esa ", " iss ",
        "phi hành gia", "tiểu hành tinh", "thiên thạch", "nhật thực", "trạm vũ trụ",
        "sao mộc", "sao thổ", "sao kim",
    ]) {
        return "space";
    }
    if has(&[
        "xe điện", "trạm sạc", "sạc nhanh", "pin xe", "tesla", "vinfast", "ô tô điện",
        "xe tự lái", "xe máy điện", "hybrid",
        "electric vehicle", " ev ", " evs ", "charging station", "battery pack",
        "solid-state battery", "plug-in", "robotaxi", "waymo", "autonomous driving",
        "rivian", "lucid motors", "xpeng", " byd ", " nio ", "pin thể rắn", "xe tự hành",
    ]) {
        return "ev";
    }
    if has(&[
        // Tránh dùng riêng chữ "game": tiếng Anh hay có "game-changer",
        // "the game is changing" chẳng liên quan gì tới trò chơi.
        "trò chơi điện tử", "tựa game", "game thủ", "làng game", "game mới", "ra mắt game",
        "phát hành game", "cộng đồng game", "máy chơi game", "nhà phát triển game",
        "esports", "thể thao điện tử", " gaming ", "video game", "game studio",
        "playstation", " ps5", " ps6", "xbox", "nintendo", "steam deck", " steam ",
        "epic games", "game pass", "rockstar", "ubisoft", "activision", "blizzard",
        "riot games", " gta ", "minecraft", "fortnite", "call of duty", "elden ring",
        "pokemon", "pokémon", "zelda", "genshin", "liên quân", "tốc chiến",
        "game awards", "gamescom", "tokyo game show", "summer game fest",
        "switch 2", "game console", "indie game", "roblox", "valorant", "dota",
        "league of legends", "counter-strike", "final fantasy", "resident evil",
        "assassin's creed", "battlefield", "starfield", "cyberpunk 2077", "baldur's gate",
        "capcom", "square enix", "bandai namco", "ea sports", "bethesda", "valve",
        "rocket league", "helldivers", "hollow knight", "silent hill",
    ]) {
        return "games";
    }
    if has(&[
        "iphone", "ipad", "macbook", "mac mini", "mac studio", "apple watch", "airpods",
        "samsung", "xiaomi", "oppo", "vivo", "realme", "pixel", "galaxy",
        "điện thoại", "smartphone", "laptop", "máy tính bảng", "tai nghe", "smartwatch",
        "đồng hồ thông minh", "máy ảnh", "mirrorless", "android", " ios ",
        "oneplus", "nothing phone", "foldable", "máy gập", "vision pro", "quest 3",
        "smart glasses", "kính thông minh", "chromebook", "surface pro", "wearable",
        "ipados", "watchos", "macos", "windows 11", "tablet", "earbuds",
        "nhà thông minh", "smart home", "sạc dự phòng",
    ]) {
        return "device";
    }
    if has(&[
        " ai ", " ai,", " ai.", " ai:", " ai-", " ai’s", " ai's", "trí tuệ nhân tạo",
        "mô hình ngôn ngữ", "llm", "chatgpt", "openai", "anthropic", "gemini", "claude",
        "copilot", "học máy", "machine learning", "deep learning", "mạng nơ-ron",
        "chatbot", "tạo sinh", "generative", "gpt-", "llama", "mistral", "deepseek",
        "midjourney", "stable diffusion", "hugging face", "perplexity", "grok",
        "fine-tuning", "agentic", "neural network", "mô hình nền tảng",
    ]) {
        return "ai";
    }
    if has(&[
        "chip", "cpu", "gpu", "bán dẫn", "semiconductor", "wafer", "vi xử lý", "nvidia",
        "intel", "amd", "tsmc", "snapdragon", "nanomet", "2nm", "3nm", " ram ", " ssd ",
        "card đồ hoạ", "trung tâm dữ liệu", "máy chủ", "siêu máy tính", "bộ nhớ",
        "qualcomm", "mediatek", "micron", "sk hynix", " asml ", "lithography", " hbm",
        "ddr5", "geforce", "radeon", "rtx ", "ryzen", "core ultra", "apple silicon",
        "risc-v", " arm ", "motherboard", "bo mạch chủ", "tản nhiệt", "data center",
        "bán dẫn", "đúc chip",
    ]) {
        return "hardware";
    }
    if has(&[
        "mạng xã hội", "facebook", "instagram", "tiktok", "threads", "twitter", "youtube",
        "telegram", "zalo", "nhà sáng tạo", "livestream",
        "reddit", "bluesky", "discord", "snapchat", "linkedin", "pinterest", "twitch",
        "mastodon", "whatsapp", "meta platforms", "social media", "creator economy",
        "content moderation", "kiểm duyệt nội dung", "influencer",
    ]) {
        return "social";
    }
    "other"
}

pub fn build(articles: &[Article]) -> Vec<Cluster> {
    if articles.is_empty() {
        return Vec::new();
    }

    // Đọc mốc thời gian đúng một lần rồi mang theo. Vòng gộp bên dưới so từng
    // cặp bài; nếu để nó phân tích lại chuỗi RFC3339 ở mỗi lần so thì riêng
    // việc đọc chuỗi đã chiếm phần lớn thời gian dựng cụm.
    let mut indexed: Vec<(&Article, DateTime<Utc>)> =
        articles.iter().map(|a| (a, parse_time(&a.published))).collect();
    indexed.sort_by(|a, b| b.1.cmp(&a.1));
    let times: Vec<DateTime<Utc>> = indexed.iter().map(|(_, t)| *t).collect();
    let sorted: Vec<&Article> = indexed.into_iter().map(|(a, _)| a).collect();

    let tokens: Vec<HashSet<String>> = sorted.iter().map(|a| tokenize(&a.title)).collect();

    // Trọng số IDF có làm trơn: tên riêng hiếm gặp nặng hơn, nhưng từ phổ biến
    // vẫn giữ trọng số tối thiểu 1.0 thay vì bị triệt tiêu khi kho tin còn nhỏ.
    let mut document_freq: HashMap<String, usize> = HashMap::new();
    for set in &tokens {
        for token in set {
            *document_freq.entry(token.clone()).or_insert(0) += 1;
        }
    }
    let total = sorted.len() as f32;
    let weights: HashMap<String, f32> = document_freq
        .into_iter()
        .map(|(token, freq)| {
            let weight = ((total + 1.0) / (freq as f32 + 1.0)).ln() + 1.0;
            (token, weight)
        })
        .collect();

    // Gộp tham lam: mỗi bài tìm cụm có bài gần nhất giống nó nhất.
    //
    // Kèm một chỉ số ngược từ từ khoá sang cụm để khỏi phải quét hết mọi cụm
    // cho từng bài. Hai tiêu đề không có lấy một từ chung thì độ tương đồng
    // chắc chắn bằng 0, so tiếp cũng vô ích — mà với kho vài nghìn bài thì
    // số phép so vô ích đó chiếm gần như toàn bộ thời gian dựng cụm.
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut index: HashMap<&str, Vec<usize>> = HashMap::new();

    for idx in 0..sorted.len() {
        let published = times[idx];

        // Xét theo thứ tự cụm tăng dần để giữ nguyên cách chọn khi hai cụm
        // hoà điểm: cụm xuất hiện trước thắng.
        let mut candidates: Vec<usize> = tokens[idx]
            .iter()
            .filter_map(|token| index.get(token.as_str()))
            .flatten()
            .copied()
            .collect();
        candidates.sort_unstable();
        candidates.dedup();

        let mut best: Option<(usize, f32)> = None;
        for group_idx in candidates {
            let group = &groups[group_idx];
            // Bài được duyệt từ mới về cũ nên phần tử cuối của cụm là bài gần
            // bài hiện tại nhất về thời gian. Nó đã quá hạn thì cả cụm cũng vậy.
            if let Some(&closest) = group.last() {
                if (published - times[closest]).num_hours().abs() > MAX_GAP_HOURS {
                    continue;
                }
            }

            let mut peak = 0.0f32;
            for &member in group {
                let gap = (published - times[member]).num_hours().abs();
                if gap > MAX_GAP_HOURS {
                    continue;
                }
                let score = similarity(&tokens[idx], &tokens[member], &weights);
                if score > peak {
                    peak = score;
                }
            }
            if peak >= SIMILARITY_THRESHOLD && best.map(|(_, b)| peak > b).unwrap_or(true) {
                best = Some((group_idx, peak));
            }
        }

        let group_idx = match best {
            Some((group_idx, _)) => {
                groups[group_idx].push(idx);
                group_idx
            }
            None => {
                groups.push(vec![idx]);
                groups.len() - 1
            }
        };
        for token in &tokens[idx] {
            index.entry(token.as_str()).or_default().push(group_idx);
        }
    }

    let now = Utc::now();
    let mut clusters: Vec<Cluster> = groups
        .into_iter()
        .map(|group| {
            // Bài đại diện: tiêu đề nhiều thông tin nhất trong cụm.
            let lead = *group
                .iter()
                .max_by_key(|&&i| tokens[i].len())
                .expect("cụm không rỗng");
            let lead_article = sorted[lead];

            let articles: Vec<Article> = group.iter().map(|&i| sorted[i].clone()).collect();
            let source_count = articles.iter().map(|a| a.source_id.as_str()).collect::<HashSet<_>>().len();
            let newest = articles
                .iter()
                .map(|a| parse_time(&a.published))
                .max()
                .unwrap_or(now);

            // Lấy tóm tắt dài nhất, và lấy luôn bản dịch của chính bài đó để
            // tiêu đề và tóm tắt không lệch nhau về ngôn ngữ.
            let summary_source = articles
                .iter()
                .max_by_key(|a| a.summary.chars().count())
                .expect("cụm không rỗng");
            let summary = summary_source.summary.clone();
            let summary_vi = summary_source.summary_vi.clone();

            let hours_old = (now - newest).num_minutes() as f32 / 60.0;
            let score = source_count as f32 * (1.0 / (1.0 + hours_old / 12.0)) * 100.0;

            Cluster {
                id: stable_id(&lead_article.url),
                title: lead_article.title.clone(),
                title_vi: lead_article.title_vi.clone(),
                summary_vi,
                summary,
                topic: classify(&lead_article.title, &lead_article.summary).to_string(),
                newest: newest.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                score,
                source_count,
                articles,
            }
        })
        .collect();

    clusters.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    clusters
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::stable_id;
    use chrono::Duration;

    fn article(source: &str, title: &str, hours_ago: i64) -> Article {
        let url = format!("https://{source}.example/{}", stable_id(title));
        Article {
            id: stable_id(&url),
            source_id: source.to_string(),
            source_title: source.to_string(),
            title: title.to_string(),
            url,
            summary: String::new(),
            title_vi: None,
            summary_vi: None,
            thumb: None,
            hero: None,
            image_checked: false,
            content_html: None,
            published: (Utc::now() - Duration::hours(hours_ago))
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            image: None,
        }
    }

    #[test]
    fn gop_cac_bai_cung_su_kien() {
        let articles = vec![
            article("vnexpress", "Lỗ hổng nghiêm trọng trong thư viện nén khiến máy chủ bị tấn công", 1),
            article("genk", "Thư viện nén dính lỗ hổng nghiêm trọng, máy chủ có nguy cơ bị tấn công", 2),
            article("tinhte", "Phát hiện lỗ hổng thư viện nén, khuyến cáo vá máy chủ ngay", 3),
        ];
        let clusters = build(&articles);
        assert_eq!(clusters.len(), 1, "ba bài cùng sự kiện phải nằm trong một cụm");
        assert_eq!(clusters[0].source_count, 3);
    }

    #[test]
    fn khong_gop_hai_su_kien_khac_nhau() {
        let articles = vec![
            article("vnexpress", "Lỗ hổng nghiêm trọng trong thư viện nén khiến máy chủ bị tấn công", 1),
            article("genk", "Hãng xe điện công bố chuẩn sạc mới rút ngắn thời gian nạp pin", 2),
        ];
        assert_eq!(build(&articles).len(), 2);
    }

    #[test]
    fn khong_gop_khi_cach_nhau_qua_lau() {
        let articles = vec![
            article("vnexpress", "Lỗ hổng nghiêm trọng trong thư viện nén khiến máy chủ bị tấn công", 1),
            article("genk", "Lỗ hổng nghiêm trọng trong thư viện nén khiến máy chủ bị tấn công", 200),
        ];
        assert_eq!(build(&articles).len(), 2, "quá 72 giờ thì coi là hai sự kiện");
    }

    #[test]
    fn dem_nguon_theo_dau_bao_khong_theo_so_bai() {
        let articles = vec![
            article("vnexpress", "Lỗ hổng nghiêm trọng trong thư viện nén khiến máy chủ bị tấn công", 1),
            article("vnexpress", "Lỗ hổng nghiêm trọng thư viện nén khiến nhiều máy chủ bị tấn công", 2),
        ];
        let clusters = build(&articles);
        assert_eq!(clusters[0].articles.len(), 2);
        assert_eq!(clusters[0].source_count, 1, "cùng một báo chỉ tính là một nguồn");
    }

    #[test]
    fn phan_loai_chu_de() {
        assert_eq!(classify("Lỗ hổng bảo mật mới được công bố", ""), "security");
        assert_eq!(classify("Mô hình ngôn ngữ mở vừa ra mắt", ""), "ai");
        assert_eq!(classify("Tiến trình chip 2nm đi vào sản xuất", ""), "hardware");
        assert_eq!(classify("Trời hôm nay đẹp", ""), "other");
    }

    #[test]
    fn phan_loai_tin_game() {
        assert_eq!(classify("Rockstar phản hồi về rò rỉ GTA 6", ""), "games");
        assert_eq!(classify("Nintendo công bố ngày phát hành máy mới", ""), "games");
        assert_eq!(classify("Tựa game nhập vai mới ra mắt trên Steam Deck", ""), "games");
        assert_eq!(classify("Giải thể thao điện tử lớn nhất năm khởi tranh", ""), "games");
        // Tin thiết bị có nhắc tai nghe vẫn phải vào nhóm game nếu là tai nghe chơi game.
        assert_eq!(classify("Sony ra tai nghe mới cho PS5", ""), "games");

        // Chữ "game" trong thành ngữ tiếng Anh không được kéo tin sang nhóm game.
        assert_eq!(
            classify("Anthropic's new model is a game-changer for coding", "The model tops benchmarks."),
            "ai"
        );
        assert_eq!(classify("iPhone 18 ra mắt với camera nâng cấp", ""), "device");
    }

    /// Phần lớn nguồn mặc định là báo tiếng Anh, nên mỗi nhóm phải bắt được
    /// tin bằng tiếng Anh chứ không chỉ tiếng Việt.
    #[test]
    fn phan_loai_tin_tieng_anh_theo_dung_nhom() {
        assert_eq!(classify("Rocket Lab launches 20 satellites into orbit", ""), "space");
        assert_eq!(classify("Zero-day exploit hits enterprise VPN gateways", ""), "security");
        assert_eq!(
            classify("Rivian opens its charging network to other electric vehicle owners", ""),
            "ev"
        );
        assert_eq!(classify("Valve announces a new handheld game console", ""), "games");
        assert_eq!(classify("OnePlus unveils a foldable with a bigger cover screen", ""), "device");
        assert_eq!(classify("DeepSeek releases a smaller model with open weights", ""), "ai");
        assert_eq!(classify("Micron starts shipping HBM4 for data center accelerators", ""), "hardware");
        assert_eq!(classify("Bluesky tightens content moderation after a spam wave", ""), "social");
        // Không còn nhóm riêng cho tin gọi vốn, chúng rơi về nhóm Khác.
        assert_eq!(classify("Y Combinator startup raises $40 million in a seed round", ""), "other");
    }

    /// Nhóm không gian được xét trước nhóm thiết bị, nên những chữ dùng chung
    /// với tên sản phẩm phải nằm ngoài danh sách của nó.
    #[test]
    fn tu_ngu_khong_gian_khong_keo_nham_tin_khac() {
        assert_eq!(classify("Samsung Galaxy S26 ra mắt với màn hình sáng hơn", ""), "device");
        assert_eq!(classify("Rocket League nhận bản cập nhật mùa mới", ""), "games");
        assert_eq!(classify("Eclipse ra bản mới cho nhà phát triển Java", ""), "other");
    }

    #[test]
    fn tin_san_pham_khong_bi_gan_nham_thanh_ai() {
        // Gần như mọi tin sản phẩm bây giờ đều nhắc tới AI ở đâu đó, nhưng
        // đây vẫn là tin thiết bị chứ không phải tin AI.
        assert_eq!(
            classify(
                "Mac Studio mới ra mắt, có thể ghép bốn máy thành cụm AI",
                "Sản phẩm hướng đến nhà phát triển và các quy trình xử lý AI."
            ),
            "device"
        );
        assert_eq!(
            classify("Apple đánh úp người dùng với chip M6 trên Mac mini", "Mac mini nâng cấp chip M6."),
            "device"
        );
        // Còn tin thật sự về AI thì vẫn phải vào đúng nhóm.
        assert_eq!(classify("Cơn sốt tuyển dụng nhân sự phim AI bùng nổ", ""), "ai");
    }
}
