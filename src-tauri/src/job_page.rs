//! 从抓到的页面里识别岗位字段，不调用 AI。
//!
//! 按可信度依次尝试：
//! 1. schema.org `JobPosting` 结构化数据（站点专门给搜索引擎准备的，最可靠）；
//! 2. 已知招聘平台的页面结构（见 [`SITES`]）；
//! 3. 页面标题兜底：只看唯一的 `<h1>` 或 `og:title`，结果仅供参考。

use crate::extract::{host_matches, url_host};
use crate::html::{html_to_text, one_line, text_lines, Page};
use serde::{Deserialize, Serialize};

/// 不靠 AI 从页面里识别出的岗位字段。
#[derive(Serialize, Deserialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    pub title: String,
    pub company: String,
    pub jd_text: Option<String>,
    pub department: Option<String>,
    pub location: Option<String>,
    pub category: Option<String>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
}

/// 识别结果的来源，前端据此决定提示语、以及是否还要提供 AI 提取。
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DetailSource {
    /// schema.org JobPosting
    Structured,
    /// 已知招聘平台的页面结构
    Site,
    /// 只从页面标题推测
    Page,
}

pub(crate) fn detect_job(html: &str, url: Option<&str>) -> Option<(JobDetail, DetailSource)> {
    let page = Page::new(html);
    json_ld_job(&page)
        .map(|job| (job, DetailSource::Structured))
        .or_else(|| site_job(&page, url).map(|job| (job, DetailSource::Site)))
        .or_else(|| page_title_job(&page).map(|job| (job, DetailSource::Page)))
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim();
    (!s.is_empty() && s != "-").then(|| s.to_string())
}

// ---------- JSON-LD ----------

/// 很多招聘站点会在页面里以 schema.org `JobPosting` 发布结构化数据（本意就是
/// 给搜索引擎读的公开信息），能拿到就不必动用 AI。与站点无关，任何页面都试一次。
fn json_ld_job(page: &Page) -> Option<JobDetail> {
    page.elements(|t| {
        t.name == "script"
            && t.attr("type")
                .is_some_and(|v| v.to_ascii_lowercase().contains("ld+json"))
    })
    .filter_map(|content| serde_json::from_str::<serde_json::Value>(content.trim()).ok())
    .find_map(|v| job_from_ld_json(&v))
}

fn job_from_ld_json(v: &serde_json::Value) -> Option<JobDetail> {
    let items: Vec<&serde_json::Value> = match v.as_array() {
        Some(arr) => arr.iter().collect(),
        None => vec![v],
    };

    for item in items {
        let typed = item
            .get("@type")
            .and_then(|x| x.as_str())
            .map(|s| s.eq_ignore_ascii_case("jobposting"))
            .unwrap_or(false);
        let has_title_org = item
            .get("title")
            .and_then(|x| x.as_str())
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
            && item.get("hiringOrganization").is_some();
        if !typed && !has_title_org {
            continue;
        }

        let Some(title) = item
            .get("title")
            .and_then(|x| x.as_str())
            .filter(|t| !t.trim().is_empty())
        else {
            continue;
        };

        let company = item
            .get("hiringOrganization")
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let jd_html = item
            .get("description")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let jd_text = if jd_html.trim().is_empty() {
            None
        } else {
            Some(html_to_text(jd_html))
        };

        let department = item
            .get("department")
            .and_then(|d| d.get("name"))
            .and_then(|x| x.as_str())
            .or_else(|| {
                item.get("hiringOrganization")
                    .and_then(|h| h.get("department"))
                    .and_then(|d| d.get("name"))
                    .and_then(|x| x.as_str())
            })
            .map(|s| s.to_string());

        let mut location: Option<String> = None;
        if let Some(loc) = item.get("jobLocation") {
            if let Some(name) = loc.get("name").and_then(|x| x.as_str()) {
                location = Some(name.to_string());
            } else if let Some(addr) = loc.get("address") {
                location = ["addressLocality", "addressRegion", "addressCountry"]
                    .iter()
                    .find_map(|k| addr.get(*k).and_then(|x| x.as_str()))
                    .map(|s| s.to_string());
            }
        }

        return Some(JobDetail {
            title: title.to_string(),
            company,
            jd_text,
            department,
            location,
            ..Default::default()
        });
    }
    None
}

// ---------- 已知招聘平台 ----------

struct SiteRule {
    /// 站点域名（含子域名），同时用来在没有来源地址的网页文件里认出站点。
    domain: &'static str,
    read: fn(&Page) -> Option<JobDetail>,
}

/// 加新平台：照 Moka 的样子写一个按页面结构读字段的函数，登记在这里。
const SITES: &[SiteRule] = &[SiteRule {
    domain: "mokahr.com",
    read: moka_job,
}];

fn site_job(page: &Page, url: Option<&str>) -> Option<JobDetail> {
    let host = url.and_then(|u| url_host(u).ok());
    SITES
        .iter()
        .filter(|site| {
            // 另存为的网页文件可能没有来源地址，退而看页面里引用的资源域名。
            host.as_deref()
                .is_some_and(|h| host_matches(h, site.domain))
                || page.mentions(site.domain)
        })
        .find_map(|site| (site.read)(page))
}

/// Moka（app.mokahr.com）岗位详情页，按渲染后的 DOM 读。类名后缀是构建哈希，按前缀匹配。
fn moka_job(page: &Page) -> Option<JobDetail> {
    // 「职位信息」卡片：每行是「标签 / 值」两段文字。
    let rows: Vec<(String, String)> = page
        .by_class_prefix("info-row-")
        .filter_map(|row| match text_lines(row).as_slice() {
            [label, value, ..] => Some((label.clone(), value.clone())),
            _ => None,
        })
        .collect();
    let row = |labels: &[&str]| {
        rows.iter()
            .find(|(l, _)| labels.contains(&l.as_str()))
            .and_then(|(_, v)| non_empty(v))
    };

    let title = page
        .by_class_prefix("title-")
        .next()
        .and_then(|inner| non_empty(&one_line(inner)))
        .or_else(|| row(&["职位名称", "Job Title"]))?;

    // 标题下面那行：「营销类 | 广东·深圳市」。`info-row-` / `info-container-` 是上面的卡片。
    let header: Vec<String> = page
        .elements(|t| {
            t.has_class_prefix("info-")
                && !t.has_class_prefix("info-row-")
                && !t.has_class_prefix("info-container-")
        })
        .next()
        .map(|inner| one_line(inner).split('|').filter_map(non_empty).collect())
        .unwrap_or_default();

    // 页面标题形如「九号公司 - 校园招聘」。
    let company = page
        .by_name("title")
        .next()
        .and_then(|t| one_line(t).split(" - ").next().and_then(non_empty))
        .or_else(|| page.meta("og:site_name"))
        .unwrap_or_default();

    Some(JobDetail {
        title,
        company,
        jd_text: page
            .by_class_prefix("job-description-")
            .next()
            .and_then(|inner| non_empty(&html_to_text(inner))),
        department: row(&["所属部门", "部门", "Department"]),
        location: row(&["工作地点", "工作城市", "Location"])
            .or_else(|| header.get(1).cloned())
            .map(|l| simplify_location(&l)),
        category: row(&["职位类别", "职位类型", "Job Category"])
            .or_else(|| header.first().cloned()),
        ..Default::default()
    })
}

/// 「广东·深圳市」→「深圳市」
fn simplify_location(raw: &str) -> String {
    raw.rsplit('·').next().unwrap_or(raw).trim().to_string()
}

// ---------- 通用兜底 ----------

/// 招聘门户首页常见的标题，不是岗位名。
const PORTAL_HEADINGS: &[&str] = &[
    "校园招聘",
    "社会招聘",
    "实习生招聘",
    "招聘官网",
    "加入我们",
    "职位列表",
    "职位详情",
    "careers",
    "jobs",
    "join us",
];

fn looks_like_job_title(title: &str, company: &str) -> bool {
    let len = title.chars().count();
    let lower = title.to_lowercase();
    (2..=60).contains(&len)
        && title != company
        && !PORTAL_HEADINGS.iter().any(|h| lower.ends_with(h))
}

/// 只认页面里唯一的 `<h1>`，其次 `og:title`。`<title>` 往往是「站点名 - 栏目」，不用。
fn page_title_job(page: &Page) -> Option<JobDetail> {
    let company = page.meta("og:site_name").unwrap_or_default();
    let h1: Vec<String> = page
        .by_name("h1")
        .map(one_line)
        .filter(|t| !t.is_empty())
        .collect();
    let title = match h1.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
    .or_else(|| page.meta("og:title"))
    .filter(|t| looks_like_job_title(t, &company))?;
    Some(JobDetail {
        title,
        company,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 按 Moka 校招岗位页渲染后的 DOM 结构裁剪出来的样例。
    const MOKA: &str = r#"<html><head><title>示例科技 - 校园招聘</title>
        <script src="https://static-ats.mokahr.com/apply/main.js"></script></head><body>
        <div class="portal-label-lSh2wLmmoi">校园招聘</div>
        <div title="海外公关专员（短交通）" class="title-ROUQFdjmhP sd-foundation-heading-40-axnSz">海外公关专员（短交通）</div>
        <div class="info-UcB_mxJq8y"><span>营销类</span><span class="sd-n30">|</span><span>广东·深圳市</span></div>
        <div id="clamp-container"><div class="job-description-VvfEUGocNE"><p>岗位职责：</p><p><br></p><p>1-项目执行&nbsp;</p></div></div>
        <div id="shadow-line-clamp"><div class="job-description-VvfEUGocNE"><p>岗位职责：</p></div></div>
        <div class="info-container-CwAGvyUTTY">
          <div class="info-row-HSEUCEHeFr"><div><div class="sd-Ellipsis-hiddenContent-12YAZ">职位性质</div><div class="tip">职位性质</div></div>
            <div class="value-mOhfHL5YFY"><div id="clamp-container">全职</div><div id="shadow-line-clamp">全职</div></div></div>
          <div class="info-row-HSEUCEHeFr"><div><div>项目类型</div></div><div class="value-mOhfHL5YFY"><div>-</div></div></div>
          <div class="info-row-HSEUCEHeFr"><div><div>工作地点</div><div>工作地点</div></div>
            <div class="value-mOhfHL5YFY"><div>北京·北京市</div><div>北京·北京市</div></div></div>
        </div></body></html>"#;

    #[test]
    fn reads_moka_job_pages() {
        let (job, source) = detect_job(
            MOKA,
            Some("https://app.mokahr.com/campus-recruitment/x/1#/job/abc"),
        )
        .unwrap();
        assert_eq!(source, DetailSource::Site);
        assert_eq!(job.title, "海外公关专员（短交通）");
        assert_eq!(job.company, "示例科技");
        assert_eq!(job.category.as_deref(), Some("营销类"));
        // 「职位信息」里的工作地点优先于标题下那一行
        assert_eq!(job.location.as_deref(), Some("北京市"));
        assert_eq!(job.department, None);
        assert_eq!(job.jd_text.as_deref(), Some("岗位职责：\n1-项目执行"));
    }

    #[test]
    fn recognizes_moka_files_without_source_url() {
        let (_, source) = detect_job(MOKA, None).unwrap();
        assert_eq!(source, DetailSource::Site);
    }

    #[test]
    fn reads_job_posting_json_ld_from_any_page() {
        let html = r#"<html><head><script type="application/ld+json">
            {"@context":"https://schema.org","@type":"JobPosting","title":"后端工程师",
             "hiringOrganization":{"@type":"Organization","name":"示例科技"},
             "jobLocation":{"address":{"addressLocality":"上海"}},
             "description":"<p>负责 &amp; 维护服务</p>"}
            </script></head><body>ignored</body></html>"#;
        let (job, source) = detect_job(html, None).unwrap();
        assert_eq!(source, DetailSource::Structured);
        assert_eq!(job.title, "后端工程师");
        assert_eq!(job.company, "示例科技");
        assert_eq!(job.location.as_deref(), Some("上海"));
        assert_eq!(job.jd_text.as_deref(), Some("负责 & 维护服务"));
        assert!(json_ld_job(&Page::new("<html><body>no data</body></html>")).is_none());
    }

    #[test]
    fn json_ld_wins_over_page_structure() {
        let html = MOKA.replace(
            "</head>",
            r#"<script type="application/ld+json">{"@type":"JobPosting","title":"后端工程师","hiringOrganization":{"name":"甲"}}</script></head>"#,
        );
        let (job, source) = detect_job(&html, None).unwrap();
        assert_eq!(source, DetailSource::Structured);
        assert_eq!(job.title, "后端工程师");
    }

    #[test]
    fn falls_back_to_single_h1_or_og_title() {
        let html = r#"<head><meta property="og:site_name" content="示例科技"></head>
            <body><h1 class="x">数据分析师</h1></body>"#;
        let (job, source) = detect_job(html, Some("https://jobs.example.com/1")).unwrap();
        assert_eq!(source, DetailSource::Page);
        assert_eq!(job.title, "数据分析师");
        assert_eq!(job.company, "示例科技");

        let html = r#"<meta content="产品经理" property="og:title"><h1>甲</h1><h1>乙</h1>"#;
        assert_eq!(detect_job(html, None).unwrap().0.title, "产品经理");
    }

    #[test]
    fn ignores_portal_headings_and_logo_h1() {
        // Moka 未渲染的页面框架：h1 里只有 logo 图片，标题是「公司 - 校园招聘」
        let shell = r#"<html><head><title>示例科技 - 校园招聘</title>
            <meta name="description" content="2026年示例科技校园招聘正在进行"></head>
            <body><h1><img src="https://public-cdn.mokahr.com/a.png" alt="示例科技"></h1></body></html>"#;
        assert!(detect_job(shell, None).is_none());
        assert!(detect_job("<h1>示例科技校园招聘</h1>", None).is_none());
        assert!(detect_job(r#"<meta property="og:title" content="Careers">"#, None).is_none());
    }
}
