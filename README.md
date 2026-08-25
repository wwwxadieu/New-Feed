# News Feed

Ứng dụng đọc tin công nghệ dạng dashboard cho Windows. Thay vì xếp bài theo dòng
thời gian, ứng dụng **gộp các bài viết nói về cùng một sự kiện thành một cụm**,
rồi cho đọc bài đã bóc tách sạch quảng cáo.

## Ý tưởng cốt lõi

Đơn vị hiển thị là **cụm sự kiện**, không phải bài báo lẻ. Khi 12 tờ báo cùng
đưa một tin, dashboard hiện một thẻ với dòng "12 nguồn", không phải 12 dòng
trùng nhau. Đó là lý do dashboard là dạng giao diện đúng cho ứng dụng này —
nó cho thấy thứ mà một danh sách một cột không diễn đạt được.

## Công cụ và lý do chọn

| Thành phần | Lựa chọn | Lý do |
| --- | --- | --- |
| Khung ứng dụng | **Tauri v2** | Bản chạy đã đo được **8,2 MB** (xem mục dưới), dùng WebView2 có sẵn trong Windows. Electron cho cùng chức năng nặng ~120–150 MB. |
| Backend | **Rust** | Tải nguồn, bóc tách HTML và gộp cụm chạy native, không chặn giao diện. |
| Giao diện | **React 19 + TypeScript + Vite** | Bundle nhỏ, dựng lại nhanh, kiểu chặt. |
| Phông chữ | **Inter Variable** (đóng gói kèm) | Hỗ trợ đầy đủ dấu tiếng Việt, tải từ trong ứng dụng nên chạy được offline. |
| Hiệu ứng | CSS thuần | Không thêm thư viện animation nào — bundle giữ ở mức ~72 KB gzip. |

## Chạy thử

```bash
npm install

# Xem giao diện trong trình duyệt với dữ liệu mẫu (không cần Rust)
npm run dev

# Chạy ứng dụng desktop thật, có tải tin và bóc tách bài
npm run app:dev
```

`npm run dev` mở ở <http://localhost:1420> với dữ liệu minh hoạ. Chức năng thêm
nguồn và đọc bài thật chỉ hoạt động ở chế độ desktop.

## Dựng bản cài Windows

**Cách 1 — trên máy Windows.** Cần [Rust](https://rustup.rs) và
[Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
(chọn "Desktop development with C++"):

```bash
npm install
npm run app:build
```

### Kích thước đã đo

Bản release dựng trên Linux (`opt-level = "s"`, LTO, `panic = "abort"`, đã
strip symbol) cho ra bản chạy **8,2 MB** đã bao gồm toàn bộ giao diện nhúng
sẵn bên trong. Bản Windows thường nhỏ hơn con số này vì không phải liên kết
GTK/WebKit — WebView2 đã có sẵn trong hệ điều hành.

Kết quả nằm ở `src-tauri/target/release/bundle/`:

- `nsis/News Feed_0.1.0_x64-setup.exe` — bộ cài
- `msi/News Feed_0.1.0_x64_en-US.msi` — gói MSI
- `News Feed.exe` — bản chạy trực tiếp

**Cách 2 — không cần cài gì.** Đẩy nhánh lên GitHub, workflow
`.github/workflows/build-windows.yml` sẽ tự dựng trên máy ảo Windows và đính
kèm bộ cài vào phần Artifacts của lần chạy đó.

## Cách hoạt động

```
Thêm nguồn ──► dò feed ──► đọc feed ──► gộp cụm ──► dashboard
   (URL bất kỳ)   (tự tìm RSS)  (feed-rs)   (Jaccard có trọng số)
                                                       │
                                            mở một cụm ─┴──► tải trang bài
                                                              ──► bóc tách
                                                              ──► đọc sạch
```

**Dò feed.** Dán địa chỉ trang chủ là đủ; ứng dụng tìm thẻ
`<link rel="alternate">`, nếu không có thì thử các đường dẫn quen thuộc
(`/feed`, `/rss.xml`, …). Dán thẳng địa chỉ RSS cũng được.

**Gộp cụm** (`src-tauri/src/cluster.rs`). Tách tiêu đề thành từ, bỏ từ dừng
tiếng Việt và tiếng Anh, tính Jaccard có trọng số — từ càng hiếm trong toàn bộ
kho tin thì càng nặng, nên tên riêng quyết định việc gộp. Hai bài cách nhau quá
72 giờ không gộp dù tiêu đề giống nhau.

**Bóc tách** (`src-tauri/src/extract.rs`). Xoá thẻ script/iframe/nav/footer, xoá
phần tử có lớp hoặc id khớp mẫu quảng cáo, popup và tường thu phí, rồi chọn
khối có mật độ chữ cao nhất làm thân bài. Ảnh được lấy cả từ `data-src` và
`srcset`. **Số khối bị loại bỏ hiện trên màn hình đọc là số đếm thật**, không
phải con số minh hoạ.

**Lưu trữ.** Toàn bộ dữ liệu nằm trong `%APPDATA%/app.newsfeed.desktop/state.json`
trên máy người dùng. Không có máy chủ, không có tài khoản, không gửi dữ liệu đi
đâu cả.

## Cấu trúc

```
src/                    Giao diện React
  components/           Các thành phần màn hình
  lib/                  Kiểu dữ liệu, gọi API, chủ đề sáng/tối
  styles/               Token thiết kế và CSS
src-tauri/src/
  lib.rs                Các lệnh Tauri, trạng thái ứng dụng
  fetcher.rs            Dò feed, tải nguồn, tải bài
  extract.rs            Bóc tách nội dung, loại quảng cáo
  cluster.rs            Gộp cụm sự kiện, phân loại chủ đề
  store.rs              Lưu/đọc trạng thái ra đĩa
```

## Về bản quyền nội dung

Ứng dụng tải và hiển thị toàn văn bài viết sau khi loại quảng cáo. Với sử dụng
cá nhân thì đây là chuyện bình thường, tương đương chế độ Reader của trình
duyệt. Nếu định phát hành rộng rãi, cần cân nhắc: hiển thị toàn văn bài có bản
quyền đồng thời loại bỏ phần tạo doanh thu của toà soạn là rủi ro pháp lý thực.
Hướng an toàn hơn là bóc tách toàn văn ở phía sau để phục vụ gộp cụm và tóm
tắt, còn phía trước chỉ hiện tóm tắt tổng hợp kèm link về bài gốc.

## Kiểm thử

```bash
# Kiểm thử logic bóc tách và gộp cụm (không cần mạng)
cargo test --manifest-path src-tauri/Cargo.toml

# Kiểm thử đường ống thật với trang tin thật (có chạm mạng)
cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture
```

Kiểm thử chạm mạng chạy trọn vòng với ba trang thật (VnExpress, GenK, Ars
Technica): dò feed từ địa chỉ trang chủ, đọc feed, tải một bài rồi bóc tách,
và in ra số từ, số ảnh cùng số khối quảng cáo đã loại bỏ.
