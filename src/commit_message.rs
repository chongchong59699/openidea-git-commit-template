use crate::i18n::UiLocale;
use serde::{Deserialize, Serialize};

pub const COMMIT_TYPES: &[(&str, &str, &str)] = &[
    ("feat", "新功能、新特性", "一个新功能"),
    ("fix", "修复 Bug", "错误修复"),
    ("docs", "编辑文档", "仅文档更改"),
    ("style", "代码格式修改", "不影响代码含义的更改（空格、格式、分号等）"),
    ("refactor", "代码重构", "既不修复错误也不添加功能的代码更改"),
    ("perf", "代码优化", "提高性能的代码更改"),
    ("test", "测试用例新增/修改", "添加缺失的测试或纠正现有测试"),
    ("build", "修改项目构建/依赖", "影响构建系统或外部依赖项的更改"),
    ("ci", "修改持续集成配置", "对 CI 配置文件和脚本的更改"),
    ("chore", "其他修改", "其他不修改 src 或测试文件的更改"),
    ("revert", "恢复上一次提交", "恢复之前的提交"),
];

pub const WRAP_WIDTH: usize = 72;
pub const HEADER_WARN_LENGTH: usize = 72;

fn default_issue_keyword() -> String {
    "Closes".to_string()
}

pub fn is_known_commit_type(t: &str) -> bool {
    COMMIT_TYPES.iter().any(|(id, _, _)| *id == t)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitForm {
    pub commit_type: String,
    pub scope: String,
    pub subject: String,
    pub body: String,
    pub wrap_72: bool,
    #[serde(default)]
    pub breaking_bang: bool,
    pub breaking: String,
    pub issues: String,
    #[serde(default = "default_issue_keyword")]
    pub issue_keyword: String,
    pub skip_ci: bool,
}

impl Default for CommitForm {
    fn default() -> Self {
        Self {
            commit_type: "feat".to_string(),
            scope: String::new(),
            subject: String::new(),
            body: String::new(),
            wrap_72: true,
            breaking_bang: false,
            breaking: String::new(),
            issues: String::new(),
            issue_keyword: default_issue_keyword(),
            skip_ci: false,
        }
    }
}

/// Validates a [`CommitForm`] before writing it to the commit message box.
pub fn validate_form(form: &CommitForm, locale: UiLocale) -> Result<(), String> {
    if form.subject.trim().is_empty() {
        return Err(locale.empty_subject_error().to_string());
    }
    Ok(())
}

/// Formats a [`CommitForm`] into a Conventional Commits message.
pub fn format_commit_message(form: &CommitForm) -> String {
    let commit_type = form.commit_type.trim();
    let commit_type = if commit_type.is_empty() { "feat" } else { commit_type };
    let scope = form.scope.trim();
    let subject = collapse_single_line(&form.subject);
    let bang = if form.breaking_bang { "!" } else { "" };

    let header = if scope.is_empty() {
        format!("{commit_type}{bang}: {subject}")
    } else {
        format!("{commit_type}({scope}){bang}: {subject}")
    };

    let mut sections: Vec<String> = vec![header];

    let body = form.body.trim();
    if !body.is_empty() {
        let formatted_body = if form.wrap_72 {
            wrap_multiline(body, WRAP_WIDTH)
        } else {
            normalize_newlines(body)
        };
        sections.push(formatted_body);
    }

    let breaking = form.breaking.trim();
    if !breaking.is_empty() {
        let with_prefix = if let Some(rest) = breaking
            .strip_prefix("BREAKING CHANGE:")
            .or_else(|| breaking.strip_prefix("BREAKING-CHANGE:"))
        {
            format!("BREAKING CHANGE: {}", rest.trim())
        } else {
            format!("BREAKING CHANGE: {breaking}")
        };
        let formatted_breaking = if form.wrap_72 {
            wrap_multiline(&with_prefix, WRAP_WIDTH)
        } else {
            normalize_newlines(&with_prefix)
        };
        sections.push(formatted_breaking);
    }

    let issues = form.issues.trim();
    if !issues.is_empty() {
        let formatted = format_issues(issues, &form.issue_keyword);
        if !formatted.is_empty() {
            sections.push(formatted);
        }
    }

    if form.skip_ci {
        sections.push("[skip ci]".to_string());
    }

    sections.join("\n\n")
}

/// Parses an existing commit message back into a [`CommitForm`] for re-editing.
pub fn parse_commit_message(raw: &str, default_type: &str, default_wrap_72: bool) -> CommitForm {
    let fallback_type = if is_known_commit_type(default_type) {
        default_type.to_string()
    } else {
        "feat".to_string()
    };

    let mut form = CommitForm {
        commit_type: fallback_type.clone(),
        wrap_72: default_wrap_72,
        ..CommitForm::default()
    };

    let normalized = normalize_newlines(raw);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return form;
    }

    let mut lines = trimmed.lines();
    let first_line = lines.next().unwrap_or_default().trim();

    match parse_header(first_line) {
        Some(ParsedHeader {
            commit_type: Some(t),
            scope,
            breaking_bang,
            subject,
        }) => {
            form.commit_type = t;
            form.scope = scope;
            form.breaking_bang = breaking_bang;
            form.subject = subject;
        }
        Some(ParsedHeader {
            commit_type: None,
            scope,
            breaking_bang,
            subject,
        }) => {
            form.commit_type = fallback_type;
            form.scope = scope;
            form.breaking_bang = breaking_bang;
            form.subject = subject;
        }
        None => {
            form.subject = first_line.to_string();
        }
    }

    let rest = lines.collect::<Vec<_>>().join("\n");
    let rest = rest.trim();
    if rest.is_empty() {
        return form;
    }

    let mut body_parts: Vec<String> = Vec::new();
    for block in rest.split("\n\n") {
        let block = block.trim();
        if block.is_empty() {
            continue;
        }
        if block.eq_ignore_ascii_case("[skip ci]") || block.eq_ignore_ascii_case("[ci skip]") {
            form.skip_ci = true;
            continue;
        }
        if let Some(after) = block
            .strip_prefix("BREAKING CHANGE:")
            .or_else(|| block.strip_prefix("BREAKING-CHANGE:"))
        {
            form.breaking = after.trim().to_string();
            continue;
        }
        if let Some((kw, issues)) = parse_issues_block(block) {
            form.issue_keyword = kw;
            form.issues = issues;
            continue;
        }
        body_parts.push(block.to_string());
    }

    form.body = body_parts.join("\n\n");
    form
}

/// Checks whether a raw commit message satisfies Conventional Commits rules.
/// Returns a list of localized problem descriptions (empty if valid).
pub fn validate_conventional_commit(raw: &str, locale: UiLocale) -> Vec<String> {
    let normalized = normalize_newlines(raw);
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return vec![locale.check_empty_message().to_string()];
    }

    let mut lines = normalized.lines();
    let first_line = lines.next().unwrap_or_default().trim();

    // Allow standard Git-generated commit messages.
    for bypassed in ["Merge ", "Revert \"", "fixup! ", "squash! "] {
        if first_line.starts_with(bypassed) {
            return Vec::new();
        }
    }

    let mut problems = Vec::new();
    match parse_header(first_line) {
        Some(ParsedHeader {
            commit_type: Some(_),
            subject,
            ..
        }) => {
            if subject.trim().is_empty() {
                problems.push(locale.check_missing_subject().to_string());
            }
        }
        Some(ParsedHeader {
            commit_type: None, ..
        }) => {
            let allowed = COMMIT_TYPES
                .iter()
                .map(|(id, _, _)| *id)
                .collect::<Vec<_>>()
                .join("/");
            problems.push(locale.check_unknown_type(&allowed));
        }
        None => {
            problems.push(locale.check_invalid_header().to_string());
        }
    }

    if let Some(second_line) = lines.next() {
        if !second_line.trim().is_empty() {
            problems.push(locale.check_missing_blank_line().to_string());
        }
    }

    problems
}

struct ParsedHeader {
    /// `Some(known_type)` when in `COMMIT_TYPES`, or `None` when it looks like an
    /// unknown ASCII commit type prefix (e.g. `wip: ...`) that should not be duplicated.
    commit_type: Option<String>,
    scope: String,
    breaking_bang: bool,
    subject: String,
}

fn parse_header(line: &str) -> Option<ParsedHeader> {
    let (raw_prefix, raw_subject) = line.split_once(':')?;
    // Require either end-of-line or whitespace after `:` unless prefix is a known type,
    // so URLs like `https://example.com` are never mistaken for commit headers.
    let prefix_trimmed = raw_prefix.trim();
    if prefix_trimmed.is_empty() {
        return None;
    }

    let (without_bang, breaking_bang) = match prefix_trimmed.strip_suffix('!') {
        Some(p) => (p.trim_end(), true),
        None => (prefix_trimmed, false),
    };

    let (raw_type, scope) = if let Some((t, rest)) = without_bang.split_once('(') {
        let sc = rest.strip_suffix(')')?.trim().to_string();
        (t.trim(), sc)
    } else {
        (without_bang, String::new())
    };

    let lower_type = raw_type.to_ascii_lowercase();
    let subject = raw_subject.trim().to_string();

    if is_known_commit_type(&lower_type) {
        return Some(ParsedHeader {
            commit_type: Some(lower_type),
            scope,
            breaking_bang,
            subject,
        });
    }

    // Recognize unknown ASCII type prefixes like `wip: ...` or `deps(ui): ...`
    // only when `: ` (or empty subject) is used and it is not a URL scheme.
    let has_conventional_sep = raw_subject.is_empty() || raw_subject.starts_with(char::is_whitespace);
    let is_ascii_type = (2..=16).contains(&raw_type.len())
        && raw_type.chars().all(|c| c.is_ascii_alphabetic())
        && !matches!(lower_type.as_str(), "http" | "https" | "ftp" | "file");

    if has_conventional_sep && is_ascii_type {
        return Some(ParsedHeader {
            commit_type: None,
            scope,
            breaking_bang,
            subject,
        });
    }

    None
}

const ISSUE_KEYWORDS: &[(&str, &str)] = &[
    ("closes", "Closes"),
    ("close", "Close"),
    ("fixes", "Fixes"),
    ("fix", "Fix"),
    ("resolves", "Resolves"),
    ("resolve", "Resolve"),
];

fn parse_issues_block(block: &str) -> Option<(String, String)> {
    if block.contains('\n') {
        return None;
    }
    let trimmed = block.trim();
    let lower = trimmed.to_ascii_lowercase();

    for &(kw_lower, kw_canonical) in ISSUE_KEYWORDS {
        if let Some(after_kw) = lower.strip_prefix(kw_lower) {
            let orig_after = &trimmed[kw_lower.len()..];
            let rest = if let Some(stripped) = orig_after.strip_prefix(':') {
                stripped.trim()
            } else if after_kw.starts_with(char::is_whitespace) {
                orig_after.trim()
            } else {
                continue;
            };

            if rest.is_empty() {
                continue;
            }

            // Every item in the rest must look like a genuine issue reference
            // (`#123`, `owner/repo#123`, `PROJ-123`) so normal English sentences
            // like `Fix the null check` remain untouched in the commit body.
            let tokens: Vec<&str> = rest
                .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
                .map(str::trim)
                .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("and"))
                .collect();

            if !tokens.is_empty() && tokens.iter().all(|t| is_issue_ref_token(t)) {
                return Some((kw_canonical.to_string(), rest.to_string()));
            }
        }
    }
    None
}

fn is_issue_ref_token(token: &str) -> bool {
    if let Some(num) = token.strip_prefix('#') {
        return !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
    }
    if let Some((repo, num)) = token.split_once('#') {
        let valid_repo = !repo.is_empty()
            && repo
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/'));
        let valid_num = !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
        return valid_repo && valid_num;
    }
    if let Some((prefix, num)) = token.split_once('-') {
        let valid_prefix = prefix.len() >= 2
            && prefix.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && prefix
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        let valid_num = !num.is_empty() && num.chars().all(|c| c.is_ascii_digit());
        return valid_prefix && valid_num;
    }
    false
}

fn format_issues(raw: &str, default_keyword: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    // Preserve explicit leading keyword if the user typed e.g. `Fixes #12` or `Resolves: 34`.
    let lower = trimmed.to_ascii_lowercase();
    for &(kw_lower, kw_canonical) in ISSUE_KEYWORDS {
        if let Some(after_kw) = lower.strip_prefix(kw_lower) {
            let orig_after = &trimmed[kw_lower.len()..];
            let rest = if let Some(stripped) = orig_after.strip_prefix(':') {
                Some(stripped.trim())
            } else if after_kw.starts_with(char::is_whitespace) {
                Some(orig_after.trim())
            } else {
                None
            };
            if let Some(rest) = rest {
                return format_issue_items(kw_canonical, rest);
            }
        }
    }

    let keyword = resolve_issue_keyword(default_keyword);
    format_issue_items(keyword, trimmed)
}

fn resolve_issue_keyword(keyword: &str) -> &'static str {
    let lower = keyword.trim().to_ascii_lowercase();
    for &(kw_lower, kw_canonical) in ISSUE_KEYWORDS {
        if lower == kw_lower {
            return kw_canonical;
        }
    }
    "Closes"
}

fn format_issue_items(keyword: &str, raw_items: &str) -> String {
    let items: Vec<String> = raw_items
        .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("and"))
        .map(|s| {
            if s.chars().all(|c| c.is_ascii_digit()) {
                format!("#{s}")
            } else {
                s.to_string()
            }
        })
        .collect();
    if items.is_empty() {
        String::new()
    } else {
        format!("{keyword} {}", items.join(", "))
    }
}

fn collapse_single_line(s: &str) -> String {
    normalize_newlines(s)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Wraps each line of `text` to at most `max_chars` Unicode scalar values,
/// preferring word boundaries for ASCII text and clean char breaks for CJK/long words.
pub fn wrap_multiline(text: &str, max_chars: usize) -> String {
    normalize_newlines(text)
        .lines()
        .map(|line| wrap_line(line, max_chars))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_line(line: &str, max_chars: usize) -> String {
    if line.chars().count() <= max_chars || max_chars == 0 {
        return line.to_string();
    }

    let mut out: Vec<String> = Vec::new();
    let mut remaining = line;

    while remaining.chars().count() > max_chars {
        let split_byte = remaining
            .char_indices()
            .nth(max_chars)
            .map(|(i, _)| i)
            .unwrap_or(remaining.len());

        let slice = &remaining[..split_byte];
        if let Some(space_idx) = slice.rfind(|c: char| c.is_whitespace()) {
            if space_idx > 0 {
                out.push(slice[..space_idx].trim_end().to_string());
                remaining = remaining[space_idx..].trim_start();
                continue;
            }
        }

        out.push(slice.to_string());
        remaining = &remaining[split_byte..];
    }

    if !remaining.is_empty() {
        out.push(remaining.to_string());
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_full_commit_message() {
        let form = CommitForm {
            commit_type: "fix".into(),
            scope: "workflow".into(),
            subject: "支持多节点工作流逐节点与多规则模型替换并兼容 prompt_template 模式转换".into(),
            body: "调整了工作流节点解析逻辑，兼容旧版 prompt_template 配置。".into(),
            wrap_72: true,
            breaking_bang: false,
            breaking: "旧版单节点配置字段已移除。".into(),
            issues: "#42, 108".into(),
            issue_keyword: "Closes".into(),
            skip_ci: true,
        };

        let msg = format_commit_message(&form);
        assert_eq!(
            msg,
            "fix(workflow): 支持多节点工作流逐节点与多规则模型替换并兼容 prompt_template 模式转换\n\n\
             调整了工作流节点解析逻辑，兼容旧版 prompt_template 配置。\n\n\
             BREAKING CHANGE: 旧版单节点配置字段已移除。\n\n\
             Closes #42, #108\n\n\
             [skip ci]"
        );
    }

    #[test]
    fn formats_minimal_commit_message() {
        let form = CommitForm {
            commit_type: "feat".into(),
            subject: "新增用户登录接口".into(),
            ..CommitForm::default()
        };
        assert_eq!(format_commit_message(&form), "feat: 新增用户登录接口");
    }

    #[test]
    fn wraps_long_english_and_cjk_lines_at_72() {
        let long_en = "This is a deliberately long commit body sentence that should wrap cleanly at seventy two characters without splitting words.";
        let wrapped = wrap_multiline(long_en, 72);
        for line in wrapped.lines() {
            assert!(line.chars().count() <= 72, "line too long: {line}");
        }
        assert!(wrapped.contains('\n'));

        let long_zh = "一".repeat(100);
        let wrapped_zh = wrap_multiline(&long_zh, 72);
        let lines: Vec<&str> = wrapped_zh.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].chars().count(), 72);
        assert_eq!(lines[1].chars().count(), 28);
    }

    #[test]
    fn round_trips_parse_and_format() {
        let original = "refactor(auth): simplify token refresh\n\n\
            Move refresh scheduling into TokenManager.\n\n\
            BREAKING CHANGE: remove legacy refreshToken helper\n\n\
            Closes #12, #34\n\n\
            [skip ci]";

        let parsed = parse_commit_message(original, "feat", true);
        assert_eq!(parsed.commit_type, "refactor");
        assert_eq!(parsed.scope, "auth");
        assert_eq!(parsed.subject, "simplify token refresh");
        assert_eq!(parsed.body, "Move refresh scheduling into TokenManager.");
        assert_eq!(parsed.breaking, "remove legacy refreshToken helper");
        assert_eq!(parsed.issues, "#12, #34");
        assert_eq!(parsed.issue_keyword, "Closes");
        assert!(parsed.skip_ci);

        assert_eq!(format_commit_message(&parsed), original);
    }

    #[test]
    fn preserves_breaking_bang_and_fixes_keyword() {
        let original = "feat(api)!: 移除旧版鉴权接口\n\nFixes #88, GH-102";
        let parsed = parse_commit_message(original, "feat", true);
        assert_eq!(parsed.commit_type, "feat");
        assert_eq!(parsed.scope, "api");
        assert!(parsed.breaking_bang);
        assert_eq!(parsed.subject, "移除旧版鉴权接口");
        assert_eq!(parsed.issue_keyword, "Fixes");
        assert_eq!(parsed.issues, "#88, GH-102");
        assert_eq!(format_commit_message(&parsed), original);
    }

    #[test]
    fn does_not_corrupt_body_starting_with_fix_or_close() {
        let original = "fix(parser): handle empty token\n\nFix the null check when input is empty.\n\nClose connection on timeout.";
        let parsed = parse_commit_message(original, "feat", false);
        assert_eq!(parsed.issues, "");
        assert_eq!(
            parsed.body,
            "Fix the null check when input is empty.\n\nClose connection on timeout."
        );
        assert_eq!(format_commit_message(&parsed), original);
    }

    #[test]
    fn strips_unknown_ascii_type_prefix_instead_of_duplicating() {
        let parsed = parse_commit_message("wip(ui): 调整按钮间距", "feat", true);
        assert_eq!(parsed.commit_type, "feat");
        assert_eq!(parsed.scope, "ui");
        assert_eq!(parsed.subject, "调整按钮间距");
        assert_eq!(format_commit_message(&parsed), "feat(ui): 调整按钮间距");
    }

    #[test]
    fn parses_plain_message_into_subject_and_body() {
        let parsed = parse_commit_message("临时提交\n\n补充说明", "fix", true);
        assert_eq!(parsed.commit_type, "fix");
        assert_eq!(parsed.scope, "");
        assert_eq!(parsed.subject, "临时提交");
        assert_eq!(parsed.body, "补充说明");
    }

    #[test]
    fn rejects_empty_subject_on_validation() {
        let form = CommitForm {
            commit_type: "fix".into(),
            scope: "api".into(),
            subject: "   ".into(),
            ..CommitForm::default()
        };
        assert!(validate_form(&form, UiLocale::En).is_err());
        assert!(validate_form(&form, UiLocale::ZhCn).is_err());
    }

    #[test]
    fn validates_conventional_commits_and_bypasses_merge() {
        assert!(validate_conventional_commit("feat(auth): add oauth2 login", UiLocale::En).is_empty());
        assert!(validate_conventional_commit("Merge branch 'main' into dev", UiLocale::En).is_empty());
        assert!(!validate_conventional_commit("", UiLocale::En).is_empty());
        assert!(!validate_conventional_commit("随手提交一下", UiLocale::ZhCn).is_empty());
        assert!(!validate_conventional_commit("wip: 临时保存", UiLocale::ZhCn).is_empty());
        assert!(!validate_conventional_commit("feat: 标题\n紧贴的第二行", UiLocale::ZhCn).is_empty());
    }
}
