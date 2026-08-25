mod cluster;
mod extract;
mod fetcher;
mod model;
mod store;

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
    data.sources.push(source);
    merge_articles(&mut data, articles);

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

    let results: Vec<(String, Result<Vec<Article>, String>)> = stream::iter(targets)
        .map(|source| {
            let client = client.clone();
            let app = app.clone();
            let done = done.clone();
            async move {
                let outcome = fetcher::fetch_source(&client, &source, limit).await;
                let finished = done.fetch_add(1, Ordering::SeqCst) + 1;
                let _ = app.emit(
                    "refresh:progress",
                    serde_json::json!({ "done": finished, "total": total, "source": source.title }),
                );
                (source.id, outcome)
            }
        })
        .buffer_unordered(FETCH_CONCURRENCY)
        .collect()
        .await;

    let mut data = state.data.lock().await;
    let now = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    let mut incoming = Vec::new();

    for (source_id, outcome) in results {
        let Some(source) = data.sources.iter_mut().find(|s| s.id == source_id) else {
            continue;
        };
        match outcome {
            Ok(articles) => {
                source.last_fetched = Some(now.clone());
                source.last_error = None;
                incoming.extend(articles);
            }
            Err(message) => source.last_error = Some(message),
        }
    }

    merge_articles(&mut data, incoming);
    data.last_refresh = Some(now);
    store::save(&app, &data)?;
    Ok(snapshot(&data))
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
            read_article
        ])
        .run(tauri::generate_context!())
        .expect("không khởi động được ứng dụng");
}
