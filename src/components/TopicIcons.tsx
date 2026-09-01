/**
 * Icon vector cho từng chủ đề. Vẽ bằng đường nét mảnh 1.7 để đồng bộ với
 * bộ icon giao diện, và dùng currentColor nên tự đổi màu theo trạng thái
 * chọn của mục điều hướng.
 */
const svg = {
  width: 16,
  height: 16,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.7,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
};

const AllTopics = () => (
  <svg {...svg}>
    <rect x="4" y="4" width="7" height="7" rx="2.2" />
    <rect x="13" y="4" width="7" height="7" rx="2.2" />
    <rect x="4" y="13" width="7" height="7" rx="2.2" />
    <rect x="13" y="13" width="7" height="7" rx="2.2" />
  </svg>
);

/** Tia sáng — quy ước quen thuộc cho nội dung liên quan tới AI. */
const AiTopic = () => (
  <svg {...svg}>
    <path d="M11 3.2 12.5 8 17.3 9.5 12.5 11 11 15.8 9.5 11 4.7 9.5 9.5 8Z" />
    <path d="M17.8 15 18.5 17 20.5 17.7 18.5 18.4 17.8 20.4 17.1 18.4 15.1 17.7 17.1 17Z" />
  </svg>
);

const SecurityTopic = () => (
  <svg {...svg}>
    <path d="M12 3.2 19 5.9v5.4c0 4-2.9 7.5-7 9.1-4.1-1.6-7-5.1-7-9.1V5.9Z" />
    <path d="m9.2 11.9 2.1 2.1 4.1-4.2" />
  </svg>
);

/** Con chip — thân vuông lớn để không bị nhầm với bánh răng ở cỡ nhỏ. */
const HardwareTopic = () => (
  <svg {...svg}>
    <rect x="6" y="6" width="12" height="12" rx="2.8" />
    <path d="M9.6 2.6V6M14.4 2.6V6M9.6 18v3.4M14.4 18v3.4M2.6 9.6H6M2.6 14.4H6M18 9.6h3.4M18 14.4h3.4" />
  </svg>
);

const DeviceTopic = () => (
  <svg {...svg}>
    <rect x="7" y="2.6" width="10" height="18.8" rx="2.8" />
    <path d="M10.6 18.4h2.8" />
  </svg>
);

/** Tia điện — xe điện. */
const EvTopic = () => (
  <svg {...svg}>
    <path d="M13.2 3 5.8 13.6h5.1L10 21l7.4-10.6h-5Z" />
  </svg>
);

/** Tay cầm chơi game.
 *
 * Thân phải cao và chi tiết bên trong phải to, nếu không ở cỡ 16px cả phím
 * điều hướng lẫn hai nút đều biến mất, chỉ còn lại một khối mờ.
 */
const GamesTopic = () => (
  <svg {...svg}>
    <path d="M8.6 7h6.8c3.4 0 6 3 5.4 6.3l-.6 3.2a2.6 2.6 0 0 1-4.8.7L14.3 15H9.7l-1.1 2.2a2.6 2.6 0 0 1-4.8-.7l-.6-3.2C2.6 10 5.2 7 8.6 7Z" />
    <path d="M7.4 10.4v2.9M5.95 11.85h2.9" strokeWidth={1.9} />
    <circle cx="15.7" cy="10.9" r="1.15" fill="currentColor" stroke="none" />
    <circle cx="17.9" cy="12.9" r="1.15" fill="currentColor" stroke="none" />
  </svg>
);

const SocialTopic = () => (
  <svg {...svg}>
    <path d="M20 11.9c0 3.8-3.6 6.9-8 6.9-1 0-1.9-.2-2.8-.5L4 20.6l1.4-3.8A6.5 6.5 0 0 1 4 11.9C4 8.1 7.6 5 12 5s8 3.1 8 6.9Z" />
  </svg>
);

/** Tên lửa — rõ nghĩa hơn hành tinh có vành đai khi thu về 16px. */
const SpaceTopic = () => (
  <svg {...svg}>
    <path d="M12 3.4c2.6 2.2 4 5.2 4 8.6l-1.6 3H9.6l-1.6-3c0-3.4 1.4-6.4 4-8.6Z" />
    <circle cx="12" cy="9.3" r="1.6" />
    <path d="M9.7 15.4h4.6L12 20.6Z" />
  </svg>
);

const OtherTopic = () => (
  <svg {...svg} strokeWidth={0}>
    <circle cx="6.2" cy="12" r="1.5" fill="currentColor" />
    <circle cx="12" cy="12" r="1.5" fill="currentColor" />
    <circle cx="17.8" cy="12" r="1.5" fill="currentColor" />
  </svg>
);

const TOPIC_ICONS: Record<string, () => React.ReactElement> = {
  all: AllTopics,
  ai: AiTopic,
  security: SecurityTopic,
  hardware: HardwareTopic,
  device: DeviceTopic,
  games: GamesTopic,
  ev: EvTopic,
  social: SocialTopic,
  space: SpaceTopic,
  other: OtherTopic,
};

export function TopicIcon({ topic }: { topic: string }) {
  const Icon = TOPIC_ICONS[topic] ?? OtherTopic;
  return <Icon />;
}
