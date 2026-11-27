use serde::{Deserialize, Serialize};

pub const COMMIT_TYPES: &[&str] = &[
    "feat", "fix", "docs", "style", "refactor", "perf", "test", "build", "ci", "chore", "revert",
];

pub const WRAP_WIDTH: usize = 72;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitForm {
    pub commit_type: String,
    pub scope: String,
    pub subject: String,
    pub body: String,
    pub wrap_72: bool,
    pub breaking: String,
    pub issues: String,
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
            breaking: String::new(),
            issues: String::new(),
            skip_ci: false,
        }
    }
}

/// Formats a [`CommitForm`] into a Conventional Commits message.
pub fn format_commit_message(form: &CommitForm) -> String {
    let commit_type = form.commit_type.trim();
    let commit_type = if commit_type.is_empty() { "feat" } else { commit_type };
    let scope = form.scope.trim();
    let subject = form.subject.trim();

    let header = if scope.is_empty() {
        format!("{commit_type}: {subject}")
    } else {
        format!("{commit_type}({scope}): {subject}")
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
        let with_prefix = if breaking.starts_with("BREAKING CHANGE:") {
            breaking.to_string()
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
        sections.push(format_issues(issues));
    }

    if form.skip_ci {
        sections.push("[skip ci]".to_string());
    }

    sections.join("\n\n")
}

/// Parses an existing commit message back into a [`CommitForm`] for re-editing.
pub fn parse_commit_message(raw: &str, default_type: &str, default_wrap_72: bool) -> CommitForm {
    let mut form = CommitForm {
        commit_type: if COMMIT_TYPES.contains(&default_type) {
            default_type.to_string()
        } else {
            "feat".to_string()
        },
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

    if let Some((t, scope, subject)) = parse_header(first_line) {
        form.commit_type = t;
        form.scope = scope;
        form.subject = subject;
    } else {
        form.subject = first_line.to_string();
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
        if let Some(after) = block.strip_prefix("BREAKING CHANGE:") {
            form.breaking = after.trim().to_string();
            continue;
        }
        if let Some(issues) = parse_issues_block(block) {
            form.issues = issues;
            continue;
        }
        body_parts.push(block.to_string());
    }

    form.body = body_parts.join("\n\n");
    form
}

fn parse_header(line: &str) -> Option<(String, String, String)> {
    let (prefix, subject) = line.split_once(':')?;
    let subject = subject.trim().to_string();
    let prefix = prefix.trim().trim_end_matches('!');

    if let Some((t, rest)) = prefix.split_once('(') {
        let scope = rest.strip_suffix(')')?.trim().to_string();
        let t = t.trim();
        if COMMIT_TYPES.contains(&t) {
            return Some((t.to_string(), scope, subject));
        }
    } else if COMMIT_TYPES.contains(&prefix) {
        return Some((prefix.to_string(), String::new(), subject));
    }
    None
}

fn parse_issues_block(block: &str) -> Option<String> {
    if block.contains('\n') {
        return None;
    }
    let lower = block.to_ascii_lowercase();
    for kw in ["closes ", "close ", "fixes ", "fix ", "resolves ", "resolve "] {
        if lower.starts_with(kw) {
            return Some(block[kw.len()..].trim().to_string());
        }
    }
    None
}

fn format_issues(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    for kw in ["closes ", "close ", "fixes ", "fix ", "resolves ", "resolve "] {
        if lower.starts_with(kw) {
            return raw.to_string();
        }
    }
    let items: Vec<String> = raw
        .split(|c: char| c == ',' || c == '，' || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
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
        format!("Closes {}", items.join(", "))
    }
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
            breaking: "旧版单节点配置字段已移除。".into(),
            issues: "#42, 108".into(),
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
        assert!(parsed.skip_ci);

        assert_eq!(format_commit_message(&parsed), original);
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
    fn allows_empty_subject_format_and_parse() {
        let form = CommitForm {
            commit_type: "fix".into(),
            scope: "api".into(),
            subject: String::new(),
            ..CommitForm::default()
        };
        let formatted = format_commit_message(&form);
        assert_eq!(formatted, "fix(api): ");

        let parsed = parse_commit_message(&formatted, "feat", true);
        assert_eq!(parsed.commit_type, "fix");
        assert_eq!(parsed.scope, "api");
        assert_eq!(parsed.subject, "");
    }
}
