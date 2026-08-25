import { useLayoutEffect, useRef, useState } from "react";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

interface Props<T extends string> {
  options: SegmentedOption<T>[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}

/**
 * Segmented control kiểu macOS: con trượt nền chuyển động giữa các mục
 * thay vì đổi màu tức thời.
 */
export function Segmented<T extends string>({ options, value, onChange, label }: Props<T>) {
  const refs = useRef<(HTMLButtonElement | null)[]>([]);
  const [indicator, setIndicator] = useState({ left: 0, width: 0 });

  const activeIndex = options.findIndex((o) => o.value === value);

  useLayoutEffect(() => {
    const node = refs.current[activeIndex];
    if (!node) return;
    setIndicator({ left: node.offsetLeft, width: node.offsetWidth });
  }, [activeIndex, options.length]);

  return (
    <div className="segmented" role="tablist" aria-label={label}>
      <span
        className="indicator"
        style={{ transform: `translateX(${indicator.left - 2}px)`, width: indicator.width }}
      />
      {options.map((option, index) => (
        <button
          key={option.value}
          ref={(node) => {
            refs.current[index] = node;
          }}
          role="tab"
          aria-selected={option.value === value}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
