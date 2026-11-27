/// Supported UI locales resolved from OpenIDEA's BCP 47 `host::locale()` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiLocale {
    En,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
    Fr,
    Es,
    Pt,
    Pl,
    Tr,
    Ru,
}

impl UiLocale {
    pub fn from_tag(tag: &str) -> Self {
        let lower = tag.trim().to_ascii_lowercase();
        if lower.starts_with("zh-tw") || lower.starts_with("zh-hk") || lower.starts_with("zh-mo") || lower.contains("hant") {
            Self::ZhTw
        } else if lower.starts_with("zh") {
            Self::ZhCn
        } else if lower.starts_with("ja") {
            Self::Ja
        } else if lower.starts_with("ko") {
            Self::Ko
        } else if lower.starts_with("fr") {
            Self::Fr
        } else if lower.starts_with("es") {
            Self::Es
        } else if lower.starts_with("pt") {
            Self::Pt
        } else if lower.starts_with("pl") {
            Self::Pl
        } else if lower.starts_with("tr") {
            Self::Tr
        } else if lower.starts_with("ru") {
            Self::Ru
        } else {
            Self::En
        }
    }

    pub fn empty_subject_error(self) -> &'static str {
        match self {
            Self::ZhCn => "请填写简短说明（提交标题不能为空）。",
            Self::ZhTw => "請填寫簡短說明（提交標題不能為空）。",
            Self::Ja => "件名（短い説明）を入力してください。",
            Self::Ko => "짧은 설명(커밋 제목)을 입력해 주세요.",
            Self::Fr => "Veuillez saisir une description courte (le sujet ne peut pas être vide).",
            Self::Es => "Introduce una descripción breve (el asunto no puede estar vacío).",
            Self::Pt => "Informe uma descrição curta (o título não pode ficar vazio).",
            Self::Pl => "Wprowadź krótki opis (temat commita nie może być pusty).",
            Self::Tr => "Lütfen kısa bir açıklama girin (commit başlığı boş olamaz).",
            Self::Ru => "Введите краткое описание (заголовок коммита не может быть пустым).",
            Self::En => "Please enter a short description (commit subject cannot be empty).",
        }
    }

    pub fn no_git_repo_warning(self) -> &'static str {
        match self {
            Self::ZhCn => "当前窗口未检测到 Git 仓库，请先打开包含 Git 仓库的项目。",
            Self::ZhTw => "目前視窗未偵測到 Git 儲存庫，請先開啟包含 Git 儲存庫的專案。",
            Self::Ja => "現在のウィンドウで Git リポジトリが見つかりません。Git プロジェクトを開いてください。",
            Self::Ko => "현재 창에서 Git 저장소를 찾을 수 없습니다. Git 프로젝트를 먼저 열어 주세요.",
            Self::Fr => "Aucun dépôt Git détecté dans cette fenêtre. Ouvrez d'abord un projet Git.",
            Self::Es => "No se detectó ningún repositorio Git en esta ventana. Abre primero un proyecto con Git.",
            Self::Pt => "Nenhum repositório Git detectado nesta janela. Abra primeiro um projeto com Git.",
            Self::Pl => "Nie wykryto repozytorium Git w tym oknie. Otwórz najpierw projekt z Git.",
            Self::Tr => "Bu pencerede Git deposu algılanmadı. Lütfen önce Git içeren bir proje açın.",
            Self::Ru => "В текущем окне не найден репозиторий Git. Сначала откройте проект с Git.",
            Self::En => "No Git repository detected in this window. Please open a project with a Git repository first.",
        }
    }

    pub fn stale_commit_error(self) -> &'static str {
        match self {
            Self::ZhCn => "Commit 状态已变化，请重试。",
            Self::ZhTw => "Commit 狀態已變更，請重試。",
            Self::Ja => "コミット状態が変更されました。もう一度お試しください。",
            Self::Ko => "커밋 상태가 변경되었습니다. 다시 시도해 주세요.",
            Self::Fr => "L'état du commit a changé, veuillez réessayer.",
            Self::Es => "El estado del commit ha cambiado; inténtalo de nuevo.",
            Self::Pt => "O estado do commit mudou; tente novamente.",
            Self::Pl => "Stan commita uległ zmianie, spróbuj ponownie.",
            Self::Tr => "Commit durumu değişti, lütfen tekrar deneyin.",
            Self::Ru => "Состояние коммита изменилось, повторите попытку.",
            Self::En => "Commit draft changed. Please try again.",
        }
    }

    pub fn write_failed_error(self, detail: &str) -> String {
        match self {
            Self::ZhCn => format!("写入提交信息失败：{detail}"),
            Self::ZhTw => format!("寫入提交訊息失敗：{detail}"),
            Self::Ja => format!("コミットメッセージの書き込みに失敗しました: {detail}"),
            Self::Ko => format!("커밋 메시지 작성 실패: {detail}"),
            Self::Fr => format!("Échec de l'écriture du message de commit : {detail}"),
            Self::Es => format!("Error al escribir el mensaje de commit: {detail}"),
            Self::Pt => format!("Falha ao gravar a mensagem de commit: {detail}"),
            Self::Pl => format!("Nie udało się zapisać wiadomości commita: {detail}"),
            Self::Tr => format!("Commit mesajı yazılamadı: {detail}"),
            Self::Ru => format!("Не удалось записать сообщение коммита: {detail}"),
            Self::En => format!("Failed to set commit message: {detail}"),
        }
    }

    pub fn view_fallback_message(self) -> &'static str {
        match self {
            Self::ZhCn => "请点击 Git 提交面板工具栏图标打开提交消息表单。",
            Self::ZhTw => "請點擊 Git 提交面板工具列圖示開啟提交訊息表單。",
            _ => "Please click the Git Commit toolbar icon to open the commit template form.",
        }
    }

    pub fn check_empty_message(self) -> &'static str {
        match self {
            Self::ZhCn => "提交信息不能为空。",
            Self::ZhTw => "提交訊息不能為空。",
            Self::Ja => "コミットメッセージが空です。",
            Self::Ko => "커밋 메시지는 비워 둘 수 없습니다.",
            Self::Fr => "Le message de commit ne peut pas être vide.",
            Self::Es => "El mensaje de commit no puede estar vacío.",
            Self::Pt => "A mensagem de commit não pode ficar vazia.",
            Self::Pl => "Wiadomość commita nie może być pusta.",
            Self::Tr => "Commit mesajı boş olamaz.",
            Self::Ru => "Сообщение коммита не может быть пустым.",
            Self::En => "Commit message cannot be empty.",
        }
    }

    pub fn check_missing_subject(self) -> &'static str {
        match self {
            Self::ZhCn => "提交标题缺少简短说明（冒号后不能为空）。",
            Self::ZhTw => "提交標題缺少簡短說明（冒號後不能為空）。",
            Self::Ja => "コミット件名が空です（コロンの後に説明を入力してください）。",
            Self::Ko => "커밋 제목 설명이 비어 있습니다(콜론 뒤에 내용을 입력하세요).",
            Self::Fr => "Le sujet du commit est manquant après les deux-points.",
            Self::Es => "Falta el asunto del commit después de los dos puntos.",
            Self::Pt => "O título do commit está ausente após os dois-pontos.",
            Self::Pl => "Brakuje opisu commita po dwukropku.",
            Self::Tr => "İki nokta üst üsteden sonra commit açıklaması eksik.",
            Self::Ru => "Отсутствует описание коммита после двоеточия.",
            Self::En => "Commit header is missing a subject after the colon.",
        }
    }

    pub fn check_unknown_type(self, allowed: &str) -> String {
        match self {
            Self::ZhCn => format!("未知的提交类型，请使用标准类型（{allowed}）。"),
            Self::ZhTw => format!("未知的提交類型，請使用標準類型（{allowed}）。"),
            Self::Ja => format!("不明なコミットタイプです。標準タイプ（{allowed}）を使用してください。"),
            Self::Ko => format!("알 수 없는 커밋 타입입니다. 표준 타입({allowed})을 사용하세요."),
            Self::Fr => format!("Type de commit inconnu. Utilisez un type standard ({allowed})."),
            Self::Es => format!("Tipo de commit desconocido. Usa un tipo estándar ({allowed})."),
            Self::Pt => format!("Tipo de commit desconhecido. Use um tipo padrão ({allowed})."),
            Self::Pl => format!("Nieznany typ commita. Użyj standardowego typu ({allowed})."),
            Self::Tr => format!("Bilinmeyen commit türü. Standart bir tür kullanın ({allowed})."),
            Self::Ru => format!("Неизвестный тип коммита. Используйте стандартный тип ({allowed})."),
            Self::En => format!("Unknown commit type. Expected one of: {allowed}."),
        }
    }

    pub fn check_invalid_header(self) -> &'static str {
        match self {
            Self::ZhCn => "提交标题不符合 Conventional Commits 规范（示例：feat(scope): 简短说明）。",
            Self::ZhTw => "提交標題不符合 Conventional Commits 規範（範例：feat(scope): 簡短說明）。",
            Self::Ja => "コミット件名が Conventional Commits 形式ではありません（例: feat(scope): 説明）。",
            Self::Ko => "커밋 제목이 Conventional Commits 형식이 아닙니다(예: feat(scope): 설명).",
            Self::Fr => "L'en-tête ne respecte pas Conventional Commits (ex. : feat(scope): description).",
            Self::Es => "El encabezado no sigue Conventional Commits (ej.: feat(scope): descripción).",
            Self::Pt => "O cabeçalho não segue o Conventional Commits (ex.: feat(scope): descrição).",
            Self::Pl => "Nagłówek nie jest zgodny z Conventional Commits (np. feat(scope): opis).",
            Self::Tr => "Başlık Conventional Commits biçimine uymuyor (örn. feat(scope): açıklama).",
            Self::Ru => "Заголовок не соответствует Conventional Commits (пример: feat(scope): описание).",
            Self::En => "Commit header does not follow Conventional Commits (e.g. `feat(scope): short summary`).",
        }
    }

    pub fn check_missing_blank_line(self) -> &'static str {
        match self {
            Self::ZhCn => "提交标题与正文之间应保留一个空行。",
            Self::ZhTw => "提交標題與內文之間應保留一個空行。",
            Self::Ja => "コミット件名と本文の間には空行を1行入れてください。",
            Self::Ko => "커밋 제목과 본문 사이에는 빈 줄이 하나 있어야 합니다.",
            Self::Fr => "Séparez l'en-tête et le corps du commit par une ligne vide.",
            Self::Es => "Separa el encabezado y el cuerpo del commit con una línea en blanco.",
            Self::Pt => "Separe o cabeçalho e o corpo do commit com uma linha em branco.",
            Self::Pl => "Oddziel nagłówek od treści commita pustym wierszem.",
            Self::Tr => "Commit başlığı ile gövdesi arasında boş bir satır bırakın.",
            Self::Ru => "Отделите заголовок коммита от основного текста пустой строкой.",
            Self::En => "Separate the commit subject from the body with a blank line.",
        }
    }
}
