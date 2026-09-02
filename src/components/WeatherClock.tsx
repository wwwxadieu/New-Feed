import { useEffect, useState } from "react";
import * as api from "../lib/api";
import type { Weather } from "../lib/types";
import { WeatherIcon } from "./WeatherIcons";

/** Open-Meteo cập nhật mỗi 15 phút; hỏi lại mỗi 15 phút là vừa. */
const WEATHER_EVERY_MS = 15 * 60 * 1000;

/** Mô tả thời tiết theo mã WMO, chỉ dùng cho chú thích khi rê chuột. */
function describe(code: number): string {
  if (code === 0) return "Trời quang";
  if (code === 1) return "Ít mây";
  if (code === 2) return "Có mây";
  if (code === 3) return "Nhiều mây";
  if (code === 45 || code === 48) return "Sương mù";
  if (code >= 51 && code <= 57) return "Mưa phùn";
  if (code >= 61 && code <= 67) return "Mưa";
  if (code >= 71 && code <= 77) return "Tuyết";
  if (code >= 80 && code <= 82) return "Mưa rào";
  if (code === 85 || code === 86) return "Mưa tuyết";
  if (code >= 95) return "Dông";
  return "Thời tiết";
}

/**
 * Ô thời tiết và giờ cạnh thanh tìm kiếm.
 *
 * Chỉ để nhìn: không bấm được, không mở ra gì thêm. Đồng hồ chạy hoàn toàn ở
 * giao diện nên không tốn lượt gọi nào; chỉ phần thời tiết mới hỏi backend.
 * Không lấy được thời tiết thì ô chỉ còn giờ, chứ không hiện lỗi — đây là
 * phần phụ, không đáng làm phiền người đang đọc tin.
 */
export function WeatherClock({ place }: { place: string }) {
  const [weather, setWeather] = useState<Weather | null>(null);
  const [now, setNow] = useState(() => new Date());

  useEffect(() => {
    let alive = true;
    const load = () => {
      api
        .getWeather()
        .then((next) => {
          // Giữ lại số liệu cũ khi lượt mới hỏng, để ô không chớp tắt.
          if (alive && next) setWeather(next);
        })
        .catch(() => {});
    };
    load();
    const timer = window.setInterval(load, WEATHER_EVERY_MS);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
    // Đổi vị trí trong cài đặt thì lấy lại ngay. Không phụ thuộc vào giá trị
    // này thì người dùng đổi xong vẫn thấy nơi cũ suốt mười lăm phút.
  }, [place]);

  // Nhịp đồng hồ bám đúng đầu phút thay vì đếm mỗi 60 giây từ lúc mở app,
  // nếu không thì số phút đổi lệch tới gần một phút so với đồng hồ máy.
  useEffect(() => {
    let timer = 0;
    const tick = () => {
      const at = new Date();
      setNow(at);
      timer = window.setTimeout(tick, 60_000 - (at.getSeconds() * 1000 + at.getMilliseconds()));
    };
    timer = window.setTimeout(tick, 60_000 - (now.getSeconds() * 1000 + now.getMilliseconds()));
    return () => window.clearTimeout(timer);
    // Chạy một lần: nhịp tự đặt lại hẹn giờ cho lần sau.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const clock = now.toLocaleTimeString("vi-VN", { hour: "2-digit", minute: "2-digit", hour12: false });
  const label = weather
    ? `${describe(weather.code)} · ${weather.tempC}°C · ${weather.place}`
    : undefined;

  return (
    <div className="weather-clock" title={label} aria-label={label ?? `Bây giờ là ${clock}`}>
      {weather && (
        <>
          <span className="wc-icon">
            <WeatherIcon code={weather.code} isDay={weather.isDay} />
          </span>
          <span className="wc-temp">{weather.tempC}°</span>
          {/* Tên nơi lấy số liệu. Cắt bớt khi quá dài: "Thành phố Hồ Chí Minh"
              để nguyên sẽ đẩy thanh công cụ xuống dòng. */}
          <span className="wc-place">{weather.place}</span>
          <span className="wc-sep" aria-hidden="true" />
        </>
      )}
      <span className="wc-time">{clock}</span>
    </div>
  );
}
