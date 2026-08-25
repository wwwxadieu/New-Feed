import { createContext, useContext, useState } from "react";
import type { Source } from "../lib/types";
import { initials } from "../lib/format";

/** Tra logo theo id nguồn, tránh phải truyền tay qua nhiều lớp thành phần. */
export const SourcesContext = createContext<Map<string, Source>>(new Map());

interface Props {
  sourceId: string;
  name: string;
  size?: number;
  radius?: number;
}

export function SourceLogo({ sourceId, name, size = 20, radius }: Props) {
  const sources = useContext(SourcesContext);
  const logo = sources.get(sourceId)?.logo ?? null;
  const [failed, setFailed] = useState(false);

  const style = {
    width: size,
    height: size,
    borderRadius: radius ?? size / 2,
    fontSize: Math.max(size * 0.42, 8),
  };

  // Chưa có logo hoặc ảnh hỏng thì lùi về chữ viết tắt của tên nguồn.
  if (!logo || failed) {
    return (
      <span className="avatar" style={style} title={name}>
        {initials(name)}
      </span>
    );
  }

  return (
    <span className="avatar has-logo" style={style} title={name}>
      <img src={logo} alt="" onError={() => setFailed(true)} />
    </span>
  );
}
