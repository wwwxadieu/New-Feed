// Ẩn cửa sổ console đen của Windows ở bản release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    news_feed_lib::run()
}
