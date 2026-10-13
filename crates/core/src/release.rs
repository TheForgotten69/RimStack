//! Whether a newer RimStack has been published: the latest release on
//! GitHub against the version running.

use crate::say;
use std::time::Duration;

const LATEST: &str = "https://api.github.com/repos/TheForgotten69/RimStack/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// As tagged, without a leading `v`.
    pub version: String,
    /// Its page, where the builds are.
    pub page: String,
}

/// The release in GitHub's answer for `releases/latest`.
pub fn parse(json: &str) -> Option<Release> {
    let answer: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = answer["tag_name"].as_str()?;
    Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        page: answer["html_url"].as_str()?.to_string(),
    })
}

/// Whether `latest` is a later version than `current`: compared number by
/// number, so 0.10.0 is later than 0.9.3; what is not a number counts as 0.
pub fn newer(current: &str, latest: &str) -> bool {
    let numbers = |version: &str| -> Vec<u64> {
        let version = version.trim_start_matches('v');
        let core = version.split(['-', '+']).next().unwrap_or_default();
        core.split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    let (mut now, mut then) = (numbers(current), numbers(latest));
    let parts = now.len().max(then.len());
    now.resize(parts, 0);
    then.resize(parts, 0);
    then > now
}

/// The latest release, or nothing where none has been published. Uses the
/// network; only the request reaches GitHub.
pub fn latest() -> Result<Option<Release>, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let failed = |e: ureq::Error| say!("the download failed: {}", e);
    let asked = agent
        .get(LATEST)
        .header("User-Agent", "RimStack")
        .header("Accept", "application/vnd.github+json")
        .call();
    match asked {
        Ok(mut answer) => Ok(parse(&answer.body_mut().read_to_string().map_err(failed)?)),
        Err(ureq::Error::StatusCode(404)) => Ok(None),
        Err(problem) => Err(failed(problem)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_later_version_is_told_from_an_earlier_one_by_its_numbers() {
        assert!(newer("0.9.3", "0.10.0") && newer("1.2", "v1.2.1") && newer("0.0.0", "0.1.0-beta"));
        assert!(!newer("1.2.0", "1.2") && !newer("2.0.0", "1.9.9") && !newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn a_release_is_read_from_the_answer() {
        let answer = r#"{"tag_name":"v0.3.1","html_url":"https://github.com/o/r/releases/tag/v0.3.1","assets":[]}"#;
        let release = parse(answer).unwrap();
        assert_eq!(
            (release.version.as_str(), release.page.as_str()),
            ("0.3.1", "https://github.com/o/r/releases/tag/v0.3.1")
        );
        assert_eq!(parse(r#"{"message":"Not Found"}"#), None);
    }
}
