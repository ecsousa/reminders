package com.reminders.config

import org.springframework.context.annotation.Bean
import org.springframework.context.annotation.Configuration
import org.springframework.context.annotation.Profile
import org.springframework.core.annotation.Order
import org.springframework.http.HttpStatus
import org.springframework.web.server.ServerWebExchange
import org.springframework.web.server.WebFilter
import org.springframework.web.server.WebFilterChain
import reactor.core.publisher.Mono

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

            exchange.attributes["username"] = username
            exchange.attributes["name"] = name
            chain.filter(exchange)
        }
    }

    @Bean
    @Profile("dev")
    @Order(1)
    fun devAuthFilter(): WebFilter {
        return WebFilter { exchange: ServerWebExchange, chain: WebFilterChain ->
            val username = System.getenv("DEV_USERNAME") ?: "dev_user"
            val name = System.getenv("DEV_NAME") ?: "Dev User"
            
            exchange.attributes["username"] = username
            exchange.attributes["name"] = name
            chain.filter(exchange)
        }
    }
}
