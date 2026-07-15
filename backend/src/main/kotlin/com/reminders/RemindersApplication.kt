package com.reminders

import org.springframework.boot.autoconfigure.SpringBootApplication
import org.springframework.boot.runApplication

@SpringBootApplication
class RemindersApplication

fun main(args: Array<String>) {
    runApplication<RemindersApplication>(*args)
}
