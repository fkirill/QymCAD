use qymcad_core::errors::CoreError;

struct LocalizedCounts {
    language: &'static str,
    expected_prefix: &'static str,
}

#[test]
fn profile_loss_warning_carries_counts_in_every_language() {
    let previous = qymcad_i18n::language();
    let warning = CoreError::ProfilesDropped { asked: 5, dropped: 2 };
    let mut failures = Vec::new();
    for expected in [
        LocalizedCounts { language: "en", expected_prefix: "2" },
        LocalizedCounts { language: "ru", expected_prefix: "2" },
        LocalizedCounts { language: "kk", expected_prefix: "5" },
        LocalizedCounts { language: "uk", expected_prefix: "2" },
    ] {
        qymcad_i18n::set_language(expected.language);
        let message = qymcad_i18n::error_words::error_text(&warning);
        if !message.starts_with(expected.expected_prefix) || !message.contains('2') || !message.contains('5') || message.contains(['$', '{', '}']) {
            failures.push(format!("{}: expected resolved counts starting with {:?}, got {message:?}", expected.language, expected.expected_prefix));
        }
    }
    qymcad_i18n::set_language(&previous);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
