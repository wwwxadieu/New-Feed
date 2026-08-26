mod cluster;
mod extract;
mod fetcher;
mod model;
mod store;
mod translate;

use chrono::{DateTime, SecondsFormat, Utc};
use futures::stream::{self, StreamExt};
use model::{stable_id, AppData, Article, CleanedArticle, Settings, Snapshot, Source};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

/// Số bài giữ lại trong kho cục bộ, đủ cho vài ngày tin mà không phình file.
const MAX_ARTICLES: usize = 1200;
/// Số nguồn tải song song. Đủ nhanh nhưng không làm nghẽn đường truyền.
const FETCH_CONCURRENCY: usize = 6;
/// Số bài được bù ảnh mỗi lượt. Chỉ bù cho tin mới nhất để lượt làm mới
/// không kéo dài; bài cũ hơn sẽ được bù ở các lượt sau.
const IMAGE_BACKFILL_LIMIT: usize = 60;
const IMAGE_BACKFILL_CONCURRENCY: usize = 8;
/// Hạn ngạch ký tự cho mỗi lượt dịch. Dịch vụ cho 5.000 ký tự mỗi ngày khi
/// dùng ẩn danh và 50.000 khi có khai báo email; chừa lại một khoảng đệm.
const TRANSLATE_BUDGET_ANON: usize = 4_500;
const TRANSLATE_BUDGET_WITH_EMAIL: usize = 45_000;
/// Gọi dồn dập dễ bị dịch vụ chặn tạm, nên giữ mức song song thấp.
const TRANSLATE_CONCURRENCY: usize = 3;

pub struct AppState {
    data: Mutex<AppData>,
    client: reqwest::Client,
}

fn parse_time(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn snapshot(data: &AppData) -> Snapshot {
    let clusters = cluster::build(&data.articles);

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

/// Danh sách bài mới nhất còn thiếu ảnh.
fn articles_missing_images(data: &AppData) -> Vec<(String, String)> {
    data.articles
        .iter()
        .filter(|a| a.image.is_none())
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

    let targets = articles_missing_images(&data);
    drop(data);
    let found = backfill_images(&state.client, targets).await;

    let mut data = state.data.lock().await;
    apply_images(&mut data, found);
    let pending = translation_targets(&data);
    drop(data);
    let (translated, notice) = run_translations(&state.client, pending.1, pending.0).await;

    let mut data = state.data.lock().await;
    apply_translations(&mut data, translated);
    data.translate_notice = notice;
    store::save(&app, &data)?;
    Ok(snapshot(&data))
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

    type FetchResult = (String, Result<Vec<Article>, String>, Option<String>);

    let results: Vec<FetchResult> = stream::iter(targets)
        .map(|source| {
            let client = client.clone();
            let app = app.clone();
            let done = done.clone();
            async move {
                let outcome = fetcher::fetch_source(&client, &source, limit).await;
                // Nguồn nào chưa có logo thì lấy luôn trong cùng lượt, để nguồn
                // mặc định và nguồn thêm lỗi lần trước đều có huy hiệu.
                let logo = match source.logo {
                    Some(_) => None,
                    None => fetcher::fetch_logo(&client, &source.home_url).await,
                };
                let finished = done.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "refresh:progress",
                    serde_json::json!({ "done": finished, "total": total, "source": source.title }),
                );
                (source.id, outcome, logo)
            }
        })
        .buffer_unordered(FETCH_CONCURRENCY)
        .collect()
        .await;

    let mut data = state.data.lock().await;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut incoming = Vec::new();

    for (source_id, outcome, logo) in results {
        let Some(source) = data.sources.iter_mut().find(|s| s.id == source_id) else {
            continue;
        };
        if logo.is_some() {
            source.logo = logo;
        }
        match outcome {
            Ok(articles) => {
                source.last_fetched = Some(now.clone());
                source.last_error = None;
                // Nhận diện lại mỗi lượt để nguồn đổi ngôn ngữ vẫn theo kịp.
                source.language = Some(detect_language(&articles));
                incoming.extend(articles);
            }
            Err(message) => source.last_error = Some(message),
        }
    }

    merge_articles(&mut data, incoming);
    data.last_refresh = Some(now);

    // Thả khoá trước khi đi tải ảnh, để thao tác của người dùng không phải chờ.
    let targets = articles_missing_images(&data);
    drop(data);
    let found = backfill_images(&state.client, targets).await;

    let mut data = state.data.lock().await;
    apply_images(&mut data, found);
    let pending = translation_targets(&data);
    drop(data);
    let (translated, notice) = run_translations(&state.client, pending.1, pending.0).await;

    let mut data = state.data.lock().await;
    apply_translations(&mut data, translated);
    data.translate_notice = notice;
    store::save(&app, &data)?;
    Ok(snapshot(&data))
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
    fetcher::fetch_article(&state.client, &url).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let mut data = store::load(&handle);

            // Lần chạy đầu: nạp sẵn vài nguồn công nghệ để giao diện có nội dung.
            if data.sources.is_empty() {
                let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
                data.sources = store::default_sources()
                    .into_iter()
                    .map(|(title, home, feed)| Source {
                        id: stable_id(feed),
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
                    })
                    .collect();
                let _ = store::save(&handle, &data);
            }

            let client = fetcher::client().map_err(|e| std::io::Error::other(e))?;
            app.manage(AppState { data: Mutex::new(data), client });
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
