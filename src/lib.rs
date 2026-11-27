mod commit_message;

use commit_message::{CommitForm, format_commit_message, parse_commit_message};
use openidea_plugin::views::{Body, Button, View, ViewEvent, Viewport, Views};
use openidea_plugin::{Error, Invocation, NotificationLevel, Plugin, host, register_plugin};
use serde_json::{Value, json};

pub const VIEW_ID: &str = "editor";
const STORAGE_SCOPES: &str = "recent_scopes";
const STORAGE_LAST_TYPE: &str = "last_type";
const STORAGE_WRAP_72: &str = "wrap_72";
const MAX_SCOPES: usize = 20;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Default)]
pub struct GitCommitTemplate {
    latest_form: Option<CommitForm>,
}

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
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "feat".to_string());
        let wrap_72 = host::storage::get(STORAGE_WRAP_72)
            .ok()
            .flatten()
            .map(|s| s != "false")
            .unwrap_or(true);
        (last_type, wrap_72)
    }

    fn save_defaults(commit_type: &str, wrap_72: bool) {
        if !commit_type.trim().is_empty() {
            let _ = host::storage::set(STORAGE_LAST_TYPE, commit_type.trim());
        }
        let _ = host::storage::set(STORAGE_WRAP_72, if wrap_72 { "true" } else { "false" });
    }

    fn load_form_from_pending_commit(&mut self) -> CommitForm {
        let (default_type, default_wrap_72) = Self::load_defaults();
        let form = match host::pending_commit() {
            Ok(Some(draft)) => parse_commit_message(&draft.message, &default_type, default_wrap_72),
            _ => CommitForm {
                commit_type: default_type,
                wrap_72: default_wrap_72,
                ..CommitForm::default()
            },
        };
        self.latest_form = Some(form.clone());
        form
    }

    fn post_json(message: &Value) {
        let _ = host::post_view_message(VIEW_ID, &message.to_string());
    }

    fn send_init(&mut self) {
        let form = self.load_form_from_pending_commit();
        let scopes = Self::load_scopes();
        Self::post_json(&json!({
            "op": "init",
            "form": form,
            "scopes": scopes,
        }));
    }

    fn apply_form(&mut self, form: CommitForm) -> Result<(), Error> {
        let formatted = format_commit_message(&form);
        Self::save_defaults(&form.commit_type, form.wrap_72);
        let scopes = Self::remember_scope(&form.scope);
        self.latest_form = Some(form);

        let draft = match host::pending_commit()? {
            Some(draft) => draft,
            None => {
                Self::post_json(&json!({
                    "op": "error",
                    "message": "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
                }));
                host::notify(
                    NotificationLevel::Warning,
                    "Git Commit Template",
                    "当前窗口未检测到 Git 仓库。",
                );
                return Ok(());
            }
        };

        match host::set_commit_message(draft.revision, &formatted) {
            Ok(()) => {
                Self::post_json(&json!({
                    "op": "applied",
                    "message": formatted,
                    "scopes": scopes,
                }));
                host::notify(
                    NotificationLevel::Info,
                    "Git Commit Template",
                    "提交信息已填充到 Commit 输入框。",
                );
                Ok(())
            }
            Err(Error::Stale) => {
                // Retry once with the fresh revision if the commit draft refreshed right before apply.
                if let Some(fresh) = host::pending_commit()? {
                    host::set_commit_message(fresh.revision, &formatted)?;
                    Self::post_json(&json!({
                        "op": "applied",
                        "message": formatted,
                        "scopes": scopes,
                    }));
                    host::notify(
                        NotificationLevel::Info,
                        "Git Commit Template",
                        "提交信息已填充到 Commit 输入框。",
                    );
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
                let form = self.load_form_from_pending_commit();
                let scopes = Self::load_scopes();
                host::reveal_view(VIEW_ID)?;
                Self::post_json(&json!({
                    "op": "init",
                    "form": form,
                    "scopes": scopes,
                }));
                Ok(None)
            }
            _ => Err(Error::NotFound(format!("command {id}"))),
        }
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
        let form = match &self.latest_form {
            Some(f) => f.clone(),
            None => self.load_form_from_pending_commit(),
        };
        let scopes = Self::load_scopes();
        let start = json!({
            "form": form,
            "scopes": scopes,
        })
        .to_string()
        .replace('<', "\\u003c");
        let page = PAGE_HTML.replacen(r#""__START__""#, &start, 1);

        Ok(View::new(Body::Html(page))
            .button(Button::new("reload", "读取当前提交消息").icon("refresh")))
    }

    fn handle(&mut self, view: &str, event: ViewEvent, _ctx: &Invocation) -> Result<(), Error> {
        if view != VIEW_ID {
            return Err(Error::NotFound(view.into()));
        }
        match event {
            ViewEvent::Button(id) if id == "reload" => {
                self.send_init();
                Ok(())
            }
            ViewEvent::Message(raw) => {
                let Ok(v) = serde_json::from_str::<Value>(&raw) else {
                    return Ok(());
                };
                match v.get("op").and_then(Value::as_str) {
                    Some("ready") | Some("reload") => {
                        self.send_init();
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
                            if let Ok(form) = serde_json::from_value::<CommitForm>(form_val.clone())
                            {
                                return self.apply_form(form);
                            }
                        }
                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }
}

register_plugin!(GitCommitTemplate, views);

#[cfg(test)]
mod tests {
    use super::*;
    use openidea_plugin::CommitDraft;
    use openidea_plugin::mock::MockHost;

    #[test]
    fn open_command_reveals_view_and_loads_existing_commit() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: "fix(api): 修复空指针问题".into(),
            amend: false,
            revision: 7,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open", &host.invocation(), None)
            .expect("open command should succeed");

        let rendered = plugin
            .render(VIEW_ID, None, &host.invocation())
            .expect("render should succeed");
        match rendered.body {
            Body::Html(html) => {
                assert!(html.contains("修复空指针问题"));
                assert!(html.contains("\"commitType\":\"fix\""));
                assert!(html.contains("\"scope\":\"api\""));
            }
            _ => panic!("expected Body::Html"),
        }
    }

    #[test]
    fn apply_event_writes_commit_message_and_remembers_scope() {
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
                "breaking": "",
                "issues": "#12",
                "skipCi": true
            }
        });

        plugin
            .handle(VIEW_ID, ViewEvent::Message(payload.to_string()), &host.invocation())
            .expect("apply should succeed");

        let updated = host.commit_draft().unwrap();
        assert_eq!(
            updated.message,
            "feat(workflow): 支持多节点工作流逐节点与多规则模型替换\n\n\
             补充详细说明内容\n\n\
             Closes #12\n\n\
             [skip ci]"
        );

        let scopes = GitCommitTemplate::load_scopes();
        assert_eq!(scopes, vec!["workflow".to_string()]);
    }
}
