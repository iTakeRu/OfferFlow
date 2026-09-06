/** 文件大小、时长等展示格式化。放在一处，免得各视图各写一套。 */

export function formatSize(bytes: number): string {
  if (!isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

export function formatTime(seconds: number): string {
  if (!isFinite(seconds) || seconds < 0) return "0:00";
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

/**
 * 复盘笔记以富文本 HTML 存储；在纯文本场景（题库卡片、列表摘要）里
 * 需要还原成可读文字，否则页面上会直接看到 <p> 标签。
 */
export function htmlToPlainText(html: string): string {
  if (!html) return "";
  const doc = new DOMParser().parseFromString(html, "text/html");
  doc.querySelectorAll("br").forEach((br) => br.replaceWith("\n"));
  doc.querySelectorAll("p, div, li, h1, h2, h3, h4, h5, h6, tr").forEach((el) => {
    el.append("\n");
  });
  return (doc.body.textContent ?? "").replace(/\n{3,}/g, "\n\n").trim();
}
