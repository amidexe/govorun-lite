package com.govorun.lite.stylization

/** Fixed local journal events; never part of anonymous reliability reports. */
internal enum class StyleAccessDiagnostic(val message: String) {
    TOKENS_RECEIVED("Токены доступа к стилизации получены."),
    TOKEN_REJECTED("Сервис отклонил токен доступа. Новые токены доступа будут получены автоматически."),
}
