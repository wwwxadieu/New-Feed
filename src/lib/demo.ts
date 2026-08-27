import type { Article, Cluster, Snapshot, Source } from "./types";

/**
 * Dữ liệu mẫu dùng khi chạy `npm run dev` trong trình duyệt (không có backend Rust).
 * Trong ứng dụng thật, toàn bộ dữ liệu này đến từ các nguồn tin người dùng thêm vào.
 */
const now = Date.now();
const at = (hoursAgo: number) => new Date(now - hoursAgo * 3_600_000).toISOString();

let seq = 0;
function article(sourceTitle: string, title: string, hoursAgo: number, summary = ""): Article {
  seq += 1;
  return {
    id: `demo-${seq}`,
    sourceId: sourceTitle.toLowerCase().replace(/\s+/g, "-"),
    sourceTitle,
    title,
    url: `https://example.com/demo/${seq}`,
    summary,
    published: at(hoursAgo),
    image: null,
    titleVi: null,
    summaryVi: null,
    thumb: null,
  };
}

const RAW_CLUSTERS: Omit<Cluster, "sourceCount" | "titleVi" | "summaryVi">[] = [
  {
    id: "demo-sec",
    topic: "security",
    title: "Lỗ hổng thực thi mã từ xa trong thư viện nén dùng chung, bản vá phát hành khẩn",
    summary:
      "Lỗi nằm ở khâu xử lý header của định dạng nén được tích hợp sẵn trong nhiều bản phân phối Linux và runtime máy chủ. Bản vá đã có cho ba nhánh còn được hỗ trợ.",
    newest: at(1),
    score: 420,
    articles: [
      article("The Register", "Lỗ hổng thực thi mã từ xa trong thư viện nén dùng chung, bản vá phát hành khẩn", 1,
        "Lỗi nằm ở khâu xử lý header của định dạng nén được tích hợp sẵn trong nhiều bản phân phối Linux và runtime máy chủ. Bản vá đã có cho ba nhánh còn được hỗ trợ."),
      article("Ars Technica", "Thư viện nén phổ biến dính lỗi ghi đè bộ nhớ, mức nghiêm trọng cao nhất", 1),
      article("VnExpress Số hóa", "Khuyến cáo quản trị hệ thống vá khẩn lỗ hổng thư viện nén", 2),
      article("Genk", "Lỗ hổng mới ảnh hưởng hàng loạt máy chủ, cách kiểm tra hệ thống", 3),
    ],
  },
  {
    id: "demo-ai",
    topic: "ai",
    title: "Mô hình ngôn ngữ mở 400 tỷ tham số được phát hành kèm trọng số đầy đủ",
    summary:
      "Bản phát hành đi kèm giấy phép cho phép dùng thương mại và tài liệu huấn luyện. Điểm gây tranh luận nhiều nhất là chi phí suy luận khi triển khai ngoài trung tâm dữ liệu lớn.",
    newest: at(2),
    score: 380,
    articles: [
      article("TechCrunch", "Mô hình ngôn ngữ mở 400 tỷ tham số được phát hành kèm trọng số đầy đủ", 2,
        "Bản phát hành đi kèm giấy phép cho phép dùng thương mại và tài liệu huấn luyện. Điểm gây tranh luận nhiều nhất là chi phí suy luận khi triển khai ngoài trung tâm dữ liệu lớn."),
      article("The Verge", "Trọng số mô hình mở mới đã có trên kho công khai", 2),
      article("Tinh tế", "Thử chạy bản lượng tử hoá của mô hình mở mới trên máy trạm", 4),
    ],
  },
  {
    id: "demo-chip",
    topic: "hardware",
    title: "Dây chuyền 2nm bước vào sản xuất thử, năng suất tấm wafer đạt khoảng 60%",
    summary:
      "Con số năng suất được xem là đủ tốt cho giai đoạn thử nghiệm nhưng vẫn cách xa ngưỡng thương mại. Lô đầu tiên dự kiến dành cho khách hàng đặt sớm.",
    newest: at(4),
    score: 260,
    articles: [
      article("Ars Technica", "Dây chuyền 2nm bước vào sản xuất thử, năng suất tấm wafer đạt khoảng 60%", 4,
        "Con số năng suất được xem là đủ tốt cho giai đoạn thử nghiệm nhưng vẫn cách xa ngưỡng thương mại."),
      article("The Register", "Số liệu năng suất 2nm đầu tiên từ dây chuyền thật", 5),
      article("Genk", "Tiến trình 2nm và ảnh hưởng tới giá thiết bị", 6),
    ],
  },
  {
    id: "demo-ev",
    topic: "ev",
    title: "Chuẩn sạc mới rút thời gian nạp 10–80% xuống còn khoảng 9 phút trong thử nghiệm",
    summary:
      "Kết quả đạt được trên trạm sạc thử nghiệm với gói pin chuyên biệt. Hạ tầng lưới điện được nêu là rào cản lớn hơn bản thân công nghệ pin.",
    newest: at(8),
    score: 190,
    articles: [
      article("Engadget", "Chuẩn sạc mới rút thời gian nạp 10–80% xuống còn khoảng 9 phút trong thử nghiệm", 8,
        "Kết quả đạt được trên trạm sạc thử nghiệm với gói pin chuyên biệt."),
      article("The Verge", "Sạc 9 phút: con số ấn tượng nhưng lưới điện có theo kịp?", 9),
      article("VnExpress Số hóa", "Công nghệ sạc nhanh mới và mạng lưới trạm trong nước", 10),
    ],
  },
  {
    id: "demo-startup",
    topic: "startup",
    title: "Nền tảng hạ tầng dữ liệu gọi vốn 120 triệu USD vòng Series B",
    summary: "Vòng gọi vốn nâng định giá lên khoảng 1,1 tỷ USD, phần lớn dành cho mở rộng đội ngũ kỹ thuật.",
    newest: at(6),
    score: 150,
    articles: [
      article("TechCrunch", "Nền tảng hạ tầng dữ liệu gọi vốn 120 triệu USD vòng Series B", 6,
        "Vòng gọi vốn nâng định giá lên khoảng 1,1 tỷ USD, phần lớn dành cho mở rộng đội ngũ kỹ thuật."),
      article("ICTnews", "Startup hạ tầng dữ liệu huy động thêm vốn, nhắm thị trường châu Á", 9),
    ],
  },
  {
    id: "demo-games",
    topic: "games",
    title: "Bản cập nhật lớn giữa mùa của tựa game nhập vai đứng đầu bảng xếp hạng",
    summary:
      "Bản cập nhật bổ sung một vùng bản đồ mới và chỉnh lại hệ thống chiến đấu sau phản hồi của cộng đồng. Giải đấu thể thao điện tử đầu tiên theo phiên bản này diễn ra cuối tháng.",
    newest: at(5),
    score: 170,
    articles: [
      article("Genk", "Bản cập nhật lớn giữa mùa của tựa game nhập vai đứng đầu bảng xếp hạng", 5,
        "Bản cập nhật bổ sung một vùng bản đồ mới và chỉnh lại hệ thống chiến đấu sau phản hồi của cộng đồng."),
      article("The Verge", "Game thủ chia rẽ vì thay đổi hệ thống chiến đấu trong bản mới", 7),
    ],
  },
  {
    id: "demo-social",
    topic: "social",
    title: "Nền tảng thử nghiệm nhãn tự động cho nội dung do AI tạo trên dòng thời gian",
    summary: "Nhãn dựa trên siêu dữ liệu đi kèm tệp và một lớp phân loại nội bộ. Giai đoạn đầu chỉ áp dụng cho ảnh tĩnh.",
    newest: at(12),
    score: 110,
    articles: [
      article("The Verge", "Nền tảng thử nghiệm nhãn tự động cho nội dung do AI tạo trên dòng thời gian", 12,
        "Nhãn dựa trên siêu dữ liệu đi kèm tệp và một lớp phân loại nội bộ."),
      article("Engadget", "Cách nhãn nội dung AI hoạt động và khi nào nó sai", 13),
    ],
  },
];

// Mỗi bài trong dữ liệu mẫu đến từ một nguồn khác nhau.
const CLUSTERS: Cluster[] = RAW_CLUSTERS.map((c) => ({
  ...c,
  sourceCount: c.articles.length,
  titleVi: null,
  summaryVi: null,
}));

const SOURCES: Source[] = [
  "VnExpress Số hóa",
  "Genk",
  "TechCrunch",
  "The Verge",
  "Ars Technica",
  "Engadget",
].map((title, i) => ({
  id: title.toLowerCase().replace(/\s+/g, "-"),
  title,
  homeUrl: "https://example.com",
  feedUrl: "https://example.com/feed",
  enabled: true,
  addedAt: at(240),
  lastFetched: at(0.2),
  lastError: null,
  articleCount: 40 + i * 7,
  logo: null,
  language: i >= 2 ? "other" : "vi",
}));

export function demoSnapshot(): Snapshot {
  const topicCounts = Object.entries(
    CLUSTERS.reduce<Record<string, number>>((acc, c) => {
      acc[c.topic] = (acc[c.topic] ?? 0) + c.articles.length * 9;
      return acc;
    }, {}),
  ).sort((a, b) => b[1] - a[1]) as [string, number][];

  return {
    sources: SOURCES,
    clusters: CLUSTERS,
    settings: { theme: "auto", windowHours: 24, maxPerSource: 25, translate: true, translateEmail: "" },
    articleCount: 284,
    topicCounts,
    hourly: [12, 9, 7, 5, 4, 6, 11, 19, 28, 34, 31, 27, 33, 41, 38, 30, 26, 24, 29, 36, 44, 52, 47, 38],
    lastRefresh: at(0.1),
    translateNotice: null,
  };
}
