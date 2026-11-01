use semver::Version;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Duration;

const LATEST_RELEASE: &str = "https://api.github.com/repos/iTakeRu/OfferFlow/releases/latest";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    status: &'static str,
    latest_version: Option<String>,
}

fn status(value: &'static str) -> UpdateCheck {
    UpdateCheck {
        status: value,
        latest_version: None,
    }
}

fn compare_release(release: Release, current: &str) -> Result<UpdateCheck, ()> {
    let latest = Version::parse(
        release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name),
    )
    .map_err(|_| ())?;
    let current = Version::parse(current).map_err(|_| ())?;
    if release.draft || release.prerelease || !latest.pre.is_empty() {
        return Err(());
    }
    Ok(UpdateCheck {
        status: if latest.cmp_precedence(&current).is_gt() {
            "available"
        } else {
            "current"
        },
        latest_version: Some(latest.to_string()),
    })
}

fn http_error(code: u16) -> UpdateCheck {
    status(match code {
        404 => "unavailable",
        403 | 429 => "rateLimited",
        _ => "networkError",
    })
}

/// Only fetch public release metadata, on explicit user request.
#[tauri::command(async)]
pub fn check_for_updates() -> UpdateCheck {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let response = match agent
        .get(LATEST_RELEASE)
        .set(
            "User-Agent",
            concat!("OfferFlow/", env!("CARGO_PKG_VERSION")),
        )
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(response) => response,
        Err(ureq::Error::Status(code, _)) => return http_error(code),
        Err(_) => return status("networkError"),
    };
    let mut body = String::new();
    if response
        .into_reader()
        .take(1_048_577)
        .read_to_string(&mut body)
        .is_err()
        || body.len() > 1_048_576
    {
        return status("invalidResponse");
    }
    serde_json::from_str::<Release>(&body)
        .map_err(|_| ())
        .and_then(|release| compare_release(release, env!("CARGO_PKG_VERSION")))
        .unwrap_or_else(|_| status("invalidResponse"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            draft: false,
            prerelease: false,
        }
    }

    #[test]
    fn compares_versions_numerically_and_ignores_build_metadata() {
        assert_eq!(
            compare_release(release("v1.10.0"), "1.9.0").unwrap().status,
            "available"
        );
        assert_eq!(
            compare_release(release("v1.0.0"), "1.0.0").unwrap().status,
            "current"
        );
        assert_eq!(
            compare_release(release("1.0.0+build2"), "1.0.0+build1")
                .unwrap()
                .status,
            "current"
        );
        assert_eq!(
            compare_release(release("v0.9.0"), "1.0.0").unwrap().status,
            "current"
        );
    }

    #[test]
    fn rejects_invalid_and_unpublished_releases() {
        for tag in ["not-a-version", "v2.0.0-beta.1", "v1.0"] {
            assert!(compare_release(release(tag), "1.0.0").is_err());
        }
        let mut candidate = release("v2.0.0");
        candidate.draft = true;
        assert!(compare_release(candidate, "1.0.0").is_err());
        let mut candidate = release("v2.0.0");
        candidate.prerelease = true;
        assert!(compare_release(candidate, "1.0.0").is_err());
    }

    #[test]
    fn missing_or_failed_requests_never_report_up_to_date() {
        assert_eq!(http_error(404).status, "unavailable");
        assert_eq!(http_error(403).status, "rateLimited");
        assert_eq!(http_error(429).status, "rateLimited");
        assert_eq!(http_error(500).status, "networkError");
    }
}
