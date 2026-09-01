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
    /// Logo của nguồn, lưu thẳng dạng data URI để hiển thị được cả khi offline.
    #[serde(default)]
    pub logo: Option<String>,
    /// "vi" nếu nguồn viết bằng tiếng Việt, "other" nếu là tiếng nước ngoài.
    #[serde(default)]
    pub language: Option<String>,
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
    /// Tiêu đề đã dịch sang tiếng Việt, chỉ có với nguồn nước ngoài.
    #[serde(default)]
    pub title_vi: Option<String>,
    /// Tóm tắt đã dịch sang tiếng Việt.
    #[serde(default)]
    pub summary_vi: Option<String>,
    /// Đường dẫn ảnh đại diện đã tải về và thu nhỏ trên máy.
    #[serde(default)]
    pub thumb: Option<String>,
    /// Nội dung HTML lấy từ feed, dùng làm phương án dự phòng khi không bóc
    /// tách được thân bài từ trang gốc.
    #[serde(default)]
    pub content_html: Option<String>,
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
    /// Đúng khi chỉ lấy được phần tóm tắt chứ không phải toàn văn.
    #[serde(default)]
    pub partial: bool,
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
    /// Tiêu đề tiếng Việt của bài đại diện, nếu nguồn là tiếng nước ngoài.
    pub title_vi: Option<String>,
    pub summary_vi: Option<String>,
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
    /// Dịch tiêu đề của nguồn nước ngoài sang tiếng Việt.
    #[serde(default = "default_true")]
    pub translate: bool,
    /// Email khai báo với dịch vụ dịch để nâng hạn mức miễn phí hằng ngày
    /// từ 5.000 lên 50.000 ký tự. Để trống vẫn dùng được.
    #[serde(default)]
    pub translate_email: String,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "auto".into(),
            window_hours: 24,
            max_per_source: 25,
            translate: true,
            translate_email: String::new(),
        }
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
    /// Mã của những nguồn mặc định đã từng được nạp vào máy này.
    ///
    /// Nhờ đó bản cập nhật có thêm nguồn mới vẫn tới được máy đang dùng dở,
    /// mà nguồn người dùng đã cố ý xoá thì không mọc lại ở lần mở sau.
    #[serde(default)]
    pub seeded_defaults: Vec<String>,
    /// Thông báo tạm về việc dịch, ví dụ khi hết hạn mức. Không lưu ra đĩa.
    #[serde(skip)]
    pub translate_notice: Option<String>,
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
    pub translate_notice: Option<String>,
}
