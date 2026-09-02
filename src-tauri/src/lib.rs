mod cluster;
mod embedded;
mod extract;
mod fetcher;
mod model;
mod store;
mod thumbs;
mod translate;

use chrono::{DateTime, SecondsFormat, Utc};
use futures::stream::{self, StreamExt};
use model::{stable_id, AppData, Article, CleanedArticle, Settings, Snapshot, Source};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// Số bài giữ lại trong kho cục bộ, đủ cho vài ngày tin mà không phình file.
const MAX_ARTICLES: usize = 1200;
/// Số nguồn tải song song. Feed là tệp XML nhỏ và phần lớn thời gian là chờ
/// máy chủ trả lời, nên tải rộng tay hơn số lõi máy vẫn không nặng CPU.
const FETCH_CONCURRENCY: usize = 12;
/// Số bài được bù ảnh mỗi lượt. Chỉ bù cho tin mới nhất để lượt làm mới
/// không kéo dài; bài cũ hơn sẽ được bù ở các lượt sau.
const IMAGE_BACKFILL_LIMIT: usize = 60;
/// Số cụm đầu bảng được tải sẵn ảnh cỡ lớn.
///
/// Giao diện chỉ hiện một tin hero và ba tin đặc tả, nhưng người dùng còn
/// đổi cách sắp xếp và lọc theo chủ đề hay nguồn, nên cụm nào lên đầu là
/// thay đổi được. Mười sáu cụm phủ hết các lựa chọn thường gặp mà vẫn xa
/// mức nhân đôi cả thư mục đệm.
const HERO_IMAGE_LIMIT: usize = 16;
const IMAGE_BACKFILL_CONCURRENCY: usize = 8;
/// Lấy logo phải tải cả trang chủ của báo nên nặng hơn tải feed, giữ tay hơn.
const LOGO_CONCURRENCY: usize = 4;
/// Khoảng cách tối thiểu giữa hai lần đẩy ảnh chụp sang giao diện trong lúc
/// đang làm mới.
const EMIT_EVERY: std::time::Duration = std::time::Duration::from_millis(700);
/// Hạn ngạch ký tự cho mỗi lượt dịch. Dịch vụ cho 5.000 ký tự mỗi ngày khi
/// dùng ẩn danh và 50.000 khi có khai báo email; chừa lại một khoảng đệm.
const TRANSLATE_BUDGET_ANON: usize = 4_500;
const TRANSLATE_BUDGET_WITH_EMAIL: usize = 45_000;
/// Gọi dồn dập dễ bị dịch vụ chặn tạm, nên giữ mức song song thấp.
const TRANSLATE_CONCURRENCY: usize = 3;
/// Bóc tách ra ít hơn ngần này từ thì coi như hụt, chuyển sang dùng nội dung
/// của feed. Có nguồn dựng bài bằng JavaScript nên trang gốc không chứa chữ.
const MIN_ARTICLE_WORDS: usize = 120;

pub struct AppState {
    data: Mutex<AppData>,
    client: reqwest::Client,
    /// Đang có một lượt bổ sung nền chạy hay không. Hai lượt chồng nhau sẽ
    /// tải trùng ảnh và đốt hai lần hạn mức dịch.
    enriching: AtomicBool,
    /// Có tin mới về trong lúc lượt bổ sung đang chạy, cần chạy lại một vòng.
    enrich_again: AtomicBool,
}

fn parse_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn snapshot(data: &AppData) -> Snapshot {
    let mut clusters = cluster::build(&data.articles);

    // Thân bài lấy từ feed chỉ dùng làm dự phòng cho màn hình đọc, giao diện
    // không đụng tới. Gửi kèm thì mỗi ảnh chụp phình lên vài megabyte JSON,
    // và việc tuần tự hoá rồi phân tích lại chỗ đó là phần chậm nhất của
    // mỗi lần gọi lệnh.
    for cluster in &mut clusters {
        for article in &mut cluster.articles {
            article.content_html = None;
        }
    }

    let mut topic_counts: HashMap<String, usize> = HashMap::new();
    for c in &clusters {
        *topic_counts.entry(c.topic.clone()).or_insert(0) += c.articles.len();
    }
    let mut topic_counts: Vec<(String, usize)> = topic_counts.into_iter().collect();
    topic_counts.sort_by(|a, b| b.1.cmp(&a.1));

    // Số bài theo từng giờ trong 24 giờ gần nhất, dùng cho biểu đồ nhịp tin.
    let now = Utc::now();
    let mut hourly = vec![0usize; 24];
    for article in &data.articles {
        let age = (now - parse_time(&article.published)).num_hours();
        if (0..24).contains(&age) {
            hourly[(23 - age) as usize] += 1;
        }
    }

    Snapshot {
        sources: data.sources.clone(),
        clusters,
        settings: data.settings.clone(),
        article_count: data.articles.len(),
        topic_counts,
        hourly,
        last_refresh: data.last_refresh.clone(),
        translate_notice: data.translate_notice.clone(),
    }
}

fn merge_articles(data: &mut AppData, incoming: Vec<Article>) {
    let mut by_id: HashMap<String, Article> = data.articles.drain(..).map(|a| (a.id.clone(), a)).collect();
    for article in incoming {
        by_id.insert(article.id.clone(), article);
    }
    let mut merged: Vec<Article> = by_id.into_values().collect();
    merged.sort_by(|a, b| parse_time(&b.published).cmp(&parse_time(&a.published)));
    merged.truncate(MAX_ARTICLES);

    let mut counts: HashMap<String, usize> = HashMap::new();
    for article in &merged {
        *counts.entry(article.source_id.clone()).or_insert(0) += 1;
    }
    for source in &mut data.sources {
        source.article_count = counts.get(&source.id).copied().unwrap_or(0);
    }
    data.articles = merged;
}

/// Bù ảnh cho những bài mà feed không kèm ảnh, bằng cách đọc og:image của
/// trang bài. Nhận và trả về theo id để không phải giữ khoá trong lúc tải.
async fn backfill_images(
    client: &reqwest::Client,
    targets: Vec<(String, String)>,
) -> Vec<(String, String)> {
    stream::iter(targets)
        .map(|(id, url)| {
            let client = client.clone();
            async move { fetcher::fetch_og_image(&client, &url).await.map(|image| (id, image)) }
        })
        .buffer_unordered(IMAGE_BACKFILL_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Bài đại diện của những cụm đầu bảng, cần bản ảnh lớn cho tin hero.
///
/// Ảnh lưới rộng 480px, còn ô ảnh của hero rộng khoảng 700px CSS tức 1400
/// điểm ảnh thật trên màn mật độ cao. Dùng bản lưới ở đó thì mờ thấy rõ.
fn hero_targets(data: &AppData) -> Vec<(String, String)> {
    let clusters = cluster::build(&data.articles);
    let leading: std::collections::HashSet<&str> = clusters
        .iter()
        .take(HERO_IMAGE_LIMIT)
        .filter_map(|c| c.articles.iter().find(|a| a.image.is_some()))
        .map(|a| a.id.as_str())
        .collect();

    data.articles
        .iter()
        .filter(|a| a.hero.is_none() && leading.contains(a.id.as_str()))
        .filter_map(|a| a.image.clone().map(|url| (a.id.clone(), url)))
        .collect()
}

fn apply_heroes(data: &mut AppData, done: Vec<(String, String)>) {
    for (id, path) in done {
        if let Some(article) = data.articles.iter_mut().find(|a| a.id == id) {
            article.hero = Some(path);
        }
    }
}

/// Bài đã có địa chỉ ảnh nhưng chưa tải về máy.
fn thumbs_to_cache(data: &AppData) -> Vec<(String, String)> {
    data.articles
        .iter()
        .filter(|a| a.thumb.is_none())
        .filter_map(|a| a.image.clone().map(|url| (a.id.clone(), url)))
        .collect()
}

fn apply_thumbs(data: &mut AppData, done: Vec<(String, String)>) {
    for (id, path) in done {
        if let Some(article) = data.articles.iter_mut().find(|a| a.id == id) {
            article.thumb = Some(path);
        }
    }
}

/// Danh sách bài mới nhất còn thiếu ảnh và chưa từng đi tìm.
fn articles_missing_images(data: &AppData) -> Vec<(String, String)> {
    data.articles
        .iter()
        .filter(|a| a.image.is_none() && !a.image_checked)
        .take(IMAGE_BACKFILL_LIMIT)
        .map(|a| (a.id.clone(), a.url.clone()))
        .collect()
}

fn apply_images(data: &mut AppData, found: Vec<(String, String)>) {
    for (id, image) in found {
        if let Some(article) = data.articles.iter_mut().find(|a| a.id == id) {
            article.image = Some(image);
        }
    }
}

fn detect_language(articles: &[Article]) -> String {
    let titles: Vec<String> = articles.iter().map(|a| a.title.clone()).collect();
    if translate::is_vietnamese(&titles) { "vi" } else { "other" }.to_string()
}

/// Gom danh sách cần dịch kèm email đã cấu hình, hoặc rỗng nếu tắt tính năng.
fn translation_targets(data: &AppData) -> (Vec<(String, Field, String)>, String) {
    if !data.settings.translate {
        return (Vec::new(), String::new());
    }
    (translation_jobs(data), data.settings.translate_email.clone())
}

/// Ô văn bản cần dịch của một bài.
#[derive(Clone, Copy, PartialEq)]
enum Field {
    Title,
    Summary,
}

/// Gom việc dịch theo hạn ngạch ký tự.
///
/// Dịch vụ tính hạn mức theo số ký tự mỗi ngày chứ không theo số lần gọi,
/// nên cấp phát theo ký tự thay vì đếm số bài. Tiêu đề và tóm tắt của cùng
/// một bài đi liền nhau để thẻ tin không bị nửa Việt nửa Anh.
fn translation_jobs(data: &AppData) -> Vec<(String, Field, String)> {
    let budget = if data.settings.translate_email.contains('@') {
        TRANSLATE_BUDGET_WITH_EMAIL
    } else {
        TRANSLATE_BUDGET_ANON
    };

    let foreign: std::collections::HashSet<&str> = data
        .sources
        .iter()
        .filter(|s| s.language.as_deref() == Some("other"))
        .map(|s| s.id.as_str())
        .collect();

    let mut spent = 0usize;
    let mut jobs = Vec::new();

    for article in &data.articles {
        if !foreign.contains(article.source_id.as_str()) {
            continue;
        }
        let fields = [
            (Field::Title, &article.title, &article.title_vi),
            (Field::Summary, &article.summary, &article.summary_vi),
        ];
        for (field, text, existing) in fields {
            if existing.is_some() || text.trim().is_empty() {
                continue;
            }
            let cost = text.chars().count();
            if spent + cost > budget {
                return jobs;
            }
            spent += cost;
            jobs.push((article.id.clone(), field, text.clone()));
        }
    }
    jobs
}

/// Chạy các việc dịch. Trả về kết quả kèm thông báo nếu hết hạn mức.
async fn run_translations(
    client: &reqwest::Client,
    email: String,
    jobs: Vec<(String, Field, String)>,
) -> (Vec<(String, Field, String)>, Option<String>) {
    if jobs.is_empty() {
        return (Vec::new(), None);
    }
    let address = if email.contains('@') { Some(email) } else { None };

    let results: Vec<Result<(String, Field, String), translate::TranslateError>> = stream::iter(jobs)
        .map(|(id, field, text)| {
            let client = client.clone();
            let address = address.clone();
            async move {
                translate::to_vietnamese(&client, &text, address.as_deref())
                    .await
                    .map(|vi| (id, field, vi))
            }
        })
        .buffer_unordered(TRANSLATE_CONCURRENCY)
        .collect()
        .await;

    let mut done = Vec::new();
    let mut notice = None;
    for result in results {
        match result {
            Ok(item) => done.push(item),
            // Hết hạn mức là điều đáng báo; lỗi lẻ của một ô thì bỏ qua và
            // để lượt làm mới sau dịch lại.
            Err(translate::TranslateError::QuotaExhausted) => {
                notice = Some(translate::TranslateError::QuotaExhausted.to_string());
            }
            Err(_) => {}
        }
    }
    (done, notice)
}

fn apply_translations(data: &mut AppData, done: Vec<(String, Field, String)>) {
    for (id, field, vi) in done {
        let Some(article) = data.articles.iter_mut().find(|a| a.id == id) else {
            continue;
        };
        match field {
            Field::Title => article.title_vi = Some(vi),
            Field::Summary => article.summary_vi = Some(vi),
        }
    }
}

/// Nhãn cho biết lượt bổ sung nền đang làm gì, hoặc None khi đã xong.
fn emit_phase(app: &AppHandle, label: Option<&str>) {
    let _ = app.emit("enrich:phase", label);
}

/// Đẩy ảnh chụp mới sang giao diện sau khi bổ sung xong một bước.
fn emit_snapshot(app: &AppHandle, data: &AppData) {
    let _ = app.emit("snapshot:updated", snapshot(data));
}

/// Lấy huy hiệu cho những nguồn còn thiếu.
async fn fetch_logos(
    client: &reqwest::Client,
    targets: Vec<(String, String)>,
) -> Vec<(String, String)> {
    stream::iter(targets)
        .map(|(id, home)| {
            let client = client.clone();
            async move { fetcher::fetch_logo(&client, &home).await.map(|logo| (id, logo)) }
        })
        .buffer_unordered(LOGO_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Phần chậm của một lượt làm mới: huy hiệu nguồn, ảnh đại diện và bản dịch.
///
/// Ba việc này chiếm gần hết thời gian chờ nhưng không việc nào cần thiết để
/// đọc tin, nên chúng chạy sau khi lệnh `refresh` đã trả danh sách tin về.
/// Xong bước nào thì phát một ảnh chụp mới để giao diện tự điền vào chỗ trống.
async fn enrich(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let cache = thumbs::cache_dir(app)?;

    let missing_logos: Vec<(String, String)> = {
        let data = state.data.lock().await;
        data.sources
            .iter()
            .filter(|s| s.enabled && s.logo.is_none())
            .map(|s| (s.id.clone(), s.home_url.clone()))
            .collect()
    };
    if !missing_logos.is_empty() {
        emit_phase(app, Some("Đang lấy huy hiệu nguồn"));
        let found = fetch_logos(&state.client, missing_logos).await;
        if !found.is_empty() {
            let mut data = state.data.lock().await;
            for (id, logo) in found {
                if let Some(source) = data.sources.iter_mut().find(|s| s.id == id) {
                    source.logo = Some(logo);
                }
            }
            emit_snapshot(app, &data);
        }
    }

    let missing_images = articles_missing_images(&*state.data.lock().await);
    if !missing_images.is_empty() {
        emit_phase(app, Some("Đang tìm ảnh cho bài chưa có"));
        let attempted: std::collections::HashSet<String> =
            missing_images.iter().map(|(id, _)| id.clone()).collect();
        let found = backfill_images(&state.client, missing_images).await;

        let mut data = state.data.lock().await;
        apply_images(&mut data, found);
        // Đánh dấu đã thử kể cả khi không tìm ra: bài không kèm ảnh thì lượt
        // sau đọc lại trang của nó cũng vẫn không có, chỉ tốn thêm lượt tải.
        for article in data.articles.iter_mut().filter(|a| attempted.contains(&a.id)) {
            article.image_checked = true;
        }
    }

    let pending_thumbs = thumbs_to_cache(&*state.data.lock().await);
    if !pending_thumbs.is_empty() {
        emit_phase(app, Some("Đang tải ảnh về máy"));
        let cached = thumbs::ensure(&state.client, &cache, pending_thumbs).await;
        let mut data = state.data.lock().await;
        apply_thumbs(&mut data, cached);
        let _ = store::save(app, &data);
        emit_snapshot(app, &data);
    }

    // Bản ảnh lớn cho tin hero, sau ảnh lưới: lưới phải đầy trước đã, vì
    // ảnh lưới cũng đủ dùng cho hero trong lúc chờ, chỉ hơi mềm nét.
    let pending_heroes = hero_targets(&*state.data.lock().await);
    if !pending_heroes.is_empty() {
        emit_phase(app, Some("Đang tải ảnh lớn cho tin nổi bật"));
        let cached = thumbs::ensure_hero(&state.client, &cache, pending_heroes).await;
        let mut data = state.data.lock().await;
        apply_heroes(&mut data, cached);
        let _ = store::save(app, &data);
        emit_snapshot(app, &data);
    }

    let (jobs, email) = translation_targets(&*state.data.lock().await);
    if !jobs.is_empty() {
        emit_phase(app, Some("Đang dịch tiêu đề nước ngoài"));
        let (translated, notice) = run_translations(&state.client, email, jobs).await;
        let mut data = state.data.lock().await;
        apply_translations(&mut data, translated);
        data.translate_notice = notice;
        let _ = store::save(app, &data);
        emit_snapshot(app, &data);
    }

    let keep: std::collections::HashSet<String> = state
        .data
        .lock()
        .await
        .articles
        .iter()
        .map(|a| a.id.clone())
        .collect();
    thumbs::prune(&cache, &keep);
    Ok(())
}

/// Chạy lượt bổ sung nền. Đang có lượt khác thì chỉ ghi lại là còn việc mới,
/// để lượt đang chạy quay lại làm nốt — hai lượt song song sẽ tải trùng ảnh
/// và đốt hai lần hạn mức dịch.
fn spawn_enrichment(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        if state
            .enriching
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            state.enrich_again.store(true, Ordering::SeqCst);
            return;
        }

        loop {
            state.enrich_again.store(false, Ordering::SeqCst);
            if enrich(&app, &state).await.is_err() {
                break;
            }
            if !state.enrich_again.swap(false, Ordering::SeqCst) {
                break;
            }
        }

        state.enriching.store(false, Ordering::SeqCst);
        emit_phase(&app, None);
    });
}

#[tauri::command]
async fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    Ok(snapshot(&*state.data.lock().await))
}

#[tauri::command]
async fn add_source(app: AppHandle, state: State<'_, AppState>, input: String) -> Result<Snapshot, String> {
    if input.trim().is_empty() {
        return Err("Hãy nhập địa chỉ trang tin hoặc địa chỉ RSS.".into());
    }
    let source = fetcher::discover(&state.client, &input).await?;

    let mut data = state.data.lock().await;
    if data.sources.iter().any(|s| s.id == source.id) {
        return Err(format!("Nguồn \"{}\" đã có trong danh sách.", source.title));
    }

    let articles = fetcher::fetch_source(&state.client, &source, data.settings.max_per_source).await?;
    let mut source = source;
    source.last_fetched = Some(Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
    source.language = Some(detect_language(&articles));
    data.sources.push(source);
    merge_articles(&mut data, articles);

    store::save(&app, &data)?;
    let snap = snapshot(&data);
    drop(data);

    // Ảnh đại diện và bản dịch của nguồn mới được bổ sung ở nền: danh sách
    // bài hiện ra ngay, phần còn thiếu tự điền vào sau.
    spawn_enrichment(app);
    Ok(snap)
}

#[tauri::command]
async fn remove_source(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<Snapshot, String> {
    let mut data = state.data.lock().await;
    data.sources.retain(|s| s.id != id);
    data.articles.retain(|a| a.source_id != id);
    store::save(&app, &data)?;
    Ok(snapshot(&data))
}

#[tauri::command]
async fn set_source_enabled(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    enabled: bool,
) -> Result<Snapshot, String> {
    let mut data = state.data.lock().await;
    if let Some(source) = data.sources.iter_mut().find(|s| s.id == id) {
        source.enabled = enabled;
    }
    store::save(&app, &data)?;
    Ok(snapshot(&data))
}

#[tauri::command]
async fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<Snapshot, String> {
    let mut data = state.data.lock().await;
    data.settings = settings;
    store::save(&app, &data)?;
    Ok(snapshot(&data))
}

#[tauri::command]
async fn refresh(app: AppHandle, state: State<'_, AppState>) -> Result<Snapshot, String> {
    let (targets, limit) = {
        let data = state.data.lock().await;
        let targets: Vec<Source> = data.sources.iter().filter(|s| s.enabled).cloned().collect();
        (targets, data.settings.max_per_source)
    };

    if targets.is_empty() {
        return Err("Chưa có nguồn nào đang bật. Hãy thêm một nguồn tin trước.".into());
    }

    let total = targets.len();
    let done = Arc::new(AtomicUsize::new(0));
    let client = state.client.clone();

    // Giai đoạn này chỉ đọc feed. Logo, ảnh đại diện và bản dịch chiếm gần hết
    // thời gian chờ của một lượt làm mới nhưng không thứ nào cần để đọc tin,
    // nên chúng được dời sang lượt bổ sung nền ở cuối hàm.
    // Bản sao riêng cho dòng tải, để `app` gốc còn dùng được ở cuối hàm.
    let emitter = app.clone();
    let mut arriving = stream::iter(targets)
        .map(move |source| {
            let client = client.clone();
            let app = emitter.clone();
            let done = done.clone();
            async move {
                // Đọc feed trong một tác vụ riêng.
                //
                // Feed là dữ liệu lấy từ Internet, tức không kiểm soát được:
                // bộ đọc có thể hoảng giữa chừng vì một chuỗi dị dạng. Nằm
                // thẳng trong dòng này thì cú hoảng đó cuốn theo cả lượt làm
                // mới — không nguồn nào được ghi nhận, kể cả những nguồn đã
                // tải xong. Tách ra thì nó chỉ là lỗi của riêng nguồn đó.
                let reading = tauri::async_runtime::spawn({
                    let client = client.clone();
                    let source = source.clone();
                    async move { fetcher::fetch_source(&client, &source, limit).await }
                });
                let outcome = reading.await.unwrap_or_else(|_| {
                    Err(format!(
                        "Feed của {} có dữ liệu làm bộ đọc dừng giữa chừng.",
                        source.title
                    ))
                });

                let finished = done.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "refresh:progress",
                    serde_json::json!({ "done": finished, "total": total, "source": source.title }),
                );
                (source.id, outcome)
            }
        })
        .buffer_unordered(FETCH_CONCURRENCY);

    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut last_emit: Option<std::time::Instant> = None;

    // Ghi nhận từng nguồn ngay khi nó về, không đợi nguồn cuối cùng.
    //
    // Gom hết rồi mới ghi một lượt thì tin chỉ hiện ra sau nguồn chậm nhất,
    // và đóng ứng dụng giữa chừng là mất trắng cả những nguồn đã tải xong.
    while let Some((source_id, outcome)) = arriving.next().await {
        let mut data = state.data.lock().await;

        let mut fetched = None;
        {
            let Some(source) = data.sources.iter_mut().find(|s| s.id == source_id) else {
                continue;
            };
            match outcome {
                Ok(articles) => {
                    source.last_fetched = Some(now.clone());
                    source.last_error = None;
                    // Nhận diện lại mỗi lượt để nguồn đổi ngôn ngữ vẫn theo kịp.
                    source.language = Some(detect_language(&articles));
                    fetched = Some(articles);
                }
                Err(message) => source.last_error = Some(message),
            }
        }
        if let Some(articles) = fetched {
            merge_articles(&mut data, articles);
        }

        // Phát ảnh chụp thưa tay: dựng cụm cho cả kho tin không rẻ, mà mắt
        // người cũng không theo kịp ba mươi lần vẽ lại trong ba giây.
        let due = last_emit.is_none_or(|at| at.elapsed() >= EMIT_EVERY);
        if due {
            last_emit = Some(std::time::Instant::now());
            let _ = store::save(&app, &data);
            emit_snapshot(&app, &data);
        }
    }

    let mut data = state.data.lock().await;
    data.last_refresh = Some(now);
    store::save(&app, &data)?;
    let snap = snapshot(&data);
    drop(data);

    spawn_enrichment(app);
    Ok(snap)
}

/// Dịch một loạt đoạn văn theo yêu cầu, dùng cho nút dịch ở màn hình đọc.
#[tauri::command]
async fn translate_texts(state: State<'_, AppState>, texts: Vec<String>) -> Result<Vec<String>, String> {
    let email = { state.data.lock().await.settings.translate_email.clone() };
    let address = if email.contains('@') { Some(email) } else { None };

    // Gọi tuần tự: dừng ngay khi hết hạn mức thay vì đốt thêm lượt gọi.
    let mut out = Vec::with_capacity(texts.len());
    for text in texts {
        match translate::to_vietnamese(&state.client, &text, address.as_deref()).await {
            Ok(vi) => out.push(vi),
            Err(err) => return Err(err.to_string()),
        }
    }
    Ok(out)
}

#[tauri::command]
async fn read_article(state: State<'_, AppState>, url: String) -> Result<CleanedArticle, String> {
    let cleaned = fetcher::fetch_article(&state.client, &url).await?;
    if cleaned.word_count >= MIN_ARTICLE_WORDS {
        return Ok(cleaned);
    }

    let stored = {
        let data = state.data.lock().await;
        data.articles
            .iter()
            .find(|a| a.url == url)
            .and_then(|a| a.content_html.clone())
    };
    let Some(html) = stored.filter(|h| !h.trim().is_empty()) else {
        return Ok(cleaned);
    };
    let Ok(base) = url::Url::parse(&url) else {
        return Ok(cleaned);
    };

    let from_feed = extract::from_fragment(&html, &base);
    Ok(if from_feed.word_count > cleaned.word_count { from_feed } else { cleaned })
}

/// Nạp những nguồn mặc định chưa từng được nạp trên máy này.
/// Trả về true nếu danh sách nguồn có thay đổi.
///
/// Không chỉ chạy ở lần mở đầu tiên. Mỗi bản cập nhật có thể kèm thêm nguồn
/// mới, mà máy đã dùng từ trước thì kho nguồn không còn rỗng để nhận ra điều
/// đó — nên phải đối chiếu theo từng nguồn một.
fn seed_defaults(data: &mut AppData) -> bool {
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut changed = false;

    for (title, home, feed) in store::default_sources() {
        let id = stable_id(feed);
        if data.seeded_defaults.iter().any(|seen| seen == &id) {
            continue;
        }
        // Ghi nhận trước khi thêm: nguồn này đã được đề nghị một lần, người
        // dùng xoá đi thì lần mở sau không đề nghị lại nữa.
        data.seeded_defaults.push(id.clone());
        changed = true;

        if data.sources.iter().any(|s| s.id == id) {
            continue;
        }
        data.sources.push(Source {
            id,
            title: title.to_string(),
            home_url: home.to_string(),
            feed_url: feed.to_string(),
            enabled: true,
            added_at: now.clone(),
            last_fetched: None,
            last_error: None,
            article_count: 0,
            logo: None,
            language: None,
        });
    }
    changed
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let mut data = store::load(&handle);
            if seed_defaults(&mut data) {
                let _ = store::save(&handle, &data);
            }

            let client = fetcher::client().map_err(|e| std::io::Error::other(e))?;
            app.manage(AppState {
                data: Mutex::new(data),
                client,
                enriching: AtomicBool::new(false),
                enrich_again: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            add_source,
            remove_source,
            set_source_enabled,
            save_settings,
            refresh,
            read_article,
            translate_texts
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được ứng dụng");
}
