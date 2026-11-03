/** 一次表单会话中，创建已成功但刷新失败时，重试沿用原来的投递 ID。 */
export function createApplicationSaver<P>(
  initialId: number | null,
  create: (payload: P) => Promise<{ id: number }>,
  update: (id: number, payload: P) => Promise<unknown>,
) {
  let savedId = initialId;
  let saving = false;
  return async (payload: P, onSaved: () => void | Promise<void>): Promise<void> => {
    if (saving) return;
    saving = true;
    try {
      if (savedId === null) savedId = (await create(payload)).id;
      else await update(savedId, payload);
      await onSaved();
    } finally {
      saving = false;
    }
  };
}
/** Use only sources we can identify from the posting URL. */
export function channelFromSource(source?: string | null): string {
  if (!source) return "";
  try {
    const { hostname, protocol } = new URL(source);
    if (protocol !== "http:" && protocol !== "https:") return "";
    if (hostname === "zhipin.com" || hostname.endsWith(".zhipin.com")) return "BOSS直聘";
    if (hostname === "mokahr.com" || hostname.endsWith(".mokahr.com")) return "官网";
  } catch { /* A snapshot may have no valid source URL. */ }
  return "";
}

export function channelOptions(applications: { channel: string | null }[]): string[] {
  const options = new Map<string, string>();
  for (const raw of ["BOSS直聘", "官网", "内推", ...applications.map((a) => a.channel ?? "")]) {
    const value = raw.trim();
    const key = value.toLowerCase();
    if (value && !options.has(key)) options.set(key, value);
  }
  return [...options.values()];
}
