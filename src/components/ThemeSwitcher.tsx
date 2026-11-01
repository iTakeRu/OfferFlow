import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { THEMES, useTheme } from "../theme";
import { useI18n } from "../i18n";

// 侧边栏底部的「界面风格」入口，点开后向上弹出主题列表
export default function ThemeSwitcher({ itemClass }: { itemClass: string }) {
  const { t } = useI18n();
  const { theme, setTheme } = useTheme();
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    // 打开后把焦点放到当前主题上，方便直接用方向键挑选
    listRef.current?.querySelector<HTMLButtonElement>('[aria-checked="true"]')?.focus();
    const onDown = (e: MouseEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    return () => document.removeEventListener("mousedown", onDown);
  }, [open]);

  const close = () => {
    setOpen(false);
    triggerRef.current?.focus();
  };

  const onListKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
      return;
    }
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    const items = Array.from(listRef.current?.querySelectorAll<HTMLButtonElement>('[role="menuitemradio"]') ?? []);
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const next = e.key === "ArrowDown" ? (i + 1) % items.length : (i - 1 + items.length) % items.length;
    items[next]?.focus();
  };

  return (
    <div ref={rootRef} className="relative">
      <button
        ref={triggerRef}
        className={itemClass + " flex items-center gap-2"}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="flex-1">{t("界面风格")}</span>
        <span className="text-xs opacity-75">{t(theme.name)}</span>
      </button>

      {open && (
        <div
          ref={listRef}
          role="menu"
          aria-label={t("界面风格")}
          onKeyDown={onListKey}
          onBlur={(e) => {
            // Tab 键离开菜单时收起，但不抢回焦点
            if (!rootRef.current?.contains(e.relatedTarget as Node | null)) setOpen(false);
          }}
          className="absolute bottom-full left-0 right-0 mb-2 z-30 bg-white text-ink border-2 border-rule rounded-xl shadow-xl p-1.5 space-y-0.5"
        >
          {THEMES.map((th) => {
            const active = th.id === theme.id;
            return (
              <button
                key={th.id}
                role="menuitemradio"
                aria-checked={active}
                onClick={() => {
                  setTheme(th.id);
                  close();
                }}
                className={
                  "w-full flex items-center gap-3 px-2.5 py-2 rounded-md text-left transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-navy " +
                  (active ? "bg-navy-tint" : "hover:bg-neutral-100")
                }
              >
                {/* 三色预览：底色 / 主色 / 强调色，固定色值，不随当前主题变 */}
                <span aria-hidden="true" className="flex shrink-0 border border-neutral-400/50 rounded-sm overflow-hidden">
                  {th.swatch.map((c) => (
                    <span key={c} className="w-3 h-7" style={{ backgroundColor: c }} />
                  ))}
                </span>
                <span className="min-w-0 flex-1">
                  <span className="block text-sm font-bold">{t(th.name)}</span>
                  <span className="block text-xs text-neutral-500 leading-snug">{t(th.hint)}</span>
                </span>
                {active && (
                  <svg aria-hidden="true" viewBox="0 0 20 20" className="w-4 h-4 shrink-0 text-navy" fill="none" stroke="currentColor" strokeWidth="2.2">
                    <path d="m4.5 10.5 3.5 3.5 7.5-8" />
                  </svg>
                )}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}
