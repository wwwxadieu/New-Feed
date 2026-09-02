/**
 * Icon thời tiết theo mã WMO của Open-Meteo.
 *
 * Vẽ ở cỡ 18px là chính, nên nét để 1.6 và chi tiết giữ ở mức tối thiểu:
 * lần trước bộ icon chủ đề phải vẽ lại vì các chi tiết nhỏ dính vào nhau khi
 * thu về cỡ thật.
 */

interface Props {
  code: number;
  isDay: boolean;
  size?: number;
}

const stroke = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.6,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
};

/** Đường viền chung của khối mây, dùng lại cho mưa, tuyết và dông.
 *
 * Đặt cao trong khung (đáy ở y=15 chứ không phải 18) để chừa chỗ cho phần
 * mưa, tuyết hay tia sét bên dưới. Bản đầu để mây thấp, còn 6 đơn vị cho
 * phần dưới, tức chưa tới 5px khi vẽ ở cỡ thật 18px — các hạt tuyết dính
 * vào nhau thành một vệt đen và tia sét thì cụt.
 */
const CLOUD = "M7 15h9.5a3.5 3.5 0 0 0 .4-6.98A5 5 0 0 0 7.2 7.2 3.9 3.9 0 0 0 7 15Z";

function Sun() {
  return (
    <>
      <circle cx="12" cy="12" r="4" {...stroke} />
      {[0, 45, 90, 135, 180, 225, 270, 315].map((deg) => (
        <line
          key={deg}
          x1="12"
          y1="4.2"
          x2="12"
          y2="2"
          transform={`rotate(${deg} 12 12)`}
          {...stroke}
        />
      ))}
    </>
  );
}

function Moon() {
  return <path d="M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5Z" {...stroke} />;
}

function Cloud({ children }: { children?: React.ReactNode }) {
  return (
    <>
      <path d={CLOUD} {...stroke} />
      {children}
    </>
  );
}

/** Mây có mặt trời hoặc mặt trăng ló ra sau. */
function PartlyCloudy({ isDay }: { isDay: boolean }) {
  return (
    <>
      {isDay ? (
        <>
          <circle cx="9" cy="8" r="2.7" {...stroke} />
          {[210, 255, 300, 345].map((deg) => (
            <line key={deg} x1="9" y1="4.1" x2="9" y2="2.4" transform={`rotate(${deg} 9 8)`} {...stroke} />
          ))}
        </>
      ) : (
        // Trăng nhỏ và lùi hẳn lên góc trái. Để nguyên cỡ như lúc đứng một
        // mình thì ở 18px nó chồng lên khối mây và cả hai nhoè thành một vệt.
        <path d="M11.5 8.4A3.8 3.8 0 0 1 7.9 3.8a3.8 3.8 0 1 0 3.6 4.6Z" {...stroke} />
      )}
      <path d="M8 18h8.5a3.5 3.5 0 0 0 .3-6.98A4.6 4.6 0 0 0 8.2 11.6 3.4 3.4 0 0 0 8 18Z" {...stroke} />
    </>
  );
}

export function WeatherIcon({ code, isDay, size = 18 }: Props) {
  const body = (() => {
    // Quang mây
    if (code === 0) return isDay ? <Sun /> : <Moon />;
    if (code === 1 || code === 2) return <PartlyCloudy isDay={isDay} />;
    if (code === 3) return <Cloud />;
    // Sương mù
    if (code === 45 || code === 48) {
      return (
        <>
          <line x1="4" y1="9" x2="20" y2="9" {...stroke} />
          <line x1="4" y1="13" x2="20" y2="13" {...stroke} />
          <line x1="7" y1="17" x2="20" y2="17" {...stroke} />
        </>
      );
    }
    // Dông
    if (code >= 95) {
      return (
        <Cloud>
          <path d="M13.2 15 10.6 19.2h3.2L11 23" {...stroke} />
        </Cloud>
      );
    }
    // Tuyết
    if ((code >= 71 && code <= 77) || code === 85 || code === 86) {
      return (
        <Cloud>
          {/* Hai bông thay vì ba, và to hơn. Ba bông sáu cánh nhét trong 11
              đơn vị thì ở cỡ thật 18px mỗi bông chỉ còn khoảng 2px, các nét
              chồng lên nhau thành một vệt đen chứ không ra hình bông tuyết. */}
          {[9.6, 14.4].map((x) => (
            <g key={x}>
              <line x1={x} y1="16.8" x2={x} y2="22" {...stroke} />
              <line x1={x - 2.2} y1="18.1" x2={x + 2.2} y2="20.7" {...stroke} />
              <line x1={x - 2.2} y1="20.7" x2={x + 2.2} y2="18.1" {...stroke} />
            </g>
          ))}
        </Cloud>
      );
    }
    // Còn lại là mưa phùn, mưa và mưa rào.
    const heavy = code === 65 || code === 82 || code === 67;
    return (
      <Cloud>
        {(heavy ? [9, 12.4, 15.8] : [10, 15].map((x) => x)).map((x) => (
          <line key={x} x1={x} y1="17.4" x2={x - 1.4} y2="21.6" {...stroke} />
        ))}
      </Cloud>
    );
  })();

  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      {body}
    </svg>
  );
}
