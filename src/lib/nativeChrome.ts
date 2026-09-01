/**
 * Gỡ những chỗ WebView tự lộ ra là trình duyệt.
 *
 * Ứng dụng chạy trong WebView2, nên nó thừa hưởng luôn menu chuột phải của
 * trình duyệt: "Tải lại", "Lưu thành...", "In", "Quay lại". Không mục nào có
 * nghĩa với một trình đọc tin, mà lại nói toạc ra rằng bên trong là một trang
 * web — cửa sổ đang cố trông như ứng dụng Windows thì hỏng ngay ở đó.
 */

/** Những chỗ vẫn giữ menu: ô nhập chữ. */
const EDITABLE = "input, textarea, [contenteditable='true']";

/**
 * Chặn menu chuột phải mặc định.
 *
 * Riêng ô nhập chữ thì để nguyên: ở đó menu hiện ra là menu soạn thảo của
 * Windows (Hoàn tác, Cắt, Dán) chứ không phải menu trình duyệt, và đó đúng
 * là thứ người dùng mong đợi khi bấm chuột phải vào một ô nhập.
 */
export function suppressWebviewContextMenu() {
  window.addEventListener("contextmenu", (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.closest?.(EDITABLE)) return;
    event.preventDefault();
  });
}
