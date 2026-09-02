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
    place: Option<(Instant, Place)>,
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
pub async fn fetch(client: &reqwest::Client, cache: &tokio::sync::Mutex<Cache>) -> Option<Weather> {
    {
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
        let any = guard.place.as_ref().map(|(_, p)| p.clone());
        let fresh = guard
            .place
            .as_ref()
            .filter(|(at, _)| at.elapsed() < PLACE_TTL)
            .map(|(_, p)| p.clone());
        (fresh, any)
    };

    // Hạn vị trí hết mà dò lại hỏng thì vẫn dùng vị trí cũ: hạn ở đây chỉ để
    // thỉnh thoảng làm mới, chứ nơi ở của người dùng không đổi sau vài tiếng.
    let place = match fresh_place {
        Some(place) => place,
        None => match locate(client).await {
            Some(found) => {
                cache.lock().await.place = Some((Instant::now(), found.clone()));
                found
            }
            None => match any_place {
                Some(cu) => cu,
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

        let first = fetch(&client, &cache).await.expect("phải lấy được thời tiết");
        println!("  {} · {}°C · mã {} · {}", first.place, first.temp_c, first.code,
                 if first.is_day { "ngày" } else { "đêm" });
        assert!((-60..=60).contains(&first.temp_c), "nhiệt độ vô lý: {}", first.temp_c);
        assert!(first.code <= 99, "mã WMO vô lý: {}", first.code);
        assert!(!first.place.is_empty());

        // Lượt thứ hai phải lấy từ bộ đệm, không gọi mạng lại.
        let started = Instant::now();
        let second = fetch(&client, &cache).await.expect("lượt thứ hai");
        println!("  lượt hai mất {:?}", started.elapsed());
        assert_eq!(second.temp_c, first.temp_c);
        assert!(started.elapsed() < Duration::from_millis(50), "lượt hai không dùng bộ đệm");
    }

    /// Bộ đệm hết hạn mà dịch vụ hỏng thì vẫn phải trả số liệu cũ.
    ///
    /// Dùng client trỏ vào cổng không ai nghe để ép mọi lượt gọi mạng hỏng.
    #[tokio::test]
    async fn hong_mang_thi_dung_lai_so_lieu_cu() {
        let cu = Weather { temp_c: 27, code: 3, is_day: true, place: "Hà Nội".into() };
        let cache = tokio::sync::Mutex::new(Cache {
            // Đặt mốc thời gian đã quá hạn để buộc đi lấy lượt mới.
            place: Some((Instant::now() - PLACE_TTL * 2, Place {
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

        let ra = fetch(&hong, &cache).await.expect("phải trả lại số liệu cũ");
        assert_eq!(ra.temp_c, cu.temp_c);
        assert_eq!(ra.place, cu.place);
    }
}
