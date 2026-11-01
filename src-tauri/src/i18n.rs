use crate::settings;
use std::sync::LazyLock;

/// The interface languages, as in Zanrei.
pub const UI_LANGUAGES: &[&str] = &["en", "tr", "es", "de", "fr", "pt", "ru", "zh", "ja", "ar"];

/// The system's language, two letters ("tr"), or "en" when it is not one of UI_LANGUAGES.
static SYSTEM: LazyLock<&'static str> = LazyLock::new(|| {
    let tag = system_tag().to_lowercase();
    let base = tag.split(['-', '_', '.']).next().unwrap_or("");
    UI_LANGUAGES
        .iter()
        .find(|l| **l == base)
        .copied()
        .unwrap_or("en")
});

#[cfg(target_os = "windows")]
fn system_tag() -> String {
    use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..(len.max(1) - 1) as usize])
}

#[cfg(target_os = "macos")]
fn system_tag() -> String {
    // ( "tr-TR", "en-US" ): the first entry is the UI language.
    std::process::Command::new("defaults")
        .args(["read", "-g", "AppleLanguages"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .trim_start_matches(|c: char| c == '(' || c == '"' || c.is_whitespace())
                .to_string()
        })
        .unwrap_or_default()
}

#[cfg(target_os = "linux")]
fn system_tag() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .unwrap_or_default()
}

pub fn system() -> &'static str {
    *SYSTEM
}

/// The interface language: the one picked in Settings, else the system's.
pub fn lang() -> &'static str {
    let picked = settings::current().ui_lang;
    UI_LANGUAGES
        .iter()
        .find(|l| **l == picked)
        .copied()
        .unwrap_or(*SYSTEM)
}

/// The tray menu: (open, settings, Discord report, quit) in every interface language.
pub fn tray_texts() -> [&'static str; 4] {
    match lang() {
        "tr" => ["Aç", "Ayarlar", "Discord tanı raporu", "Çıkış"],
        "es" => ["Abrir", "Ajustes", "Informe de Discord", "Salir"],
        "de" => ["Öffnen", "Einstellungen", "Discord-Bericht", "Beenden"],
        "fr" => ["Ouvrir", "Paramètres", "Rapport Discord", "Quitter"],
        "pt" => ["Abrir", "Configurações", "Relatório do Discord", "Sair"],
        "ru" => ["Открыть", "Настройки", "Отчёт Discord", "Выход"],
        "zh" => ["打开", "设置", "Discord 诊断报告", "退出"],
        "ja" => ["開く", "設定", "Discord 診断レポート", "終了"],
        "ar" => ["فتح", "الإعدادات", "تقرير Discord", "خروج"],
        _ => ["Open", "Settings", "Discord report", "Quit"],
    }
}

/// Every notification and error the Rust side shows, in the ten interface languages.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(windows), allow(dead_code))] // the Discord messages are Windows-only
pub enum Msg {
    ReportWritten,
    NoDiscord,
    NotSent,
    BadHotkey,
    HotkeyTaken,
    Busy,
    NoText,
    Empty,
    SameLang,
    Timeout,
    Connection,
    AccessDenied,
    TooMany,
    Quota,
    BadAnswer,
    NeedKey,
    Updating,
}

/// The text of `m` in the interface language; `{0}`, `{1}` are replaced with `args`.
pub fn msg(m: Msg, args: &[&dyn std::fmt::Display]) -> String {
    let i = UI_LANGUAGES.iter().position(|l| *l == lang()).unwrap_or(0);
    let mut text = texts(m)[i].to_string();
    for (n, arg) in args.iter().enumerate() {
        text = text.replace(&format!("{{{n}}}"), &arg.to_string());
    }
    text
}

/// Columns in the order of UI_LANGUAGES.
fn texts(m: Msg) -> [&'static str; 10] {
    match m {
        Msg::ReportWritten => [
            "Discord report written. The file contains the messages that were on screen.",
            "Discord tanı raporu oluşturuldu. Bu dosya ekranda görünen mesajları içerir.",
            "Informe de Discord creado. El archivo contiene los mensajes que había en pantalla.",
            "Discord-Bericht geschrieben. Die Datei enthält die Nachrichten, die auf dem Bildschirm waren.",
            "Rapport Discord créé. Le fichier contient les messages qui étaient à l'écran.",
            "Relatório do Discord criado. O arquivo contém as mensagens que estavam na tela.",
            "Отчёт Discord создан. Файл содержит сообщения, которые были на экране.",
            "Discord 诊断报告已生成。文件包含屏幕上显示的消息。",
            "Discord 診断レポートを作成しました。画面上のメッセージが含まれます。",
            "تم إنشاء تقرير Discord. يحتوي الملف على الرسائل التي كانت على الشاشة.",
        ],
        Msg::NoDiscord => [
            "No open Discord window found",
            "Açık bir Discord penceresi bulunamadı",
            "No hay ninguna ventana de Discord abierta",
            "Kein offenes Discord-Fenster gefunden",
            "Aucune fenêtre Discord ouverte",
            "Nenhuma janela do Discord aberta",
            "Открытое окно Discord не найдено",
            "未找到打开的 Discord 窗口",
            "開いている Discord ウィンドウが見つかりません",
            "لم يُعثر على نافذة Discord مفتوحة",
        ],
        Msg::NotSent => [
            "Message not sent, translation failed: {0}\nPress Enter again to send it untranslated.",
            "Mesaj gönderilmedi, çeviri başarısız: {0}\nÇevirmeden göndermek için tekrar Enter'a basın.",
            "Mensaje no enviado, la traducción falló: {0}\nPulsa Intro otra vez para enviarlo sin traducir.",
            "Nachricht nicht gesendet, Übersetzung fehlgeschlagen: {0}\nNoch einmal Enter drücken, um sie unübersetzt zu senden.",
            "Message non envoyé, la traduction a échoué : {0}\nAppuyez de nouveau sur Entrée pour l'envoyer sans traduction.",
            "Mensagem não enviada, a tradução falhou: {0}\nPressione Enter de novo para enviá-la sem tradução.",
            "Сообщение не отправлено, перевод не удался: {0}\nНажмите Enter ещё раз, чтобы отправить без перевода.",
            "消息未发送，翻译失败：{0}\n再按一次 Enter 可不翻译直接发送。",
            "メッセージは送信されませんでした。翻訳に失敗しました：{0}\nもう一度 Enter を押すと翻訳せずに送信します。",
            "لم تُرسل الرسالة، فشلت الترجمة: {0}\nاضغط Enter مرة أخرى لإرسالها دون ترجمة.",
        ],
        Msg::BadHotkey => [
            "\"{0}\" is not a valid hotkey",
            "\"{0}\" geçerli bir kısayol değil",
            "«{0}» no es un atajo válido",
            "„{0}“ ist kein gültiges Tastenkürzel",
            "« {0} » n'est pas un raccourci valide",
            "\"{0}\" não é um atalho válido",
            "«{0}» — недопустимое сочетание клавиш",
            "“{0}”不是有效的快捷键",
            "「{0}」は有効なショートカットではありません",
            "«{0}» ليس اختصارًا صالحًا",
        ],
        Msg::HotkeyTaken => [
            "{0} is taken by another app, pick a different one",
            "{0} başka bir uygulama tarafından kullanılıyor, başka bir kısayol seçin",
            "{0} lo usa otra aplicación, elige otro",
            "{0} wird von einer anderen App verwendet, bitte ein anderes wählen",
            "{0} est déjà utilisé par une autre app, choisissez-en un autre",
            "{0} já é usado por outro aplicativo, escolha outro",
            "{0} занято другим приложением, выберите другое",
            "{0} 已被其他应用占用，请换一个",
            "{0} は別のアプリが使用中です。別のものを選んでください",
            "{0} يستخدمه تطبيق آخر، اختر اختصارًا غيره",
        ],
        Msg::Busy => [
            "Already translating",
            "Çeviri zaten sürüyor",
            "Ya se está traduciendo",
            "Es wird bereits übersetzt",
            "Une traduction est déjà en cours",
            "Já está traduzindo",
            "Перевод уже идёт",
            "正在翻译中",
            "すでに翻訳中です",
            "الترجمة جارية بالفعل",
        ],
        Msg::NoText => [
            "No text to copy: click into a text box or select some text",
            "Kopyalanacak metin bulunamadı: bir metin kutusuna tıklayın ya da metin seçin",
            "No hay texto que copiar: haz clic en un cuadro de texto o selecciona texto",
            "Kein Text zum Kopieren: in ein Textfeld klicken oder Text markieren",
            "Aucun texte à copier : cliquez dans un champ de texte ou sélectionnez du texte",
            "Nenhum texto para copiar: clique em uma caixa de texto ou selecione um texto",
            "Нет текста для копирования: щёлкните в поле ввода или выделите текст",
            "没有可复制的文字：请点击输入框或选中文字",
            "コピーするテキストがありません。入力欄をクリックするかテキストを選択してください",
            "لا يوجد نص للنسخ: انقر داخل مربع نص أو حدّد نصًا",
        ],
        Msg::Empty => [
            "There is no text to translate",
            "Çevrilecek metin boş",
            "No hay texto que traducir",
            "Es gibt keinen Text zum Übersetzen",
            "Il n'y a aucun texte à traduire",
            "Não há texto para traduzir",
            "Нет текста для перевода",
            "没有需要翻译的文字",
            "翻訳するテキストがありません",
            "لا يوجد نص للترجمة",
        ],
        Msg::SameLang => [
            "Source and target language are the same",
            "Kaynak ve hedef dil aynı",
            "El idioma de origen y el de destino son el mismo",
            "Ausgangs- und Zielsprache sind gleich",
            "La langue source et la langue cible sont identiques",
            "Os idiomas de origem e destino são iguais",
            "Исходный и целевой языки совпадают",
            "源语言和目标语言相同",
            "翻訳元と翻訳先が同じ言語です",
            "لغة المصدر واللغة الهدف متطابقتان",
        ],
        Msg::Timeout => [
            "{0} did not answer in time",
            "{0} yanıt vermedi (zaman aşımı)",
            "{0} no respondió a tiempo",
            "{0} hat nicht rechtzeitig geantwortet",
            "{0} n'a pas répondu à temps",
            "{0} não respondeu a tempo",
            "{0} не ответил вовремя",
            "{0} 未及时响应",
            "{0} が時間内に応答しませんでした",
            "لم يستجب {0} في الوقت المناسب",
        ],
        Msg::Connection => [
            "{0} connection error: {1}",
            "{0} bağlantı hatası: {1}",
            "Error de conexión con {0}: {1}",
            "Verbindungsfehler bei {0}: {1}",
            "Erreur de connexion à {0} : {1}",
            "Erro de conexão com {0}: {1}",
            "Ошибка подключения к {0}: {1}",
            "{0} 连接错误：{1}",
            "{0} 接続エラー：{1}",
            "خطأ في الاتصال بـ{0}: {1}",
        ],
        Msg::AccessDenied => [
            "access denied, check the API key",
            "erişim reddedildi, API anahtarını kontrol edin",
            "acceso denegado, revisa la clave de API",
            "Zugriff verweigert, API-Schlüssel prüfen",
            "accès refusé, vérifiez la clé d'API",
            "acesso negado, verifique a chave de API",
            "доступ запрещён, проверьте API-ключ",
            "访问被拒绝，请检查 API 密钥",
            "アクセスが拒否されました。API キーを確認してください",
            "تم رفض الوصول، تحقق من مفتاح API",
        ],
        Msg::TooMany => [
            "too many requests, wait a moment",
            "çok fazla istek, biraz bekleyin",
            "demasiadas solicitudes, espera un momento",
            "zu viele Anfragen, kurz warten",
            "trop de requêtes, patientez un instant",
            "muitas solicitações, aguarde um pouco",
            "слишком много запросов, подождите немного",
            "请求过多，请稍候",
            "リクエストが多すぎます。少し待ってください",
            "طلبات كثيرة جدًا، انتظر قليلًا",
        ],
        Msg::Quota => [
            "the quota is used up",
            "kota doldu",
            "se agotó la cuota",
            "das Kontingent ist aufgebraucht",
            "le quota est épuisé",
            "a cota acabou",
            "квота исчерпана",
            "配额已用完",
            "利用枠を使い切りました",
            "نفدت الحصة",
        ],
        Msg::BadAnswer => [
            "{0} sent an unexpected answer: {1}",
            "{0} beklenmeyen bir yanıt verdi: {1}",
            "{0} envió una respuesta inesperada: {1}",
            "{0} hat unerwartet geantwortet: {1}",
            "{0} a renvoyé une réponse inattendue : {1}",
            "{0} enviou uma resposta inesperada: {1}",
            "{0} прислал неожиданный ответ: {1}",
            "{0} 返回了意外的响应：{1}",
            "{0} から予期しない応答がありました：{1}",
            "أرسل {0} ردًا غير متوقع: {1}",
        ],
        Msg::NeedKey => [
            "{0} needs an API key",
            "{0} API anahtarı gerekli",
            "{0} necesita una clave de API",
            "{0} braucht einen API-Schlüssel",
            "{0} a besoin d'une clé d'API",
            "{0} precisa de uma chave de API",
            "Для {0} нужен API-ключ",
            "{0} 需要 API 密钥",
            "{0} には API キーが必要です",
            "يحتاج {0} إلى مفتاح API",
        ],
        Msg::Updating => [
            "Updating QuickTranslate to {0}…",
            "QuickTranslate {0} sürümüne güncelleniyor…",
            "Actualizando QuickTranslate a {0}…",
            "QuickTranslate wird auf {0} aktualisiert…",
            "Mise à jour de QuickTranslate vers {0}…",
            "Atualizando o QuickTranslate para {0}…",
            "QuickTranslate обновляется до {0}…",
            "正在将 QuickTranslate 更新到 {0}…",
            "QuickTranslate を {0} に更新しています…",
            "جارٍ تحديث QuickTranslate إلى {0}…",
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_in_arguments() {
        assert_eq!(
            msg(Msg::Connection, &[&"Google", &"timed out"]),
            texts(Msg::Connection)[UI_LANGUAGES.iter().position(|l| *l == lang()).unwrap()]
                .replace("{0}", "Google")
                .replace("{1}", "timed out")
        );
        assert!(UI_LANGUAGES[0] == "en" && UI_LANGUAGES[1] == "tr");
    }
}
