import { useEffect, type RefObject } from "react";

/** Độ bám của chuyển động cuộn: càng nhỏ càng trôi lâu. */
const EASE = 0.16;
/** Kéo quá mép tối đa bao nhiêu pixel. */
const OVERSCROLL_MAX = 92;
/** Mỗi khung hình, độ kéo quá co lại bấy nhiêu lần. */
const OVERSCROLL_DECAY = 0.85;

/**
 * Cuộn mượt có độ nảy ở hai mép.
 *
 * WebView trên Windows không có hiệu ứng nảy như macOS, nên phải tự làm:
 * chặn sự kiện lăn chuột, nội suy dần vị trí cuộn qua từng khung hình, và
 * khi người dùng lăn quá mép thì đẩy lớp nội dung ra rồi cho bật về.
 *
 * Thanh cuộn kéo tay và phím điều hướng vẫn chạy theo cơ chế gốc của trình
 * duyệt; hàm này chỉ đồng bộ lại vị trí khi điều đó xảy ra.
 */
export function useElasticScroll(
  outerRef: RefObject<HTMLElement | null>,
  innerRef: RefObject<HTMLElement | null>,
) {
  useEffect(() => {
    const outer = outerRef.current;
    const inner = innerRef.current;
    if (!outer || !inner) return;
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) return;

    let target = outer.scrollTop;
    let current = target;
    let over = 0;
    let frame = 0;

    const maxScroll = () => Math.max(0, outer.scrollHeight - outer.clientHeight);

    const tick = () => {
      frame = 0;
      // Nội dung có thể ngắn đi sau khi đổi bộ lọc.
      target = Math.min(target, maxScroll());

      const distance = target - current;
      current = Math.abs(distance) < 0.4 ? target : current + distance * EASE;

      outer.scrollTop = current;

      over *= OVERSCROLL_DECAY;
      if (Math.abs(over) < 0.3) over = 0;
      inner.style.transform = over === 0 ? "" : `translate3d(0, ${over.toFixed(2)}px, 0)`;

      if (current !== target || over !== 0) frame = requestAnimationFrame(tick);
    };

    const wake = () => {
      if (!frame) frame = requestAnimationFrame(tick);
    };

    const onWheel = (event: WheelEvent) => {
      // Ctrl + lăn là thao tác phóng to, không phải cuộn.
      if (event.ctrlKey) return;
      const max = maxScroll();
      if (max <= 0) return;
      event.preventDefault();

      const next = target + event.deltaY;
      // Càng kéo ra xa mép thì càng nặng tay, giống cách iOS ghì lại.
      const resistance = 0.32 / (1 + Math.abs(over) / 70);

      if (next < 0) {
        over = Math.min(OVERSCROLL_MAX, over - next * resistance);
        target = 0;
      } else if (next > max) {
        over = Math.max(-OVERSCROLL_MAX, over - (next - max) * resistance);
        target = max;
      } else {
        target = next;
      }
      wake();
    };

    const onScroll = () => {
      // Trình duyệt bắn sự kiện scroll bất đồng bộ, nên không thể dùng một cờ
      // bật tắt quanh lệnh gán để nhận ra cuộn nào là của mình. Thay vào đó
      // so vị trí thật với vị trí vòng lặp vừa đặt: lệch nhiều nghĩa là người
      // dùng kéo thanh cuộn hoặc bấm phím, lúc đó mới đồng bộ lại.
      const drifted = Math.abs(outer.scrollTop - current) > 2;
      if (frame !== 0 && !drifted) return;
      current = outer.scrollTop;
      target = current;
    };

    outer.addEventListener("wheel", onWheel, { passive: false });
    outer.addEventListener("scroll", onScroll, { passive: true });

    return () => {
      outer.removeEventListener("wheel", onWheel);
      outer.removeEventListener("scroll", onScroll);
      if (frame) cancelAnimationFrame(frame);
      inner.style.transform = "";
    };
  }, [outerRef, innerRef]);
}
