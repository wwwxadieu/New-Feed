use serde::{Deserialize, Serialize};

/// Băm ổn định theo URL để mỗi bài/nguồn có một id không đổi giữa các lần chạy.
pub fn stable_id(input: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub title: String,
    pub home_url: String,
    pub feed_url: String,
    pub enabled: bool,
    pub added_at: String,
    #[serde(default)]
    pub last_fetched: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub article_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Article {
    pub id: String,
    pub source_id: String,
    pub source_title: String,
    pub title: String,
    pub url: String,
    pub summary: String,
    /// RFC3339
    pub published: String,
    #[serde(default)]
    pub image: Option<String>,
}

/// Nội dung bài đã bóc tách, kèm số khối rác thực sự đã loại bỏ.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanedArticle {
    pub blocks: Vec<Block>,
    pub images: Vec<String>,
    #[serde(default)]
    pub lead_image: Option<String>,
    #[serde(default)]
    pub byline: Option<String>,
    pub word_count: usize,
    pub read_minutes: usize,
    pub removed_ads: usize,
    pub removed_popups: usize,
    pub removed_trackers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Block {
    Paragraph { text: String },
    Heading { text: String },
    Quote { text: String },
    Image { src: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cluster {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub topic: String,
    pub newest: String,
    pub score: f32,
    pub source_count: usize,
    pub articles: Vec<Article>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// "auto" | "light" | "dark"
    pub theme: String,
    /// Cửa sổ thời gian mặc định của dashboard, tính bằng giờ.
    pub window_hours: i64,
    /// Số bài lấy tối đa cho mỗi nguồn ở một lần làm mới.
    pub max_per_source: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self { theme: "auto".into(), window_hours: 24, max_per_source: 25 }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub articles: Vec<Article>,
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub last_refresh: Option<String>,
}

/// Gói dữ liệu trả về cho giao diện sau mỗi thao tác.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub sources: Vec<Source>,
    pub clusters: Vec<Cluster>,
    pub settings: Settings,
    pub article_count: usize,
    pub topic_counts: Vec<(String, usize)>,
    pub hourly: Vec<usize>,
    pub last_refresh: Option<String>,
}
