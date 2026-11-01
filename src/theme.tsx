import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

// 每套主题只是一组 CSS 变量（见 index.css 里的 [data-theme=...]），组件本身不分主题写样式。
// 唯一的结构差异是杂志式的「NO.xx — 日期」刊头，只有带刊头的主题才显示。
export type ThemeId = "magazine" | "clean" | "midnight" | "pop";

export interface ThemeMeta {
  id: ThemeId;
  name: string;
  hint: string;
  masthead: boolean;
  // 切换菜单里的三色预览：底色 / 主色 / 强调色
  swatch: [string, string, string];
}

export const THEMES: ThemeMeta[] = [
  { id: "magazine", name: "杂志", hint: "米白纸面、衬线标题、墨蓝赭石", masthead: true, swatch: ["#f9f5eb", "#22356f", "#c97b63"] },
  { id: "clean", name: "简约", hint: "浅灰白底、无衬线、柔和圆角", masthead: false, swatch: ["#f6f7f9", "#2563eb", "#15803d"] },
  { id: "midnight", name: "夜间", hint: "深色护眼、靛蓝与琥珀点缀", masthead: false, swatch: ["#0e1117", "#4f63e0", "#f2b544"] },
  { id: "pop", name: "波普", hint: "粗黑描边、硬投影、高饱和撞色", masthead: true, swatch: ["#fff6d8", "#3348e8", "#ff8fb8"] },
];

const STORAGE_KEY = "offerflow.theme";

function isThemeId(v: unknown): v is ThemeId {
  return THEMES.some((t) => t.id === v);
}

export function readStoredTheme(): ThemeId {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return isThemeId(v) ? v : "magazine";
  } catch {
    return "magazine";
  }
}

// 在 React 渲染前就写到 <html> 上，避免启动时先闪一下默认主题
export function applyTheme(id: ThemeId) {
  document.documentElement.dataset.theme = id;
}

interface ThemeValue {
  theme: ThemeMeta;
  setTheme: (id: ThemeId) => void;
}

const ThemeContext = createContext<ThemeValue>({ theme: THEMES[0], setTheme: () => {} });

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [id, setId] = useState<ThemeId>(readStoredTheme);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, id);
    } catch {
      // 存不了就只在本次会话生效
    }
  }, [id]);

  // 先同步改 <html> 再触发渲染：子组件的 effect 比这里的 effect 先跑，
  // 像波形图这种要读 CSS 变量的，必须在重渲染时就能拿到新主题的颜色
  const setTheme = (next: ThemeId) => {
    applyTheme(next);
    setId(next);
  };

  const theme = THEMES.find((t) => t.id === id) ?? THEMES[0];
  return <ThemeContext.Provider value={{ theme, setTheme }}>{children}</ThemeContext.Provider>;
}

export function useTheme(): ThemeValue {
  return useContext(ThemeContext);
}

// 给 canvas 类组件（如波形图）取当前主题色，它们没法直接用 Tailwind 类
export function cssColor(name: string, fallback: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(`--color-${name}`).trim();
  return v || fallback;
}
