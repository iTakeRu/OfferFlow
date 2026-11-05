import { useI18n } from "../i18n";

interface Props {
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

export default function JdTextEditor({ value, onChange, disabled }: Props) {
  const { t } = useI18n();
  return (
    <textarea
      aria-label={t("JD 正文")}
      value={value}
      onChange={(event) => onChange(event.target.value)}
      disabled={disabled}
      rows={8}
      maxLength={500_000}
      className="mt-2 w-full resize-y rounded border border-neutral-300 bg-white p-2 text-sm"
    />
  );
}
