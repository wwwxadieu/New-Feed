//! Lưu trạng thái ứng dụng ra đĩa dưới dạng JSON trong thư mục cấu hình của người dùng.

use crate::model::AppData;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

fn state_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Không xác định được thư mục cấu hình: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Không tạo được thư mục cấu hình: {e}"))?;
    Ok(dir.join("state.json"))
}

pub fn load(app: &AppHandle) -> AppData {
    let Ok(path) = state_path(app) else {
        return AppData::default();
    };
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return AppData::default();
    };
    // Nếu file hỏng thì bắt đầu lại từ đầu thay vì làm sập ứng dụng.
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn save(app: &AppHandle, data: &AppData) -> Result<(), String> {
    let path = state_path(app)?;
    // Ghi gọn chứ không xuống dòng cho đẹp: kho tin đầy là vài nghìn bài kèm
    // thân bài dự phòng, bản xuống dòng tốn thêm hàng megabyte và chừng ấy
    // thời gian ghi đĩa ở mỗi lượt làm mới.
    let raw = serde_json::to_string(data).map_err(|e| format!("Không tuần tự hoá được dữ liệu: {e}"))?;
    // Ghi ra file tạm rồi đổi tên: mất điện giữa chừng không làm hỏng state.json.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, raw).map_err(|e| format!("Không ghi được dữ liệu: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Không lưu được dữ liệu: {e}"))?;
    Ok(())
}

/// Nguồn có sẵn, xếp theo đúng các chủ đề mà ứng dụng phân loại.
///
/// Mỗi chủ đề trong thanh bên cần ít nhất vài nguồn chuyên về nó, nếu không
/// thì nhóm đó gần như luôn rỗng và bộ gộp cụm cũng không có gì để đối chiếu
/// — một sự kiện chỉ đáng chú ý khi nhiều báo cùng đưa.
///
/// Vài trang có cả feed tổng lẫn feed chuyên mục (TechCrunch chẳng hạn). Để
/// cả hai không sao: mã định danh bài lấy theo địa chỉ bài nên bài trùng tự
/// gộp làm một, còn feed chuyên mục thì đi sâu hơn feed tổng.
pub fn default_sources() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        // Báo công nghệ trong nước.
        ("VnExpress Số hóa", "https://vnexpress.net/so-hoa", "https://vnexpress.net/rss/so-hoa.rss"),
        ("Genk", "https://genk.vn", "https://genk.vn/rss/home.rss"),
        ("VietnamNet Công nghệ", "https://vietnamnet.vn/cong-nghe", "https://vietnamnet.vn/rss/cong-nghe.rss"),
        ("Tinh tế", "https://tinhte.vn", "https://tinhte.vn/rss"),
        ("Dân trí Sức mạnh số", "https://dantri.com.vn/suc-manh-so.htm", "https://dantri.com.vn/rss/suc-manh-so.rss"),
        ("Tuổi Trẻ Nhịp sống số", "https://tuoitre.vn/nhip-song-so.htm", "https://tuoitre.vn/rss/nhip-song-so.rss"),
        ("Thanh Niên Công nghệ", "https://thanhnien.vn/cong-nghe.htm", "https://thanhnien.vn/rss/cong-nghe.rss"),

        // Báo công nghệ quốc tế, đưa tin rộng.
        ("TechCrunch", "https://techcrunch.com", "https://techcrunch.com/feed/"),
        ("The Verge", "https://www.theverge.com", "https://www.theverge.com/rss/index.xml"),
        ("Ars Technica", "https://arstechnica.com", "https://feeds.arstechnica.com/arstechnica/index"),
        ("Engadget", "https://www.engadget.com", "https://www.engadget.com/rss.xml"),

        // AI & mô hình.
        ("VentureBeat AI", "https://venturebeat.com/category/ai/", "https://venturebeat.com/category/ai/feed/"),
        ("MIT Technology Review", "https://www.technologyreview.com", "https://www.technologyreview.com/feed/"),
        ("The Decoder", "https://the-decoder.com", "https://the-decoder.com/feed/"),

        // Bảo mật.
        ("BleepingComputer", "https://www.bleepingcomputer.com", "https://www.bleepingcomputer.com/feed/"),
        ("The Hacker News", "https://thehackernews.com", "https://feeds.feedburner.com/TheHackersNews"),
        ("Krebs on Security", "https://krebsonsecurity.com", "https://krebsonsecurity.com/feed/"),

        // Phần cứng.
        ("Tom's Hardware", "https://www.tomshardware.com", "https://www.tomshardware.com/feeds/all"),
        ("ServeTheHome", "https://www.servethehome.com", "https://www.servethehome.com/feed/"),

        // Điện thoại & thiết bị.
        ("9to5Mac", "https://9to5mac.com", "https://9to5mac.com/feed/"),
        ("Android Authority", "https://www.androidauthority.com", "https://www.androidauthority.com/feed/"),

        // Game & esports.
        ("Eurogamer", "https://www.eurogamer.net", "https://www.eurogamer.net/feed"),
        ("PC Gamer", "https://www.pcgamer.com", "https://www.pcgamer.com/rss/"),
        ("GameSpot", "https://www.gamespot.com", "https://www.gamespot.com/feeds/news/"),

        // Startup & vốn.
        ("TechCrunch Startups", "https://techcrunch.com/category/startups/", "https://techcrunch.com/category/startups/feed/"),
        ("Rest of World", "https://restofworld.org", "https://restofworld.org/feed/latest/"),

        // Xe điện.
        ("Electrek", "https://electrek.co", "https://electrek.co/feed/"),
        ("InsideEVs", "https://insideevs.com", "https://insideevs.com/rss/articles/all/"),

        // Mạng xã hội.
        ("Social Media Today", "https://www.socialmediatoday.com", "https://www.socialmediatoday.com/feeds/news/"),

        // Không gian.
        ("Space.com", "https://www.space.com", "https://www.space.com/feeds/all"),
        ("SpaceNews", "https://spacenews.com", "https://spacenews.com/feed/"),
    ]
}
