import { useState } from "react";
import { useI18n } from "../i18n";

interface Props {
  value: string;
  onChange: (value: string) => void;
  /** 下拉里的候选：默认分类加已经用过的自定义分类，见 `categoryOptions`。 */
  options: string[];
  className: string;
}

const CUSTOM = "\u0000custom";

/** 岗位分类：从列表选，或切换成文本框自己填。 */
export default function CategoryInput({ value, onChange, options, className }: Props) {
  const { t } = useI18n();
  const [typing, setTyping] = useState(false);
  // 外部填进来的值（比如抓取识别到的「营销类」）不在列表里时，也直接显示成文本框。
  if (typing || (value.trim() !== "" && !options.includes(value))) {
    return (
      <div className="flex gap-1">
        <input
          className={className}
          autoFocus={typing}
          maxLength={20}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t("输入自定义分类")}
          aria-label={t("岗位分类")}
        />
        <button
          type="button"
          className="px-2 text-xs text-neutral-500 hover:text-navy whitespace-nowrap"
          onClick={() => {
            setTyping(false);
            if (!options.includes(value)) onChange("");
          }}
        >
          {t("从列表选")}
        </button>
      </div>
    );
  }
  return (
    <select
      className={className}
      aria-label={t("岗位分类")}
      value={value}
      onChange={(e) => {
        if (e.target.value === CUSTOM) {
          setTyping(true);
          onChange("");
        } else {
          onChange(e.target.value);
        }
      }}
    >
      <option value="">{t("选择分类")}</option>
      {options.map((c) => <option key={c} value={c}>{t(c)}</option>)}
      <option value={CUSTOM}>{t("自定义…")}</option>
    </select>
  );
}
