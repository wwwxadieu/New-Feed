//! Thời tiết hiện tại cho ô hiển thị cạnh thanh tìm kiếm.
//!
//! Phải tải ở phía Rust chứ không phải ở giao diện: CSP của ứng dụng chỉ cho
//! connect-src tới 'self' và ipc, nên WebView không gọi được ra ngoài.
//!
//! Cả hai dịch vụ dùng ở đây đều không cần khoá API, đúng nguyên tắc của ứng
//! dụng là chạy được ngay sau khi cài mà không phải đăng ký tài khoản nào.

use serde::Serialize;
use std::time::{Duration, Instant};

/// Toạ độ và tên nơi đang ở.
#[derive(Clone)]
pub struct Place {
    lat: f64,
    lon: f64,
    name: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Weather {
    /// Nhiệt độ hiện tại, độ C, đã làm tròn.
    pub temp_c: i32,
    /// Mã thời tiết theo chuẩn WMO, giao diện tự chọn icon theo mã này.
    pub code: u8,
    /// Ban ngày hay ban đêm ở nơi đó, để chọn icon mặt trời hay mặt trăng.
    pub is_day: bool,
    /// Tên nơi đo, chỉ dùng làm chú thích khi rê chuột.
    pub place: String,
}

/// Vị trí hầu như không đổi trong một phiên chạy nên chỉ dò một lần.
const PLACE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// Open-Meteo cập nhật mỗi 15 phút, hỏi dày hơn cũng không có số liệu mới.
const WEATHER_TTL: Duration = Duration::from_secs(15 * 60);
/// Ô thời tiết là phần phụ; chờ lâu hơn mức này thì thà không hiện còn hơn.
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Default)]
pub struct Cache {
    /// Kèm cả chuỗi đã dùng để ra vị trí này. Người dùng đổi thành phố trong
    /// cài đặt thì chuỗi khác đi và bộ đệm tự mất hiệu lực — không có nó thì
    /// đổi xong vẫn phải chờ hết sáu tiếng mới thấy vị trí mới.
    place: Option<(Instant, String, Place)>,
    weather: Option<(Instant, Weather)>,
}

/// Dò vị trí theo địa chỉ IP.
///
/// Chỉ chính xác tới mức thành phố, và sai hẳn khi người dùng qua VPN — đo
/// thử trên cùng một máy thì hai dịch vụ khác nhau trả về hai thành phố cách
/// nhau hơn hai nghìn cây số. Chấp nhận được vì thời tiết chỉ cần đúng vùng,
/// và nếu hỏng thì ô này lặng lẽ biến mất chứ không báo lỗi.
async fn locate(client: &reqwest::Client) -> Option<Place> {
    let res = client.get("https://ipwho.is/").timeout(TIMEOUT).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    // reqwest ở dự án này tắt default-features nên không có .json().
    let body: serde_json::Value = serde_json::from_str(&res.text().await.ok()?).ok()?;
    if !body.get("success").and_then(serde_json::Value::as_bool).unwrap_or(false) {
        return None;
    }
    let lat = body.get("latitude")?.as_f64()?;
    let lon = body.get("longitude")?.as_f64()?;
    let name = body
        .get("city")
        .and_then(serde_json::Value::as_str)
        .filter(|s: &&str| !s.is_empty())
        .or_else(|| body.get("region").and_then(serde_json::Value::as_str))
        .unwrap_or("nơi bạn đang ở")
        .to_string();
    Some(Place { lat, lon, name })
}

/// Tra toạ độ từ tên thành phố người dùng nhập.
///
/// Dùng dịch vụ geocoding của chính Open-Meteo, cũng không cần khoá API. Lấy
/// kết quả đầu tiên: dịch vụ đã xếp theo mức phổ biến nên "Đà Nẵng" ra thành
/// phố Đà Nẵng chứ không ra một xã trùng tên.
async fn geocode(client: &reqwest::Client, name: &str) -> Option<Place> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=vi",
        urlencoding(name)
    );
    let res = client.get(&url).timeout(TIMEOUT).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    let body: serde_json::Value = serde_json::from_str(&res.text().await.ok()?).ok()?;
    let first = body.get("results")?.as_array()?.first()?;
    Some(Place {
        lat: first.get("latitude")?.as_f64()?,
        lon: first.get("longitude")?.as_f64()?,
        name: first
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(name)
            .to_string(),
    })
}

/// Mã hoá phần truy vấn. Tên thành phố tiếng Việt có dấu và có dấu cách, để
/// nguyên thì địa chỉ hỏng.
fn urlencoding(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() * 3);
    for byte in raw.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

async fn current(client: &reqwest::Client, place: &Place) -> Option<Weather> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.4}&longitude={:.4}\
         &current=temperature_2m,weather_code,is_day",
        place.lat, place.lon
    );
    let res = client.get(&url).timeout(TIMEOUT).send().await.ok()?;
    if !res.status().is_success() {
        return None;
    }
    // reqwest ở dự án này tắt default-features nên không có .json().
    let body: serde_json::Value = serde_json::from_str(&res.text().await.ok()?).ok()?;
    let now = body.get("current")?;
    Some(Weather {
        temp_c: now.get("temperature_2m")?.as_f64()?.round() as i32,
        code: now.get("weather_code")?.as_u64()? as u8,
        // is_day về dạng số 0/1 chứ không phải boolean.
        is_day: now.get("is_day").and_then(serde_json::Value::as_u64).unwrap_or(1) == 1,
        place: place.name.clone(),
    })
}

/// Thời tiết hiện tại, dùng lại bản đã lấy nếu còn mới.
///
/// `wanted` là thành phố người dùng đặt trong cài đặt; để trống thì tự dò
/// theo địa chỉ IP.
pub async fn fetch(
    client: &reqwest::Client,
    cache: &tokio::sync::Mutex<Cache>,
    wanted: &str,
) -> Option<Weather> {
    let wanted = wanted.trim();
    // Đổi thành phố thì số liệu cũ không còn đúng chỗ nữa, phải lấy lại ngay.
    let same_query = {
        let guard = cache.lock().await;
        guard.place.as_ref().is_some_and(|(_, q, _)| q == wanted)
    };
    if same_query {
        let guard = cache.lock().await;
        if let Some((at, weather)) = &guard.weather {
            if at.elapsed() < WEATHER_TTL {
                return Some(weather.clone());
            }
        }
    }

    // Thả khoá trước khi đi ra mạng, để lượt gọi khác không phải xếp hàng chờ.
    let (fresh_place, any_place) = {
        let guard = cache.lock().await;
        let matching = guard.place.as_ref().filter(|(_, q, _)| q == wanted);
        let any = matching.map(|(_, _, p)| p.clone());
        let fresh = matching.filter(|(at, _, _)| at.elapsed() < PLACE_TTL).map(|(_, _, p)| p.clone());
        (fresh, any)
    };

    // Hạn vị trí hết mà dò lại hỏng thì vẫn dùng vị trí cũ: hạn ở đây chỉ để
    // thỉnh thoảng làm mới, chứ nơi ở của người dùng không đổi sau vài tiếng.
    let resolve = async {
        if wanted.is_empty() {
            locate(client).await
        } else {
            geocode(client, wanted).await
        }
    };
    let place = match fresh_place {
        Some(place) => place,
        None => match resolve.await {
            Some(found) => {
                cache.lock().await.place =
                    Some((Instant::now(), wanted.to_string(), found.clone()));
                found
            }
            None => match any_place {
                Some(cu) => cu,
                // Người dùng gõ tên thành phố không tra được thì lùi về tự dò
                // theo IP, còn hơn là ô thời tiết trống trơn.
                None if !wanted.is_empty() => {
                    let found = locate(client).await?;
                    cache.lock().await.place =
                        Some((Instant::now(), wanted.to_string(), found.clone()));
                    found
                }
                None => return stale(cache).await,
            },
        },
    };

    // Lượt mới hỏng thì dùng lại số liệu cũ chứ không để ô trống. Đo thực tế:
    // chạy kiểm thử hai lần liên tiếp với cùng mã nguồn thì lần đầu dịch vụ
    // không trả lời, lần sau bình thường — nhiệt độ 15 phút trước vẫn đúng
    // hơn nhiều so với không hiện gì.
    match current(client, &place).await {
        Some(weather) => {
            cache.lock().await.weather = Some((Instant::now(), weather.clone()));
            Some(weather)
        }
        None => stale(cache).await,
    }
}

/// Số liệu đã lấy lần trước, dù đã quá hạn.
async fn stale(cache: &tokio::sync::Mutex<Cache>) -> Option<Weather> {
    cache.lock().await.weather.as_ref().map(|(_, w)| w.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chạm mạng thật nên bị bỏ qua ở lần chạy thường.
    /// Chạy bằng: `cargo test -- --ignored --nocapture thoi_tiet`
    #[tokio::test]
    #[ignore]
    async fn lay_duoc_thoi_tiet_that() {
        let client = crate::fetcher::client().expect("tạo client");
        let cache = tokio::sync::Mutex::new(Cache::default());

        let first = fetch(&client, &cache, "").await.expect("phải lấy được thời tiết");
        println!("  {} · {}°C · mã {} · {}", first.place, first.temp_c, first.code,
                 if first.is_day { "ngày" } else { "đêm" });
        assert!((-60..=60).contains(&first.temp_c), "nhiệt độ vô lý: {}", first.temp_c);
        assert!(first.code <= 99, "mã WMO vô lý: {}", first.code);
        assert!(!first.place.is_empty());

        // Lượt thứ hai phải lấy từ bộ đệm, không gọi mạng lại.
        let started = Instant::now();
        let second = fetch(&client, &cache, "").await.expect("lượt thứ hai");
        println!("  lượt hai mất {:?}", started.elapsed());
        assert_eq!(second.temp_c, first.temp_c);
        assert!(started.elapsed() < Duration::from_millis(50), "lượt hai không dùng bộ đệm");
    }

    /// Đặt thành phố trong cài đặt thì phải lấy đúng thành phố đó.
    ///
    /// Chạm mạng thật nên bị bỏ qua ở lần chạy thường.
    #[tokio::test]
    #[ignore]
    async fn lay_dung_thanh_pho_nguoi_dung_dat() {
        let client = crate::fetcher::client().expect("tạo client");
        let cache = tokio::sync::Mutex::new(Cache::default());

        let hn = fetch(&client, &cache, "Hà Nội").await.expect("Hà Nội");
        println!("  Hà Nội  → {} · {}°C", hn.place, hn.temp_c);
        assert_eq!(hn.place, "Hà Nội");

        // Đổi thành phố phải làm mất hiệu lực bộ đệm ngay, không đợi hết hạn.
        let dn = fetch(&client, &cache, "Đà Nẵng").await.expect("Đà Nẵng");
        println!("  Đà Nẵng → {} · {}°C", dn.place, dn.temp_c);
        assert_eq!(dn.place, "Đà Nẵng");
        assert_ne!(dn.place, hn.place, "đổi thành phố mà vẫn trả về nơi cũ");
    }

    /// Gõ tên không tra được thì lùi về tự dò theo IP, không để ô trống.
    #[tokio::test]
    #[ignore]
    async fn ten_khong_tra_duoc_thi_lui_ve_tu_do() {
        let client = crate::fetcher::client().expect("tạo client");
        let cache = tokio::sync::Mutex::new(Cache::default());
        let ra = fetch(&client, &cache, "zzzqqqxxx không có thật").await;
        println!("  → {:?}", ra.as_ref().map(|w| w.place.clone()));
        assert!(ra.is_some(), "phải lùi về tự dò theo IP");
    }

    /// Bộ đệm hết hạn mà dịch vụ hỏng thì vẫn phải trả số liệu cũ.
    ///
    /// Dùng client trỏ vào cổng không ai nghe để ép mọi lượt gọi mạng hỏng.
    #[tokio::test]
    async fn hong_mang_thi_dung_lai_so_lieu_cu() {
        let cu = Weather { temp_c: 27, code: 3, is_day: true, place: "Hà Nội".into() };
        let cache = tokio::sync::Mutex::new(Cache {
            // Đặt mốc thời gian đã quá hạn để buộc đi lấy lượt mới.
            place: Some((Instant::now() - PLACE_TTL * 2, String::new(), Place {
                lat: 21.03, lon: 105.85, name: "Hà Nội".into(),
            })),
            weather: Some((Instant::now() - WEATHER_TTL * 2, cu.clone())),
        });
        // Không dùng timeout ngắn để ép hỏng: current() đặt .timeout() cho
        // từng lượt gọi, và mức đó đè lên mức của client, nên lượt gọi vẫn
        // thành công và kiểm thử hoá ra đang đo dữ liệu thật.
        // Trỏ thẳng tên miền vào cổng không ai nghe thì chắc chắn hỏng.
        let hong = reqwest::Client::builder()
            .resolve("api.open-meteo.com", ([127, 0, 0, 1], 9).into())
            .resolve("ipwho.is", ([127, 0, 0, 1], 9).into())
            .no_proxy()
            .build()
            .expect("client");

        let ra = fetch(&hong, &cache, "").await.expect("phải trả lại số liệu cũ");
        assert_eq!(ra.temp_c, cu.temp_c);
        assert_eq!(ra.place, cu.place);
    }
}
