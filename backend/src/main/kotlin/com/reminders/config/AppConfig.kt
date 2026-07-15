package com.reminders.config

import org.springframework.boot.context.properties.ConfigurationProperties
import org.springframework.context.annotation.Configuration
import jakarta.annotation.PostConstruct
import org.slf4j.LoggerFactory
import java.io.File

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

    private val logger = LoggerFactory.getLogger(AppConfig::class.java)

    @PostConstruct
    fun logDbPath() {
        val dbFile = File(dbFolder, "reminders.db3")
        logger.info("Using database file at: {}", dbFile.absolutePath)
    }
}
