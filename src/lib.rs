mod commit_message;

use commit_message::{
    CommitForm, HEADER_WARN_LENGTH, format_commit_message, is_known_commit_type,
    parse_commit_message, validate_conventional_commit, validate_form,
};
use openidea_plugin::commit::{CommitChecks, CommitProblem};
use openidea_plugin::views::{Body, View, ViewEvent, Viewport, Views};
use openidea_plugin::{
    CommitDraft, Error, Invocation, NotificationLevel, Plugin, host, register_plugin,
};
use serde_json::{Value, json};

pub const VIEW_ID: &str = "editor";
pub const CHECK_ID: &str = "conventional";
const FORM_FILE_NAME: &str = "编辑提交消息.git-commit-template";

const STORAGE_SCOPES: &str = "recent_scopes";
const STORAGE_LAST_TYPE: &str = "last_type";
const STORAGE_WRAP_72: &str = "wrap_72";
const MAX_SCOPES: usize = 20;

#[derive(Default)]
pub struct GitCommitTemplate;

impl GitCommitTemplate {
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
        let _ = host::post_view_message(VIEW_ID, &message.to_string());
    }

    fn send_init_from_pending_commit() {
        let form = Self::load_form_from_pending_commit();
        let scopes = Self::load_scopes();
        let preview = Self::build_preview_json(&form);
        Self::post_json(&json!({
            "op": "init",
            "form": form,
            "scopes": scopes,
            "preview": preview,
        }));
    }

    fn open_centered_form() -> Result<(), Error> {
        if host::pending_commit()?.is_none() {
            host::notify(
                NotificationLevel::Warning,
                "Git Commit Template",
                "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
            );
            return Ok(());
        }

        let dir = host::fs::plugin_dir()?;
        let path = format!("{}/{FORM_FILE_NAME}", dir.trim_end_matches(['/', '\\']));
        host::fs::write_bytes(&path, b"GCT")?;
        host::open_location(&path, 0, 0)?;
        Self::send_init_from_pending_commit();
        Ok(())
    }

    fn close_form_tab() {
        // Ensure focus is in the editor area before closing the active custom editor tab.
        let _ = host::execute_command("toolwindow.focusEditor", None);
        let _ = host::execute_command("editor.closeTab", None);
    }

    fn apply_form(&mut self, form: CommitForm) -> Result<(), Error> {
        if let Err(validation_msg) = validate_form(&form) {
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
                "message": "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
            }));
            return Ok(());
        };

        match host::set_commit_message(draft.revision, &formatted) {
            Ok(()) => {
                Self::close_form_tab();
                Ok(())
            }
            Err(Error::Stale) => {
                if let Some(fresh) = host::pending_commit()? {
                    host::set_commit_message(fresh.revision, &formatted)?;
                    Self::close_form_tab();
                    Ok(())
                } else {
                    Self::post_json(&json!({
                        "op": "error",
                        "message": "Commit 状态已变化，请重试。",
                    }));
                    Ok(())
                }
            }
            Err(err) => {
                Self::post_json(&json!({
                    "op": "error",
                    "message": format!("写入提交信息失败：{err}"),
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
                Self::open_centered_form()?;
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
        let problems = validate_conventional_commit(&draft.message)
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
        if view != VIEW_ID {
            return Err(Error::NotFound(view.into()));
        }
        Ok(View::new(Body::Message(
            "请点击 Commit 工具栏图标打开提交消息表单。".into(),
        )))
    }

    fn handle(&mut self, view: &str, event: ViewEvent, _ctx: &Invocation) -> Result<(), Error> {
        if view != VIEW_ID {
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
                Some("cancel") => {
                    Self::close_form_tab();
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
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn open_command_opens_centered_form_and_posts_init() {
        let host = MockHost::install();
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

        let opened = host.opened_locations();
        assert_eq!(opened.len(), 1);
        assert!(opened[0].0.ends_with(".git-commit-template"));

        let messages = host.view_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].0, VIEW_ID);
        assert!(messages[0].1.contains("\"commitType\":\"fix\""));
        assert!(messages[0].1.contains("\"scope\":\"api\""));
        assert!(messages[0].1.contains("\"breakingBang\":true"));
        assert!(messages[0].1.contains("\"issueKeyword\":\"Fixes\""));
    }

    #[test]
    fn apply_writes_commit_message_and_closes_form_tab() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 3,
            changes: vec![],
        }));

        let closed = Rc::new(Cell::new(false));
        let closed_ref = closed.clone();
        host.on_command("editor.closeTab", move |_| {
            closed_ref.set(true);
            Ok(None)
        });

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
                VIEW_ID,
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
        assert!(closed.get(), "form tab should auto-close after apply");
        assert_eq!(GitCommitTemplate::load_scopes(), vec!["workflow".to_string()]);
    }

    #[test]
    fn cancel_closes_form_tab_without_modifying_commit() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: "chore: 原始草稿".into(),
            amend: false,
            revision: 5,
            changes: vec![],
        }));

        let closed = Rc::new(Cell::new(false));
        let closed_ref = closed.clone();
        host.on_command("editor.closeTab", move |_| {
            closed_ref.set(true);
            Ok(None)
        });

        let mut plugin = GitCommitTemplate::default();
        plugin
            .handle(
                VIEW_ID,
                ViewEvent::Message(json!({ "op": "cancel" }).to_string()),
                &host.invocation(),
            )
            .expect("cancel should succeed");

        assert!(closed.get(), "form tab should auto-close on cancel");
        let draft = host.commit_draft().unwrap();
        assert_eq!(draft.message, "chore: 原始草稿");
        assert_eq!(draft.revision, 5);
    }

    #[test]
    fn apply_rejects_empty_subject_without_closing_tab() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 1,
            changes: vec![],
        }));

        let closed = Rc::new(Cell::new(false));
        let closed_ref = closed.clone();
        host.on_command("editor.closeTab", move |_| {
            closed_ref.set(true);
            Ok(None)
        });

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
                VIEW_ID,
                ViewEvent::Message(empty_payload.to_string()),
                &host.invocation(),
            )
            .expect("empty subject should return Ok with inline error");

        assert!(!closed.get(), "tab must stay open when validation fails");
        assert_eq!(host.commit_draft().unwrap().message, "");
    }

    #[test]
    fn commit_checker_flags_invalid_and_allows_valid_commits() {
        let host = MockHost::install();
        let mut plugin = GitCommitTemplate::default();

        let bad_draft = CommitDraft {
            message: "随便写点东西".into(),
            amend: false,
            revision: 1,
            changes: vec![],
        };
        let problems = plugin
            .check(CHECK_ID, &bad_draft, &host.invocation())
            .expect("check should succeed");
        assert!(!problems.is_empty());

        let ok_draft = CommitDraft {
            message: "feat(core): add centered form".into(),
            amend: false,
            revision: 2,
            changes: vec![],
        };
        let problems = plugin
            .check(CHECK_ID, &ok_draft, &host.invocation())
            .expect("check should succeed");
        assert!(problems.is_empty());
    }
}
