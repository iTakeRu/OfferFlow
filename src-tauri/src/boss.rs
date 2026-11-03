//! BOSS 的公司职位页可能同时展示列表和一个展开的详情。
//! 用「职位描述」和紧邻它的岗位页首定位详情，链接本身不能确定用户选中了哪一项。
use crate::extract::{host_matches, url_host};
use crate::html::{html_to_text, job_card_text, one_line, Page};
use crate::job_page::JobDetail;
use regex::Regex;
use std::sync::LazyLock;

pub(crate) fn is_url(url: &str) -> bool {
    url_host(url).is_ok_and(|h| host_matches(&h, "zhipin.com"))
}

pub(crate) fn is_company_listing(url: &str) -> bool {
    is_url(url) && url::Url::parse(url.trim()).is_ok_and(|u| u.path().starts_with("/gongsi/job/"))
}

static PAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:[0-9]+(?:\.[0-9]+)?\s*k?\s*[-~–至]\s*[0-9]+(?:\.[0-9]+)?\s*(?:k|元/(?:天|时)|万/月)(?:\s*[·•]\s*[0-9]+薪)?|薪资面议|面议)$").unwrap()
});
static MONTHLY_K: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([0-9]+)\s*k?\s*[-~–至]\s*([0-9]+)\s*k(?:\s*[·•]\s*[0-9]+薪)?$").unwrap()
});
static QUALIFICATIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^([^\s|·]{2,24}?)\s*[|·]?\s*((?:[0-9]+(?:\s*[-–至]\s*[0-9]+)?\s*年(?:以内|以上)?|经验不限|在校/应届|应届生))\s*[|·]?\s*(本科|硕士|博士|大专|中专|高中|学历不限|初中及以下)").unwrap()
});

fn unique_class<'a>(page: &Page<'a>, classes: &[&str]) -> Option<&'a str> {
    let mut found = page.elements(|tag| {
        tag.attr("class")
            .is_some_and(|c| c.split_whitespace().any(|name| classes.contains(&name)))
    });
    let inner = found.next()?;
    found.next().is_none().then_some(inner)
}

pub(crate) fn from_html(page: &Page, html: &str) -> Option<JobDetail> {
    let visible = job_card_text(page.by_name("body").next().unwrap_or(html));
    let mut job = from_text(&visible);
    // 详情页有独立的页首、正文容器时，即使薪资使用了图标字体，也能读其他字段。
    if let (Some(header), Some(description)) = (
        unique_class(page, &["job-detail-header", "job-banner"]),
        unique_class(page, &["job-sec-text"]),
    ) {
        let meta = job_card_text(header);
        let header = Page::new(header);
        let title = header
            .by_name("h1")
            .next()
            .or_else(|| unique_class(&header, &["job-name"]));
        if let Some(title) = title.map(one_line).filter(|s| !s.is_empty()) {
            let pay = unique_class(&header, &["salary", "job-salary"])
                .map(one_line)
                .unwrap_or_default();
            let body = html_to_text(description);
            if !body.trim().is_empty() {
                let same_title = job.as_ref().filter(|j| j.title == title);
                let company = same_title.map(|j| j.company.as_str()).unwrap_or("");
                let mut selected = detail(&title, company, &pay, &meta, &body);
                if selected.location.is_none() {
                    selected.location = unique_class(&header, &["text-city"]).map(one_line);
                }
                if let Some(previous) = same_title {
                    if pay.is_empty() && previous.salary_min.is_some() {
                        selected.salary_min = previous.salary_min;
                        selected.salary_max = previous.salary_max;
                        selected.jd_text = previous.jd_text.clone();
                    }
                }
                job = Some(selected);
            }
        }
    }
    if let Some(j) = &mut job {
        if j.company.is_empty() {
            if let Some(attr) = unique_class(page, &["boss-info-attr"]) {
                let info = Page::new(attr);
                let label = one_line(attr);
                if let Some(org) = label
                    .split_once('·')
                    .map(|(s, _)| s.trim().to_string())
                    .or_else(|| info.by_name("span").next().map(one_line))
                {
                    j.company = org;
                }
            }
        }
    }
    job
}

fn heading(line: &str) -> bool {
    matches!(
        line.trim_end_matches([':', '：']).trim(),
        "职位描述" | "岗位描述"
    )
}

pub(crate) fn from_text(text: &str) -> Option<JobDetail> {
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let mut sections = lines.iter().enumerate().filter(|(_, l)| heading(l));
    let (index, _) = sections.next()?;
    if sections.next().is_some() {
        return None;
    }
    let header = &lines[index.saturating_sub(24)..index];
    let (pos, line, pay) = header.iter().enumerate().rev().find_map(|(i, line)| {
        let line = line
            .split("感兴趣")
            .next()?
            .split("立即沟通")
            .next()?
            .trim();
        Some((i, line, PAY.find(line)?))
    })?;
    let title = match line[..pay.start()].trim() {
        "" => *header.get(pos.checked_sub(1)?)?,
        title => title,
    };
    if !(2..=80).contains(&title.chars().count()) || matches!(title, "立即沟通" | "感兴趣") {
        return None;
    }
    let meta = header[pos + 1..].join("\n");
    let (body, company) = description_body(&lines[index + 1..]);
    if body.is_empty() {
        return None;
    }
    Some(detail(
        title,
        company,
        pay.as_str(),
        &meta,
        &body.join("\n"),
    ))
}

/// A detail page may continue with rankings, recommendations and other recruiters.
/// Limit the search to this job's card, then use its recruiter signature as the end of the JD.
fn description_body<'a, 't>(lines: &'a [&'t str]) -> (&'a [&'t str], &'t str) {
    let footer = lines
        .iter()
        .position(|s| {
            matches!(
                s.trim_end_matches([':', '：']),
                "工作地址"
                    | "工作地点"
                    | "公司介绍"
                    | "相似职位"
                    | "推荐职位"
                    | "竞争力分析"
                    | "BOSS 安全提示"
                    | "BOSS安全提示"
            )
        })
        .unwrap_or(lines.len());
    let body = &lines[..footer];
    for (i, line) in body.iter().enumerate() {
        let Some((left, right)) = line.split_once('·') else {
            continue;
        };
        // Chrome can expose the company, separator and role as one, two or three text runs.
        let mut preceding = &body[..i];
        let mut following = &body[i + 1..];
        let org = if left.trim().is_empty() {
            preceding.split_off_last().copied().unwrap_or("")
        } else {
            left.trim()
        };
        let role = if right.trim().is_empty() {
            following.split_off_first().copied().unwrap_or("")
        } else {
            right.trim()
        };
        let status = preceding.last().copied().unwrap_or("");
        let recruiter =
            status.chars().count() <= 20 && (status.ends_with("在线") || status.ends_with("活跃"));
        if !org.is_empty()
            && org.chars().count() <= 40
            && !org.starts_with(['-', '【'])
            && !org.contains(['。', '，', '：'])
            && role.chars().count() <= 20
            && !role.is_empty()
            && !role.contains(['。', '，', '·'])
            && (recruiter
                || following.is_empty()
                    && ["人事", "招聘", "HR", "经理", "负责人", "总监", "创始人"]
                        .iter()
                        .any(|r| role.contains(r)))
        {
            let mut body = preceding;
            if recruiter {
                body = &body[..body.len() - 1];
                if (status == "在线" || status.ends_with("活跃") && status.chars().count() <= 6)
                    && body.last().is_some_and(|s| s.chars().count() <= 6)
                {
                    body = &body[..body.len() - 1];
                }
            }
            return (body, org);
        }
    }
    (body, "")
}

fn detail(title: &str, company: &str, pay: &str, meta: &str, body: &str) -> JobDetail {
    let meta = QUALIFICATIONS.captures(meta);
    // 表单以整数 K 为单位。薪数、日薪及无法无损表示的范围仍保留在快照中。
    let salary = MONTHLY_K.captures(pay).and_then(|c| {
        let min = c[1].parse().ok()?;
        let max = c[2].parse().ok()?;
        (min > 0 && min <= max).then_some((min, max))
    });
    let mut summary = title.to_string();
    if !pay.is_empty() && !pay.chars().any(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)) {
        summary.push('\n');
        summary.push_str(pay);
    }
    if let Some(m) = &meta {
        summary.push_str(&format!("\n{} {} {}", &m[1], &m[2], &m[3]));
    }
    JobDetail {
        title: title.to_string(),
        company: company.to_string(),
        jd_text: Some(format!("{summary}\n\n{body}")),
        location: meta.map(|m| m[1].to_string()),
        salary_min: salary.map(|p| p.0),
        salary_max: salary.map(|p| p.1),
        ..Default::default()
    }
}

/// Chrome's accessible text may split one heading or phrase between text runs.
/// Repair only the JD body of our formatted snapshot; keep its metadata and list boundaries.
pub(crate) fn clean_accessible_snapshot(text: &str) -> String {
    let Some((header, body)) = text.split_once("\n\n") else {
        return text.to_owned();
    };
    static ITEM: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(?:\d+[.、)）]|[-•●【])").unwrap());
    let mut out = format!("{header}\n\n");
    let mut previous = "";
    let lines: Vec<_> = body.split('\n').collect();
    let (body, _) = description_body(&lines);
    for (index, &line) in body.iter().enumerate() {
        let next = line.trim_start();
        if index > 0 {
            // A matching close on the next line, e.g. “（测试\n项目说明）”.
            let bracket = [('【', '】'), ('（', '）'), ('(', ')')]
                .iter()
                .any(|&(open, close)| {
                    previous
                        .rsplit_once(open)
                        .is_some_and(|(_, tail)| !tail.contains(close))
                        && next
                            .split_once(close)
                            .is_some_and(|(head, _)| !head.contains(open))
                });
            // An orphaned final character just before an inline comma, e.g. “手\n册，拥有…”.
            let mut chars = next.chars();
            let orphan = chars.next().is_some_and(is_han)
                && matches!(chars.next(), Some('，' | '、'))
                && previous.ends_with(is_han);
            if !previous.is_empty() && !ITEM.is_match(next) && (bracket || orphan) {
                if previous.ends_with(|c: char| c.is_ascii_alphanumeric())
                    && next.starts_with(|c: char| c.is_ascii_alphanumeric())
                {
                    out.push(' ');
                }
            } else {
                out.push('\n');
            }
        }
        out.push_str(line);
        previous = line.trim_end();
    }
    out
}

fn is_han(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job_page::{detect_job, DetailSource};
    use crate::snapshot::capture_snapshot_from_text;

    const URL: &str = "https://www.zhipin.com/gongsi/job/example~.html";
    const TITLE: &str = "产品运营（示例项目）";
    // 合成样例，模拟职位列表旁已展开的详情。
    const JD: &str = "<div>【关于我们】</div><p>为示例团队编写指南。</p>\
        <div>【岗位职责】</div><p>维护项目文档。</p>\
        <div>【职位要求】</div><p>具有 2-4 年文档维护经验。</p>\
        <div>【岗位亮点】</div><p>提供入职培训。</p>";

    #[test]
    fn selected_company_job_and_copied_detail_match() {
        let html = format!(
            r#"<body><aside>其他岗位 8-12K</aside><div><span>{TITLE}</span><span>12-24K·13薪</span></div>
          <p><a>杭州</a><span>2-4年</span><span>本科</span></p><a>感兴趣</a><div>立即沟通</div>
          <h3>职位描述：</h3>{JD}<div>甲专员 在线</div><p>示例科技 · 人事专员</p>
          <h3>工作地址：</h3><div>推荐职位</div></body>"#
        );
        let (j, source) = detect_job(&html, Some(URL)).unwrap();
        assert_eq!(source, DetailSource::Site);
        assert_eq!(j.title, TITLE);
        assert_eq!(j.company, "示例科技");
        assert_eq!(j.location.as_deref(), Some("杭州"));
        assert_eq!((j.salary_min, j.salary_max), (Some(12), Some(24)));
        let text = j.jd_text.unwrap();
        for expected in [
            "12-24K·13薪",
            "杭州 2-4年 本科",
            "关于我们",
            "岗位职责",
            "2-4 年",
            "岗位亮点",
        ] {
            assert!(text.contains(expected));
        }
        for noise in ["其他岗位", "甲专员", "工作地址", "推荐职位", "立即沟通"] {
            assert!(!text.contains(noise));
        }
        let copied = format!("{TITLE}\n12-24K·13薪\n杭州\n2-4年\n本科\n立即沟通\n职位描述：\n负责项目运营。\n示例科技 · 人事专员");
        let draft = capture_snapshot_from_text(copied.clone(), None).unwrap();
        assert_eq!(draft.text, copied);
        assert_eq!(draft.detail.unwrap().company, "示例科技");
    }

    #[test]
    fn detail_page_scopes_both_company_and_description_to_the_job_card() {
        let title = "数据分析（示例项目）";
        let jd = "数据校验\n报表维护\n【岗位职责】：\n-维护测试用例\n【任职要求】：\n-熟悉基本统计方法\n请提供作品示例。";
        let summary = format!("{title}\n11-18K·14薪\n杭州 2-4年 本科");
        let expected = format!("{summary}\n\n{jd}");
        for signature in [
            "示例科技\n·\nHR",
            "示例科技 · HR",
            "示例科技\n· HR",
            "示例科技 ·\nHR",
        ] {
            let trailer = format!("乙女士\n在线\n{signature}\n竞争力分析\n查看完整个人竞争力\n个人综合排名：在\nBOSS 安全提示\n推荐职位\n其他公司 · HR");
            let page = format!("招聘中\n{summary}\n感兴趣\n立即沟通\n职位描述\n{jd}\n{trailer}");
            let job = from_text(&page).unwrap();
            assert_eq!(job.company, "示例科技");
            assert_eq!(job.location.as_deref(), Some("杭州"));
            assert_eq!((job.salary_min, job.salary_max), (Some(11), Some(18)));
            assert_eq!(job.jd_text.as_ref(), Some(&expected));
            // Already saved by the old one-click reader with the recruiter and footer included.
            assert_eq!(
                clean_accessible_snapshot(&format!("{expected}\n{trailer}")),
                expected
            );
        }
    }

    #[test]
    fn ignores_recruiters_in_recommendations_and_preserves_dot_in_prose() {
        let lines = [
            "【岗位职责】",
            "协作对象 · HR 和业务团队",
            "沟通岗位需求。",
            "推荐职位",
            "在线",
            "另一家公司",
            "·",
            "HR",
        ];
        assert_eq!(description_body(&lines), (&lines[..3], ""));
    }

    #[test]
    fn long_list_and_verification_are_not_jobs() {
        assert!(is_company_listing(URL));
        assert!(!is_company_listing(
            "https://www.zhipin.com/job_detail/abc.html"
        ));
        for html in [
            "<h1>示例科技 在招职位</h1><div>运营 20-40K</div>",
            "<h1>请完成安全验证</h1>",
        ] {
            let html = format!("{html}<p>{}</p>", "更多信息 ".repeat(100));
            assert!(detect_job(&html, Some(URL)).is_none());
        }
    }

    #[test]
    fn daily_rate_does_not_fill_monthly_salary() {
        let job = from_text("实习运营\n200-300元/天\n杭州 经验不限 本科\n职位描述\n协助项目运营。")
            .unwrap();
        assert_eq!((job.salary_min, job.salary_max), (None, None));
        assert!(job.jd_text.unwrap().contains("200-300元/天"));
    }

    #[test]
    fn font_encoded_salary_keeps_other_dom_fields() {
        let html =
            r#"<body><div class="job-banner"><h1>产品运营</h1><span class="salary">&#xe001;</span>
            <p>杭州 2-4年 本科</p></div><div class="job-sec-text">维护示例文档。</div>
            <span class="boss-info-attr"><span>示例科技</span><span>人事专员</span></span></body>"#
                .replace("&#xe001;", "\u{e001}");
        let (job, _) = detect_job(&html, Some(URL)).unwrap();
        assert_eq!(job.company, "示例科技");
        assert_eq!(job.salary_min, None);
        assert_eq!(job.location.as_deref(), Some("杭州"));
        assert!(!job.jd_text.unwrap().contains('\u{e001}'));
    }
}
