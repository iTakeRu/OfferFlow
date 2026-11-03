//! 读 HTML 的小工具：转纯文本，以及按标签遍历页面。
//!
//! 不是完整的 HTML 解析器——够从招聘页里取标题、meta、带特定类名的元素即可；
//! 碰到畸形标记时宁可少读，也不报错。

// ---------- 转纯文本 ----------
fn clean_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    let mut prev_newline = 0u8;
    for ch in s.chars() {
        if ch == '\n' || ch == '\r' {
            if prev_newline == 0 {
                out.push('\n');
                prev_newline += 1;
            }
            in_space = true;
        } else if ch == ' ' || ch == '\t' {
            in_space = true;
        } else {
            if in_space && prev_newline == 0 {
                out.push(' ');
            }
            out.push(ch);
            in_space = false;
            prev_newline = 0;
        }
    }
    out.trim().to_string()
}

pub(crate) fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
}

pub fn html_to_text(html: &str) -> String {
    to_text(html, false)
}

/// 招聘卡片常把城市、经验等相邻字段放在没有空白的行内元素里。
pub(crate) fn job_card_text(html: &str) -> String {
    to_text(html, true)
}

fn to_text(html: &str, separate_fields: bool) -> String {
    let lower = html.to_ascii_lowercase();
    let lb = lower.as_bytes();
    let b = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0usize;
    let mut skip_until: Option<&'static str> = None;

    while i < b.len() {
        if let Some(close) = skip_until {
            if let Some(pos) = lower[i..].find(close) {
                i += pos + close.len();
                while i < b.len() && b[i] != b'>' {
                    i += 1;
                }
                if i < b.len() {
                    i += 1;
                }
                skip_until = None;
                continue;
            } else {
                break;
            }
        }

        if lb[i] == b'<' {
            let after = &lower[i..];
            let mut block_close: Option<&'static str> = None;
            if after.starts_with("<script") {
                block_close = Some("</script");
            } else if after.starts_with("<style") {
                block_close = Some("</style");
            } else if after.starts_with("<noscript") {
                block_close = Some("</noscript");
            }
            if let Some(close) = block_close {
                skip_until = Some(close);
                continue;
            }
            if separate_fields
                && (after.starts_with("</span>")
                    || after.starts_with("</a>")
                    || after.starts_with("</em>"))
            {
                out.push(' ');
            } else if after.starts_with("<br") {
                out.push('\n');
            } else {
                let mut is_block_close = false;
                for t in [
                    "</p",
                    "</div",
                    "</li",
                    "</tr",
                    "</h1",
                    "</h2",
                    "</h3",
                    "</h4",
                    "</h5",
                    "</h6",
                    "</section",
                    "</article",
                    "</aside",
                    "</header",
                    "</nav",
                    "</dd",
                    "</dt",
                    "</ul",
                    "</ol",
                    "</table",
                    "</pre",
                ] {
                    if after.starts_with(t) {
                        is_block_close = true;
                        break;
                    }
                }
                if is_block_close {
                    out.push('\n');
                }
            }
            i += 1;
            while i < b.len() && b[i] != b'>' {
                i += 1;
            }
            if i < b.len() {
                i += 1;
            }
            continue;
        } else {
            let ch = html[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }

    clean_text(&decode_entities(&out))
}

/// 元素文字压成一行。
pub(crate) fn one_line(html: &str) -> String {
    html_to_text(html)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// 非空文字行，去掉相邻重复（有的页面为了做省略号效果，同一段文字会渲染两遍）。
pub(crate) fn text_lines(html: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in html_to_text(html).lines() {
        let line = line.trim();
        if !line.is_empty() && out.last().map(String::as_str) != Some(line) {
            out.push(line.to_string());
        }
    }
    out
}

// ---------- 按标签遍历 ----------

/// 内容是纯文本的元素，里面的 `<` 不是标签。
const RAW_TEXT: &[&str] = &["script", "style", "textarea", "title"];
/// 没有内容、也没有闭合标签的元素。
const VOID: &[&str] = &["meta", "link", "img", "br", "hr", "input", "source", "base"];

pub(crate) struct Page<'a> {
    html: &'a str,
    /// 只转换 ASCII 大小写，字节位置与 `html` 一一对应。
    lower: String,
}

/// 一个开始标签。
pub(crate) struct Tag<'p> {
    /// 小写标签名。
    pub name: &'p str,
    attrs: &'p str,
    attrs_lower: &'p str,
    /// 开始标签 `>` 之后的位置。
    end: usize,
}

impl<'a> Page<'a> {
    pub fn new(html: &'a str) -> Self {
        Self {
            html,
            lower: html.to_ascii_lowercase(),
        }
    }

    /// 页面源码里是否出现某段文字（不区分 ASCII 大小写，`needle` 需为小写）。
    pub fn mentions(&self, needle: &str) -> bool {
        self.lower.contains(needle)
    }

    /// 依次给出每个开始标签；跳过闭合标签、注释、doctype 和纯文本元素的内容。
    pub fn tags(&self) -> impl Iterator<Item = Tag<'_>> + '_ {
        let mut offset = 0usize;
        std::iter::from_fn(move || loop {
            let start = offset + self.lower[offset..].find('<')?;
            offset = start + 1;
            let rest = &self.lower[start + 1..];
            let name_len = rest
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(rest.len());
            if name_len == 0 {
                continue;
            }
            let end = start + self.lower[start..].find('>')? + 1;
            let attrs = start + 1 + name_len..end - 1;
            let tag = Tag {
                name: &self.lower[start + 1..start + 1 + name_len],
                attrs: &self.html[attrs.clone()],
                attrs_lower: &self.lower[attrs],
                end,
            };
            if RAW_TEXT.contains(&tag.name) {
                offset = self.close_of(&tag);
            }
            return Some(tag);
        })
    }

    /// 元素的内部 HTML。
    pub fn inner(&self, tag: &Tag) -> &'a str {
        &self.html[tag.end..self.close_of(tag)]
    }

    /// 配对的闭合标签起点；按同名标签计数处理嵌套，找不到就到文末。
    fn close_of(&self, tag: &Tag) -> usize {
        if VOID.contains(&tag.name) {
            return tag.end;
        }
        let close = format!("</{}", tag.name);
        if RAW_TEXT.contains(&tag.name) {
            return find_tag(&self.lower, &close, tag.end).unwrap_or(self.lower.len());
        }
        let open = format!("<{}", tag.name);
        let mut depth = 0usize;
        let mut i = tag.end;
        loop {
            let Some(c) = find_tag(&self.lower, &close, i) else {
                return self.lower.len();
            };
            match find_tag(&self.lower, &open, i) {
                Some(o) if o < c => {
                    depth += 1;
                    i = o + open.len();
                }
                _ if depth == 0 => return c,
                _ => {
                    depth -= 1;
                    i = c + close.len();
                }
            }
        }
    }

    /// 满足条件的元素的内部 HTML。
    pub fn elements<'s>(
        &'s self,
        pred: impl Fn(&Tag) -> bool + 's,
    ) -> impl Iterator<Item = &'a str> + 's {
        self.tags().filter(move |t| pred(t)).map(|t| self.inner(&t))
    }

    pub fn by_name<'s>(&'s self, name: &'s str) -> impl Iterator<Item = &'a str> + 's {
        self.elements(move |t| t.name == name)
    }

    /// 类名以 `prefix` 开头的元素。CSS Modules 的类名后缀是构建时生成的哈希，只能按前缀匹配。
    pub fn by_class_prefix<'s>(&'s self, prefix: &'s str) -> impl Iterator<Item = &'a str> + 's {
        self.elements(move |t| t.has_class_prefix(prefix))
    }

    /// `<meta property|name="key" content="...">`
    pub fn meta(&self, key: &str) -> Option<String> {
        self.tags()
            .filter(|t| t.name == "meta")
            .find(|t| {
                ["property", "name"]
                    .iter()
                    .any(|a| t.attr(a).is_some_and(|v| v.eq_ignore_ascii_case(key)))
            })
            .and_then(|t| t.attr("content"))
            .filter(|v| !v.is_empty())
    }

    /// `<link rel="rel" href="...">`
    pub fn link(&self, rel: &str) -> Option<String> {
        self.tags()
            .filter(|t| t.name == "link")
            .find(|t| t.attr("rel").is_some_and(|v| v.eq_ignore_ascii_case(rel)))
            .and_then(|t| t.attr("href"))
            .filter(|v| !v.is_empty())
    }
}

impl Tag<'_> {
    /// 属性值，已解码实体；`name` 需为小写。
    pub fn attr(&self, name: &str) -> Option<String> {
        let mut from = 0usize;
        while let Some(rel) = self.attrs_lower[from..].find(name) {
            let at = from + rel;
            from = at + name.len();
            // 前面必须是空白，避免 `data-title=` 命中 `title=`
            if !self.attrs_lower[..at].ends_with(|c: char| c.is_ascii_whitespace()) {
                continue;
            }
            let rest = self.attrs[at + name.len()..].trim_start();
            let Some(value) = rest.strip_prefix('=').map(str::trim_start) else {
                continue;
            };
            let raw = match value.chars().next() {
                Some(q @ ('"' | '\'')) => value[1..].split(q).next().unwrap_or(""),
                _ => value.split_whitespace().next().unwrap_or(""),
            };
            return Some(decode_entities(raw.trim()));
        }
        None
    }

    pub fn has_class_prefix(&self, prefix: &str) -> bool {
        self.attr("class")
            .is_some_and(|c| c.split_whitespace().any(|t| t.starts_with(prefix)))
    }
}

/// 找下一个 `<name` / `</name`，要求后面紧跟空白、`>` 或 `/`，避免 `<div` 命中 `<divider`。
fn find_tag(lower: &str, pat: &str, from: usize) -> Option<usize> {
    let mut i = from;
    while let Some(rel) = lower[i..].find(pat) {
        let at = i + rel;
        let next = lower[at + pat.len()..].chars().next();
        if matches!(next, None | Some(' ' | '\t' | '\n' | '\r' | '>' | '/')) {
            return Some(at);
        }
        i = at + pat.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_attributes() {
        let page = Page::new(r#"<a data-title="x" TITLE='y' class=a&amp;b href=/j>"#);
        let tag = page.tags().next().unwrap();
        assert_eq!(tag.attr("title").as_deref(), Some("y"));
        assert_eq!(tag.attr("class").as_deref(), Some("a&b"));
        assert_eq!(tag.attr("href").as_deref(), Some("/j"));
        assert_eq!(tag.attr("id"), None);
    }

    #[test]
    fn matches_nested_elements() {
        let page =
            Page::new("<div class=\"t-1\"><div>a</div><divider></divider>b</div><div>c</div>");
        assert_eq!(
            page.by_class_prefix("t-").next(),
            Some("<div>a</div><divider></divider>b")
        );
        assert_eq!(page.by_name("div").count(), 3);
    }

    #[test]
    fn skips_markup_inside_scripts() {
        let page = Page::new(
            r#"<script>var s = "<h1>假标题</h1>";</script><title>a <b> c</title><h1>真标题</h1>"#,
        );
        assert_eq!(page.by_name("h1").collect::<Vec<_>>(), ["真标题"]);
        assert_eq!(page.by_name("title").next(), Some("a <b> c"));
    }

    #[test]
    fn reads_meta_and_link() {
        let page = Page::new(
            r#"<meta content="产品经理" property="og:title"><link rel="canonical" href="https://a.example/j?x=1&amp;y=2"><p>x</p>"#,
        );
        assert_eq!(page.meta("og:title").as_deref(), Some("产品经理"));
        assert_eq!(
            page.link("canonical").as_deref(),
            Some("https://a.example/j?x=1&y=2")
        );
        // void 元素没有内容，不会把后面的整页当成它的内部
        assert_eq!(page.by_name("meta").next(), Some(""));
    }
}
