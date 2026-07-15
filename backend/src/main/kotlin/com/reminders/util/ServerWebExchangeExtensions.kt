package com.reminders.util

import org.springframework.web.server.ServerWebExchange

var ServerWebExchange.username: String
    get() = this.attributes["username"] as String
    set(value) {
        this.attributes["username"] = value
    }

var ServerWebExchange.name: String
    get() = this.attributes["name"] as String
    set(value) {
        this.attributes["name"] = value
    }