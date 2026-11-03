import { useState } from "react";
import { useI18n } from "../i18n";

interface Props {
  value: string;
  onChange: (value: string) => void;
  options: string[];
  className: string;
}

const CUSTOM = "\u0000custom";

export function CategoryInput(props: Props) {
  return <CustomSelect {...props} label="岗位分类" prompt="选择分类" placeholder="输入自定义分类" maxLength={20} />;
}

export function CustomSelect({ value, onChange, options, className, id, label, prompt, placeholder, maxLength }: Props & {
  id?: string; label: string; prompt: string; placeholder: string; maxLength?: number;
}) {
  const { t } = useI18n();
  const [typing, setTyping] = useState(false);
  // 外部填进来的值（比如抓取识别到的「营销类」）不在列表里时，也直接显示成文本框。
  if (typing || (value.trim() !== "" && !options.includes(value))) {
    return (
      <div className="flex gap-1">
        <input
          className={className}
          autoFocus={typing}
          id={id}
          maxLength={maxLength}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t(placeholder)}
          aria-label={t(label)}
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
      id={id}
      aria-label={t(label)}
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
      <option value="">{t(prompt)}</option>
      {options.map((c) => <option key={c} value={c}>{t(c)}</option>)}
      <option value={CUSTOM}>{t("自定义…")}</option>
    </select>
  );
}
