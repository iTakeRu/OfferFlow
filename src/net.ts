/** 从地址里取主机名（去掉 IPv6 的方括号），解析失败时原样返回。 */
export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^\[|\]$/g, "").toLowerCase();
  } catch {
    return url.trim().toLowerCase();
  }
}

/**
 * 主机是否指向本机。用来判断正文会不会离开这台电脑，所以宁严勿松：
 * 只认 localhost、::1 和真正的 127.x.x.x 地址，`127.evil.com` 这类域名不算。
 */
export function isLoopback(host: string): boolean {
  if (host === "localhost" || host.endsWith(".localhost") || host === "::1") return true;
  const m = host.match(/^127\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
  return m !== null && m.slice(1).every((part) => Number(part) <= 255);
}
