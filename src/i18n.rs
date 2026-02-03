// ==========================================
// 国际化 (i18n) 模块
// ==========================================
// 使用 rust-i18n 库
// 支持中文（默认）和英文
// ==========================================
// 注意: rust_i18n::i18n! 宏已在 lib.rs 中初始化
// ==========================================

/// 获取当前语言
pub fn current_locale() -> String {
    rust_i18n::locale().to_string()
}

/// 设置语言
///
/// # 参数
/// - locale: 语言代码（"zh-CN" 或 "en"）
pub fn set_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}

/// 翻译消息（无参数）
///
/// # 示例
/// ```no_run
/// use hot_rolling_aps::i18n::t;
/// let msg = t("common.success");
/// ```
pub fn t(key: &str) -> String {
    rust_i18n::t!(key).to_string()
}

/// 翻译消息（带参数）
///
/// # 示例
/// ```no_run
/// use hot_rolling_aps::i18n::t_with_args;
/// let msg = t_with_args("import.file_not_found", &[("path", "/tmp/test.csv")]);
/// ```
pub fn t_with_args(key: &str, args: &[(&str, &str)]) -> String {
    let mut result = rust_i18n::t!(key).to_string();
    for (k, v) in args {
        let placeholder = format!("%{{{}}}", k);
        result = result.replace(&placeholder, v);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // rust-i18n 的 locale 为全局状态，且 Rust 测试默认并行执行；
    // 为避免测试互相干扰，这里对 i18n 相关测试串行化。
    static LOCALE_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_locale() {
        let _guard = LOCALE_TEST_LOCK.lock().unwrap();
        // 显式设置为默认语言
        set_locale("zh-CN");
        assert_eq!(current_locale(), "zh-CN");
    }

    #[test]
    fn test_set_locale() {
        let _guard = LOCALE_TEST_LOCK.lock().unwrap();
        // 测试切换语言
        set_locale("zh-CN");
        assert_eq!(current_locale(), "zh-CN");

        set_locale("en");
        assert_eq!(current_locale(), "en");

        // 恢复默认语言
        set_locale("zh-CN");
    }

    #[test]
    fn test_translate_simple() {
        let _guard = LOCALE_TEST_LOCK.lock().unwrap();
        // 测试中文翻译
        set_locale("zh-CN");
        let msg = t("common.success");
        assert_eq!(msg, "操作成功");

        // 测试英文翻译
        set_locale("en");
        let msg = t("common.success");
        assert_eq!(msg, "Operation successful");

        // 恢复默认语言
        set_locale("zh-CN");
    }

    #[test]
    fn test_translate_with_args() {
        let _guard = LOCALE_TEST_LOCK.lock().unwrap();
        // 测试中文翻译（带参数）
        set_locale("zh-CN");
        let msg = t_with_args("import.file_not_found", &[("path", "/tmp/test.csv")]);
        assert!(msg.contains("/tmp/test.csv"));
        assert!(msg.contains("文件不存在"));

        // 测试英文翻译（带参数）
        set_locale("en");
        let msg = t_with_args("import.file_not_found", &[("path", "/tmp/test.csv")]);
        assert!(msg.contains("/tmp/test.csv"));
        assert!(msg.contains("File not found"));

        // 恢复默认语言
        set_locale("zh-CN");
    }
}
