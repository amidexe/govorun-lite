package com.govorun.lite.stylization

import java.net.InetSocketAddress
import java.net.Proxy
import java.net.URI

internal object SelfHostedProxy {
    fun address(value: String): URI = selfHostedAddress(value, true)

    fun normalized(value: String): String? = runCatching { address(value).toASCIIString() }.getOrNull()

    fun route(enabled: Boolean, value: String): Proxy {
        if (!enabled) return Proxy.NO_PROXY
        val uri = address(value)
        val type = if (uri.scheme == "socks5") Proxy.Type.SOCKS else Proxy.Type.HTTP
        val socket = InetSocketAddress.createUnresolved(uri.host, uri.port)
        return Proxy(type, socket)
    }
}
