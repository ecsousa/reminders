package com.reminders.config

import com.reminders.util.name
import com.reminders.util.username
import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.context.annotation.Profile
import org.springframework.core.annotation.Order
import org.springframework.http.HttpStatus
import org.springframework.web.server.ServerWebExchange
import org.springframework.web.server.WebFilter
import org.springframework.web.server.WebFilterChain

@Configuration
class SecurityConfig {

    @Bean
    @Profile("!dev")
    @Order(1)
    fun prodAuthFilter(): WebFilter {
        return WebFilter { exchange: ServerWebExchange, chain: WebFilterChain ->
            val username = exchange.request.headers.getFirst("X-authentik-username")
            val name = exchange.request.headers.getFirst("X-authentik-name")

            if (username.isNullOrBlank() || name.isNullOrBlank()) {
                exchange.response.statusCode = HttpStatus.UNAUTHORIZED
                return@WebFilter exchange.response.setComplete()
            }

            exchange.username = username
            exchange.name = name
            chain.filter(exchange)
        }
    }

    @Bean
    @Profile("dev")
    @Order(1)
    fun devAuthFilter(appConfig: AppConfig): WebFilter {
        return WebFilter { exchange: ServerWebExchange, chain: WebFilterChain ->
            exchange.username = appConfig.dev.username
            exchange.name = appConfig.dev.name
            chain.filter(exchange)
        }
    }
}
