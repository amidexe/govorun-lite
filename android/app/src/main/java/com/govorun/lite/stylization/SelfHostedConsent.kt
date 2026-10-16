package com.govorun.lite.stylization

internal object SelfHostedConsent {
    const val NOTICE = "Текст, промпт и API-ключ (если задан) передаются напрямую на ваш сервер. Аудио остаётся на устройстве. Говорун не отвечает за обработку данных на вашем сервере."
    const val ACKNOWLEDGEMENT = "Я доверяю этому серверу и согласен передавать ему данные"
    const val SUBSCRIPTION = "Свой сервер не расходует лимиты Говоруна и не отменяет подписку или её продление."

    fun recipient(endpoint: String, proxyEnabled: Boolean = false, proxyURL: String = ""): String? {
        val value = selfHostedDecision(org.json.JSONObject().put("kind", "recipient")
            .put("endpoint", SelfHostedEndpointPolicy.normalize(endpoint).orEmpty()).put("proxy", proxyEnabled)
            .put("proxyURL", SelfHostedProxy.normalized(proxyURL).orEmpty()))
        return value.optString("value").takeUnless { it == "null" || it.isEmpty() }
    }

    fun accepted(endpoint: String, token: String): Boolean =
        token.isNotEmpty() && recipient(endpoint) == token
}
