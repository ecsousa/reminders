package com.reminders.config

import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.context.annotation.Configuration

@Configuration
@ConfigurationProperties(prefix = "reminders")
class AppConfig {
    var dbFolder: String = "."
    var appriseUrl: String = "http://localhost:8000/notify"
    var dev: Dev = Dev()

    class Dev {
        var username: String = "dev_user"
        var name: String = "Dev User"
    }
}
