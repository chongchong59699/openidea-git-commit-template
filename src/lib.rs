mod commit_message;
mod i18n;

use commit_message::{
    CommitForm, HEADER_WARN_LENGTH, format_commit_message, is_known_commit_type,
    parse_commit_message, validate_conventional_commit, validate_form,
};
use i18n::UiLocale;
use openidea_plugin::commit::{CommitChecks, CommitProblem};
use openidea_plugin::views::{Body, View, ViewEvent, Viewport, Views};
use openidea_plugin::{
    CommitDraft, Error, Invocation, NotificationLevel, Plugin, host, register_plugin,
};
use serde_json::{Value, json};

pub const DIALOG_ID: &str = "editor";
pub const CHECK_ID: &str = "conventional";

const STORAGE_SCOPES: &str = "recent_scopes";
const STORAGE_LAST_TYPE: &str = "last_type";
const STORAGE_WRAP_72: &str = "wrap_72";
const MAX_SCOPES: usize = 20;

#[derive(Default)]
pub struct GitCommitTemplate;

impl GitCommitTemplate {
    fn current_locale() -> (String, UiLocale) {
        let tag = host::locale();
        let locale = UiLocale::from_tag(&tag);
        (tag, locale)
    }

    fn load_scopes() -> Vec<String> {
        let Ok(Some(raw)) = host::storage::get(STORAGE_SCOPES) else {
            return Vec::new();
        };
        serde_json::from_str::<Vec<String>>(&raw).unwrap_or_default()
    }

    fn save_scopes(scopes: &[String]) {
        if let Ok(json) = serde_json::to_string(scopes) {
            let _ = host::storage::set(STORAGE_SCOPES, &json);
        }
    }

    fn remember_scope(scope: &str) -> Vec<String> {
        let trimmed = scope.trim();
        let mut scopes = Self::load_scopes();
        if trimmed.is_empty() {
            return scopes;
        }
        scopes.retain(|s| s != trimmed);
        scopes.insert(0, trimmed.to_string());
        scopes.truncate(MAX_SCOPES);
        Self::save_scopes(&scopes);
        scopes
    }

    fn remove_scope(scope: &str) -> Vec<String> {
        let mut scopes = Self::load_scopes();
        scopes.retain(|s| s != scope);
        Self::save_scopes(&scopes);
        scopes
    }

    fn load_defaults() -> (String, bool) {
        let last_type = host::storage::get(STORAGE_LAST_TYPE)
            .ok()
            .flatten()
            .filter(|s| is_known_commit_type(s))
            .unwrap_or_else(|| "feat".to_string());
        let wrap_72 = host::storage::get(STORAGE_WRAP_72)
            .ok()
            .flatten()
            .map(|s| s != "false")
            .unwrap_or(true);
        (last_type, wrap_72)
    }

    fn save_defaults(commit_type: &str, wrap_72: bool) {
        let trimmed = commit_type.trim();
        if is_known_commit_type(trimmed) {
            let _ = host::storage::set(STORAGE_LAST_TYPE, trimmed);
        }
        let _ = host::storage::set(STORAGE_WRAP_72, if wrap_72 { "true" } else { "false" });
    }

    fn load_form_from_pending_commit() -> CommitForm {
        let (default_type, default_wrap_72) = Self::load_defaults();
        match host::pending_commit() {
            Ok(Some(draft)) => parse_commit_message(&draft.message, &default_type, default_wrap_72),
            _ => CommitForm {
                commit_type: default_type,
                wrap_72: default_wrap_72,
                ..CommitForm::default()
            },
        }
    }

    fn build_preview_json(form: &CommitForm) -> Value {
        let formatted = format_commit_message(form);
        let first_line = formatted.lines().next().unwrap_or_default();
        let header_len = first_line.chars().count();
        let line_count = formatted.lines().count().max(1);
        json!({
            "message": formatted,
            "headerLen": header_len,
            "headerWarn": header_len > HEADER_WARN_LENGTH,
            "lineCount": line_count,
        })
    }

    fn post_json(message: &Value) {
        let _ = host::post_view_message(DIALOG_ID, &message.to_string());
    }

    fn send_init_from_pending_commit() {
        let (locale_tag, _) = Self::current_locale();
        let form = Self::load_form_from_pending_commit();
        let scopes = Self::load_scopes();
        let preview = Self::build_preview_json(&form);
        Self::post_json(&json!({
            "op": "init",
            "locale": locale_tag,
            "form": form,
            "scopes": scopes,
            "preview": preview,
        }));
    }

    fn open_dialog() -> Result<(), Error> {
        let (_, locale) = Self::current_locale();
        if host::pending_commit()?.is_none() {
            host::notify(
                NotificationLevel::Warning,
                "Git Commit Template",
                locale.no_git_repo_warning(),
            );
            return Ok(());
        }

        host::reveal_view(DIALOG_ID)?;
        Self::send_init_from_pending_commit();
        Ok(())
    }

    fn apply_form(&mut self, form: CommitForm) -> Result<(), Error> {
        let (_, locale) = Self::current_locale();
        if let Err(validation_msg) = validate_form(&form, locale) {
            Self::post_json(&json!({
                "op": "error",
                "message": validation_msg,
            }));
            return Ok(());
        }

        let formatted = format_commit_message(&form);
        Self::save_defaults(&form.commit_type, form.wrap_72);
        let _ = Self::remember_scope(&form.scope);

        let Some(draft) = host::pending_commit()? else {
            Self::post_json(&json!({
                "op": "error",
                "message": locale.no_git_repo_warning(),
            }));
            return Ok(());
        };

        match host::set_commit_message(draft.revision, &formatted) {
            Ok(()) => {
                Self::post_json(&json!({ "op": "applied" }));
                Ok(())
            }
            Err(Error::Stale) => {
                if let Some(fresh) = host::pending_commit()? {
                    host::set_commit_message(fresh.revision, &formatted)?;
                    Self::post_json(&json!({ "op": "applied" }));
                    Ok(())
                } else {
                    Self::post_json(&json!({
                        "op": "error",
                        "message": locale.stale_commit_error(),
                    }));
                    Ok(())
                }
            }
            Err(err) => {
                Self::post_json(&json!({
                    "op": "error",
                    "message": locale.write_failed_error(&err.to_string()),
                }));
                Err(err)
            }
        }
    }
}

impl Plugin for GitCommitTemplate {
    fn run_command(
        &mut self,
        id: &str,
        _ctx: &Invocation,
        _arg: Option<&str>,
    ) -> Result<Option<String>, Error> {
        match id {
            "open" => {
                Self::open_dialog()?;
                Ok(None)
            }
            _ => Err(Error::NotFound(format!("command {id}"))),
        }
    }
}

impl CommitChecks for GitCommitTemplate {
    fn check(
        &mut self,
        check: &str,
        draft: &CommitDraft,
        _ctx: &Invocation,
    ) -> Result<Vec<CommitProblem>, Error> {
        if check != CHECK_ID {
            return Err(Error::NotFound(format!("commit check {check}")));
        }
        let (_, locale) = Self::current_locale();
        let problems = validate_conventional_commit(&draft.message, locale)
            .into_iter()
            .map(CommitProblem::new)
            .collect();
        Ok(problems)
    }
}

impl Views for GitCommitTemplate {
    fn render(
        &mut self,
        view: &str,
        _viewport: Option<Viewport>,
        _ctx: &Invocation,
    ) -> Result<View, Error> {
        if view != DIALOG_ID {
            return Err(Error::NotFound(view.into()));
        }
        let (_, locale) = Self::current_locale();
        Ok(View::new(Body::Message(
            locale.view_fallback_message().into(),
        )))
    }

    fn handle(&mut self, view: &str, event: ViewEvent, _ctx: &Invocation) -> Result<(), Error> {
        if view != DIALOG_ID {
            return Err(Error::NotFound(view.into()));
        }
        if let ViewEvent::Message(raw) = event {
            let Ok(v) = serde_json::from_str::<Value>(&raw) else {
                return Ok(());
            };
            match v.get("op").and_then(Value::as_str) {
                Some("ready") | Some("reload") => {
                    Self::send_init_from_pending_commit();
                    Ok(())
                }
                Some("preview") => {
                    if let Some(form_val) = v.get("form") {
                        if let Ok(form) = serde_json::from_value::<CommitForm>(form_val.clone()) {
                            let preview = Self::build_preview_json(&form);
                            Self::post_json(&json!({
                                "op": "preview",
                                "preview": preview,
                            }));
                        }
                    }
                    Ok(())
                }
                Some("remove-scope") => {
                    if let Some(scope) = v.get("scope").and_then(Value::as_str) {
                        let scopes = Self::remove_scope(scope);
                        Self::post_json(&json!({
                            "op": "scopes",
                            "scopes": scopes,
                        }));
                    }
                    Ok(())
                }
                Some("apply") => {
                    if let Some(form_val) = v.get("form") {
                        if let Ok(form) = serde_json::from_value::<CommitForm>(form_val.clone()) {
                            return self.apply_form(form);
                        }
                    }
                    Ok(())
                }
                _ => Ok(()),
            }
        } else {
            Ok(())
        }
    }
}

register_plugin!(GitCommitTemplate, views, commit_checks);

#[cfg(test)]
mod tests {
    use super::*;
    use openidea_plugin::mock::MockHost;

    #[test]
    fn open_command_reveals_dialog_and_posts_localized_init() {
        let host = MockHost::install();
        host.set_locale("zh-CN");
        host.set_commit_draft(Some(CommitDraft {
            message: "fix(api)!: 修复空指针问题\n\nFixes #42".into(),
            amend: false,
            revision: 7,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open", &host.invocation(), None)
            .expect("open command should succeed");

        assert_eq!(host.revealed_views(), vec![DIALOG_ID.to_string()]);

        let messages = host.view_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, DIALOG_ID);
        assert!(messages[0].1.contains("\"locale\":\"zh-CN\""));
        assert!(messages[0].1.contains("\"commitType\":\"fix\""));
        assert!(messages[0].1.contains("\"scope\":\"api\""));
        assert!(messages[0].1.contains("\"breakingBang\":true"));
        assert!(messages[0].1.contains("\"issueKeyword\":\"Fixes\""));
    }

    #[test]
    fn apply_writes_commit_message_and_notifies_dialog_to_close() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 3,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        let payload = json!({
            "op": "apply",
            "form": {
                "commitType": "feat",
                "scope": "workflow",
                "subject": "支持多节点工作流逐节点与多规则模型替换",
                "body": "补充详细说明内容",
                "wrap72": true,
                "breakingBang": false,
                "breaking": "",
                "issues": "#12",
                "issueKeyword": "Closes",
                "skipCi": true
            }
        });

        plugin
            .handle(
                DIALOG_ID,
                ViewEvent::Message(payload.to_string()),
                &host.invocation(),
            )
            .expect("apply should succeed");

        let updated = host.commit_draft().unwrap();
        assert_eq!(
            updated.message,
            "feat(workflow): 支持多节点工作流逐节点与多规则模型替换\n\n\
             补充详细说明内容\n\n\
             Closes #12\n\n\
             [skip ci]"
        );
        let messages = host.view_messages();
        assert!(messages.iter().any(|(v, m)| v == DIALOG_ID && m.contains("\"op\":\"applied\"")));
        assert_eq!(GitCommitTemplate::load_scopes(), vec!["workflow".to_string()]);
    }

    #[test]
    fn apply_rejects_empty_subject_without_applied_signal() {
        let host = MockHost::install();
        host.set_locale("en");
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 1,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        let empty_payload = json!({
            "op": "apply",
            "form": {
                "commitType": "feat",
                "scope": "",
                "subject": "   ",
                "body": "",
                "wrap72": true,
                "breakingBang": false,
                "breaking": "",
                "issues": "",
                "issueKeyword": "Closes",
                "skipCi": false
            }
        });

        plugin
            .handle(
                DIALOG_ID,
                ViewEvent::Message(empty_payload.to_string()),
                &host.invocation(),
            )
            .expect("empty subject should return Ok with inline error");

        assert_eq!(host.commit_draft().unwrap().message, "");
        let messages = host.view_messages();
        assert!(messages.iter().any(|(v, m)| v == DIALOG_ID && m.contains("\"op\":\"error\"")));
        assert!(!messages.iter().any(|(_, m)| m.contains("\"op\":\"applied\"")));
    }

    #[test]
    fn commit_checker_localizes_messages_for_en_and_zh() {
        let host = MockHost::install();
        let mut plugin = GitCommitTemplate::default();

        let bad_draft = CommitDraft {
            message: "not conventional".into(),
            amend: false,
            revision: 1,
            changes: vec![],
        };

        host.set_locale("en");
        let en_problems = plugin
            .check(CHECK_ID, &bad_draft, &host.invocation())
            .expect("en check should succeed");
        assert_eq!(en_problems.len(), 1);
        assert!(en_problems[0].message.contains("Conventional Commits"));

        host.set_locale("zh-CN");
        let zh_problems = plugin
            .check(CHECK_ID, &bad_draft, &host.invocation())
            .expect("zh check should succeed");
        assert_eq!(zh_problems.len(), 1);
        assert!(zh_problems[0].message.contains("提交标题不符合"));
    }
}
