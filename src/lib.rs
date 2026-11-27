mod commit_message;

use commit_message::{
    COMMIT_TYPES, CommitForm, HEADER_WARN_LENGTH, format_commit_message, is_known_commit_type,
    parse_commit_message, validate_conventional_commit, validate_form,
};
use openidea_plugin::commit::{CommitChecks, CommitProblem};
use openidea_plugin::views::{Body, View, ViewEvent, Viewport, Views};
use openidea_plugin::{
    CommitDraft, Error, Invocation, NotificationLevel, PickItem, Plugin, host, register_plugin,
};
use serde_json::{Value, json};

pub const VIEW_ID: &str = "editor";
pub const CHECK_ID: &str = "conventional";

const STORAGE_SCOPES: &str = "recent_scopes";
const STORAGE_LAST_TYPE: &str = "last_type";
const STORAGE_WRAP_72: &str = "wrap_72";
const MAX_SCOPES: usize = 20;

const PAGE_HTML: &str = include_str!("page.html");

#[derive(Default)]
pub struct GitCommitTemplate {
    /// Holds unsaved form state only when handing off from the modal wizard to the side panel.
    staged_form: Option<CommitForm>,
}

enum WizardOutcome {
    Applied,
    Cancelled,
    OpenPanel(CommitForm),
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

    fn send_init_with_form(form: &CommitForm) {
        let scopes = Self::load_scopes();
        let preview = Self::build_preview_json(form);
        Self::post_json(&json!({
            "op": "init",
            "form": form,
            "scopes": scopes,
            "preview": preview,
        }));
    }

    fn send_init_from_pending_commit() {
        let form = Self::load_form_from_pending_commit();
        Self::send_init_with_form(&form);
    }

    fn open_side_panel(&mut self, form: CommitForm) -> Result<(), Error> {
        self.staged_form = Some(form.clone());
        host::reveal_view(VIEW_ID)?;
        Self::send_init_with_form(&form);
        Ok(())
    }

    fn write_to_commit_draft(formatted: &str) -> Result<bool, Error> {
        let Some(draft) = host::pending_commit()? else {
            host::notify(
                NotificationLevel::Warning,
                "Git Commit Template",
                "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
            );
            return Ok(false);
        };

        match host::set_commit_message(draft.revision, formatted) {
            Ok(()) => Ok(true),
            Err(Error::Stale) => {
                if let Some(fresh) = host::pending_commit()? {
                    host::set_commit_message(fresh.revision, formatted)?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            Err(err) => Err(err),
        }
    }

    fn apply_form_from_panel(&mut self, form: CommitForm) -> Result<(), Error> {
        if let Err(validation_msg) = validate_form(&form) {
            Self::post_json(&json!({
                "op": "error",
                "message": validation_msg,
            }));
            return Ok(());
        }

        let formatted = format_commit_message(&form);
        Self::save_defaults(&form.commit_type, form.wrap_72);
        let scopes = Self::remember_scope(&form.scope);
        self.staged_form = None;

        match Self::write_to_commit_draft(&formatted) {
            Ok(true) => {
                let preview = Self::build_preview_json(&form);
                Self::post_json(&json!({
                    "op": "applied",
                    "message": formatted,
                    "scopes": scopes,
                    "preview": preview,
                }));
                Ok(())
            }
            Ok(false) => {
                Self::post_json(&json!({
                    "op": "error",
                    "message": "当前窗口未检测到可用的 Git 提交上下文，请先打开 Git 仓库。",
                }));
                Ok(())
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

    /// Runs the native modal quick-fill wizard (`pick` / `input` / `input_multiline`).
    /// Every dialog closes automatically when confirmed or dismissed with `Esc`.
    fn run_quick_wizard(&mut self) -> Result<WizardOutcome, Error> {
        if host::pending_commit()?.is_none() {
            host::notify(
                NotificationLevel::Warning,
                "Git Commit Template",
                "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
            );
            return Ok(WizardOutcome::Cancelled);
        }

        let mut form = Self::load_form_from_pending_commit();
        let scopes = Self::load_scopes();

        // Step 1: Commit type (current/recent type placed first for instant Enter).
        let mut type_items: Vec<PickItem> = Vec::with_capacity(COMMIT_TYPES.len() + 1);
        for &(id, title, desc) in COMMIT_TYPES {
            if id == form.commit_type {
                type_items.insert(
                    0,
                    PickItem::new(id, &format!("{id} — {title}"))
                        .description(&format!("当前默认 · {desc}")),
                );
            } else {
                type_items.push(PickItem::new(id, &format!("{id} — {title}")).description(desc));
            }
        }
        type_items.push(
            PickItem::new("__panel__", "打开侧边高级编辑面板…")
                .description("适合多段落反复编辑与实时对照预览"),
        );

        let Some(chosen_type) = host::pick("选择提交类型 (1/3)", &type_items)? else {
            return Ok(WizardOutcome::Cancelled);
        };
        if chosen_type == "__panel__" {
            return Ok(WizardOutcome::OpenPanel(form));
        }
        form.commit_type = chosen_type;

        // Step 2: Scope (skip with Enter when no scope, or pick recent / type custom).
        let mut scope_items: Vec<PickItem> = Vec::new();
        let existing_scope = form.scope.trim().to_string();
        if !existing_scope.is_empty() {
            scope_items.push(
                PickItem::new("__keep__", &format!("保持当前范围：{existing_scope}"))
                    .description("直接回车沿用当前变更范围"),
            );
        }
        scope_items.push(
            PickItem::new("__none__", "不填写变更范围（无 Scope）")
                .description(&format!("生成如 {}: 简短说明", form.commit_type)),
        );
        scope_items.push(
            PickItem::new("__custom__", "输入新的变更范围…")
                .description("手动输入 Scope 并自动加入常用列表"),
        );
        for sc in &scopes {
            if sc != &existing_scope {
                scope_items.push(
                    PickItem::new(&format!("scope:{sc}"), sc).description("最近使用的变更范围"),
                );
            }
        }

        let Some(chosen_scope) = host::pick("选择变更范围 Scope (2/3)", &scope_items)? else {
            return Ok(WizardOutcome::Cancelled);
        };
        match chosen_scope.as_str() {
            "__keep__" => {}
            "__none__" => form.scope.clear(),
            "__custom__" => {
                let initial = if existing_scope.is_empty() {
                    None
                } else {
                    Some(existing_scope.as_str())
                };
                let Some(typed) =
                    host::input("输入变更范围 Scope（如 auth、api、workflow）", initial)?
                else {
                    return Ok(WizardOutcome::Cancelled);
                };
                form.scope = typed.trim().to_string();
            }
            other => {
                if let Some(sc) = other.strip_prefix("scope:") {
                    form.scope = sc.to_string();
                }
            }
        }

        // Step 3: Subject (required; re-prompts if left blank, Esc cancels cleanly).
        loop {
            let bang = if form.breaking_bang { "!" } else { "" };
            let prefix = if form.scope.is_empty() {
                format!("{}{bang}: ", form.commit_type)
            } else {
                format!("{}({}){bang}: ", form.commit_type, form.scope)
            };
            let initial = if form.subject.trim().is_empty() {
                None
            } else {
                Some(form.subject.as_str())
            };
            let prompt = format!("输入简短说明 (3/3) — 前缀 `{prefix}`");
            let Some(typed_subject) = host::input(&prompt, initial)? else {
                return Ok(WizardOutcome::Cancelled);
            };
            if !typed_subject.trim().is_empty() {
                form.subject = typed_subject.trim().to_string();
                break;
            }
            host::notify(
                NotificationLevel::Warning,
                "Git Commit Template",
                "简短说明不能为空，请输入内容或按 Esc 取消。",
            );
        }

        // Step 4: Action hub — first item applies immediately on Enter; optional items edit details.
        loop {
            let formatted = format_commit_message(&form);
            let first_line = formatted.lines().next().unwrap_or_default();
            let header_len = first_line.chars().count();
            let apply_desc = if header_len > HEADER_WARN_LENGTH {
                format!("预览：{first_line}（标题 {header_len} 字符，建议精简至 72 字符内）")
            } else {
                format!("预览：{first_line}（直接回车写入提交框并关闭）")
            };

            let body_desc = if form.body.trim().is_empty() {
                "可选：补充改动背景、原因或实现要点（Ctrl+Enter 确认）".to_string()
            } else {
                format!("已填写：{}", summarize_line(&form.body, 40))
            };

            let breaking_desc = if form.breaking.trim().is_empty() {
                "可选：自动添加 BREAKING CHANGE: 前缀".to_string()
            } else {
                format!("已填写：{}", summarize_line(&form.breaking, 40))
            };

            let bang_label = if form.breaking_bang {
                "取消标题 ! 破坏性标记（当前已开启）"
            } else {
                "在标题添加 ! 破坏性标记（如 feat!:）"
            };

            let issues_desc = if form.issues.trim().is_empty() {
                "可选：如 #123, #456（自动添加 Closes 前缀）".to_string()
            } else {
                format!("已填写：{} ({})", form.issues.trim(), form.issue_keyword)
            };

            let skip_ci_label = if form.skip_ci {
                "取消跳过 CI（当前已附带 [skip ci]）"
            } else {
                "跳过 CI 流程（自动附带 [skip ci]）"
            };

            let hub_items = vec![
                PickItem::new("apply", "完成并填入提交框").description(&apply_desc),
                PickItem::new("body", "编辑详细说明 (Body)").description(&body_desc),
                PickItem::new("issues", "编辑关联 Issues").description(&issues_desc),
                PickItem::new("breaking", "编辑重大变化 (BREAKING CHANGE)")
                    .description(&breaking_desc),
                PickItem::new("bang", bang_label)
                    .description("在类型/范围后附带 `!` 标识不兼容变更"),
                PickItem::new("skip_ci", skip_ci_label)
                    .description("在提交末尾附带或移除 [skip ci]"),
                PickItem::new("panel", "转到侧边高级面板继续编辑…")
                    .description("保留当前已填内容并在右侧完整表单中打开"),
            ];

            let Some(action) = host::pick("确认填入或补充可选字段", &hub_items)? else {
                return Ok(WizardOutcome::Cancelled);
            };

            match action.as_str() {
                "apply" => {
                    Self::save_defaults(&form.commit_type, form.wrap_72);
                    let updated_scopes = Self::remember_scope(&form.scope);
                    self.staged_form = None;
                    if Self::write_to_commit_draft(&formatted)? {
                        let preview = Self::build_preview_json(&form);
                        Self::post_json(&json!({
                            "op": "applied",
                            "message": formatted,
                            "scopes": updated_scopes,
                            "form": form,
                            "preview": preview,
                        }));
                    }
                    return Ok(WizardOutcome::Applied);
                }
                "body" => {
                    let initial = if form.body.is_empty() {
                        None
                    } else {
                        Some(form.body.as_str())
                    };
                    if let Some(text) = host::input_multiline(
                        "输入详细说明 Body（留空可清除，Ctrl+Enter 确认）",
                        initial,
                    )? {
                        form.body = text.trim().to_string();
                    }
                }
                "issues" => {
                    let initial = if form.issues.is_empty() {
                        None
                    } else {
                        Some(form.issues.as_str())
                    };
                    if let Some(text) = host::input(
                        "输入要关闭的 Issues（如 #123, #456 或 Fixes #123）",
                        initial,
                    )? {
                        form.issues = text.trim().to_string();
                    }
                }
                "breaking" => {
                    let initial = if form.breaking.is_empty() {
                        None
                    } else {
                        Some(form.breaking.as_str())
                    };
                    if let Some(text) = host::input_multiline(
                        "输入重大变化说明（自动添加 BREAKING CHANGE: 前缀，Ctrl+Enter 确认）",
                        initial,
                    )? {
                        form.breaking = text.trim().to_string();
                    }
                }
                "bang" => {
                    form.breaking_bang = !form.breaking_bang;
                }
                "skip_ci" => {
                    form.skip_ci = !form.skip_ci;
                }
                "panel" => {
                    return Ok(WizardOutcome::OpenPanel(form));
                }
                _ => {}
            }
        }
    }
}

fn summarize_line(text: &str, max_chars: usize) -> String {
    let collapsed = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.chars().count() <= max_chars {
        collapsed
    } else {
        let head: String = collapsed.chars().take(max_chars).collect();
        format!("{head}…")
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
                if let WizardOutcome::OpenPanel(form) = self.run_quick_wizard()? {
                    self.open_side_panel(form)?;
                }
                Ok(None)
            }
            "open_panel" => {
                let form = Self::load_form_from_pending_commit();
                self.open_side_panel(form)?;
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
        let form = self
            .staged_form
            .take()
            .unwrap_or_else(Self::load_form_from_pending_commit);
        let scopes = Self::load_scopes();
        let preview = Self::build_preview_json(&form);
        let start = json!({
            "form": form,
            "scopes": scopes,
            "preview": preview,
        })
        .to_string()
        .replace('<', "\\u003c");
        let page = PAGE_HTML.replacen(r#""__START__""#, &start, 1);

        Ok(View::new(Body::Html(page)))
    }

    fn handle(&mut self, view: &str, event: ViewEvent, _ctx: &Invocation) -> Result<(), Error> {
        if view != VIEW_ID {
            return Err(Error::NotFound(view.into()));
        }
        match event {
            ViewEvent::Button(id) if id == "reload" => {
                Self::send_init_from_pending_commit();
                Ok(())
            }
            ViewEvent::Message(raw) => {
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
                            if let Ok(form) = serde_json::from_value::<CommitForm>(form_val.clone())
                            {
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
                            if let Ok(form) = serde_json::from_value::<CommitForm>(form_val.clone())
                            {
                                return self.apply_form_from_panel(form);
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

register_plugin!(GitCommitTemplate, views, commit_checks);

#[cfg(test)]
mod tests {
    use super::*;
    use openidea_plugin::mock::MockHost;

    #[test]
    fn quick_wizard_fast_path_populates_commit_message() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 1,
            changes: vec![],
        }));

        host.answer_next_prompt(Some(&["fix"]));
        host.answer_next_prompt(Some(&["__none__"]));
        host.answer_next_prompt(Some(&["修复登录接口空指针"]));
        host.answer_next_prompt(Some(&["apply"]));

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open", &host.invocation(), None)
            .expect("quick wizard should succeed");

        let updated = host.commit_draft().unwrap();
        assert_eq!(updated.message, "fix: 修复登录接口空指针");
        assert!(host.revealed_views().is_empty());
    }

    #[test]
    fn quick_wizard_supports_custom_scope_and_optional_fields() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 2,
            changes: vec![],
        }));

        host.answer_next_prompt(Some(&["feat"]));
        host.answer_next_prompt(Some(&["__custom__"]));
        host.answer_next_prompt(Some(&["auth"]));
        host.answer_next_prompt(Some(&["支持 OAuth2 PKCE 授权"]));
        host.answer_next_prompt(Some(&["body"]));
        host.answer_next_prompt(Some(&["补充 PKCE 校验流程"]));
        host.answer_next_prompt(Some(&["issues"]));
        host.answer_next_prompt(Some(&["108"]));
        host.answer_next_prompt(Some(&["bang"]));
        host.answer_next_prompt(Some(&["skip_ci"]));
        host.answer_next_prompt(Some(&["apply"]));

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open", &host.invocation(), None)
            .expect("wizard with optional fields should succeed");

        let updated = host.commit_draft().unwrap();
        assert_eq!(
            updated.message,
            "feat(auth)!: 支持 OAuth2 PKCE 授权\n\n\
             补充 PKCE 校验流程\n\n\
             Closes #108\n\n\
             [skip ci]"
        );
        assert_eq!(GitCommitTemplate::load_scopes(), vec!["auth".to_string()]);
    }

    #[test]
    fn quick_wizard_cancel_leaves_commit_untouched() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: "chore: 原始草稿".into(),
            amend: false,
            revision: 5,
            changes: vec![],
        }));

        host.answer_next_prompt(Some(&["feat"]));
        host.answer_next_prompt(None); // User hits Esc at scope step.

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open", &host.invocation(), None)
            .expect("cancelled wizard should return Ok");

        let draft = host.commit_draft().unwrap();
        assert_eq!(draft.message, "chore: 原始草稿");
        assert_eq!(draft.revision, 5);
    }

    #[test]
    fn open_panel_command_reveals_view_and_loads_existing_commit() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: "fix(api)!: 修复空指针问题\n\nFixes #42".into(),
            amend: false,
            revision: 7,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        plugin
            .run_command("open_panel", &host.invocation(), None)
            .expect("open_panel command should succeed");

        assert_eq!(host.revealed_views(), vec![VIEW_ID.to_string()]);

        let rendered = plugin
            .render(VIEW_ID, None, &host.invocation())
            .expect("render should succeed");
        match rendered.body {
            Body::Html(html) => {
                assert!(html.contains("修复空指针问题"));
                assert!(html.contains("\"commitType\":\"fix\""));
                assert!(html.contains("\"scope\":\"api\""));
                assert!(html.contains("\"breakingBang\":true"));
                assert!(html.contains("\"issueKeyword\":\"Fixes\""));
            }
            _ => panic!("expected Body::Html"),
        }
    }

    #[test]
    fn panel_apply_rejects_empty_subject_and_writes_valid_form() {
        let host = MockHost::install();
        host.set_commit_draft(Some(CommitDraft {
            message: String::new(),
            amend: false,
            revision: 3,
            changes: vec![],
        }));

        let mut plugin = GitCommitTemplate::default();
        let empty_payload = json!({
            "op": "apply",
            "form": {
                "commitType": "feat",
                "scope": "workflow",
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
            .expect("empty subject apply should return Ok with error message");
        assert_eq!(host.commit_draft().unwrap().message, "");

        let valid_payload = json!({
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
                ViewEvent::Message(valid_payload.to_string()),
                &host.invocation(),
            )
            .expect("valid apply should succeed");

        let updated = host.commit_draft().unwrap();
        assert_eq!(
            updated.message,
            "feat(workflow): 支持多节点工作流逐节点与多规则模型替换\n\n\
             补充详细说明内容\n\n\
             Closes #12\n\n\
             [skip ci]"
        );
        assert_eq!(GitCommitTemplate::load_scopes(), vec!["workflow".to_string()]);
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
            message: "feat(core): add template wizard".into(),
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
