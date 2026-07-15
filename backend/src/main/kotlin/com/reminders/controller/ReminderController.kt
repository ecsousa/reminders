package com.reminders.controller

import com.reminders.model.Reminder
import com.reminders.model.ReminderCreateRequest
import com.reminders.model.RemindersResponse
import com.reminders.model.UserInfoResponse
import com.reminders.repository.ReminderRepository
import com.reminders.util.name
import com.reminders.util.username
import org.springframework.http.HttpStatus
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.*
import org.springframework.web.server.ServerWebExchange
import kotlinx.coroutines.reactive.awaitFirstOrNull
import org.springframework.web.reactive.function.client.WebClient
import org.springframework.web.reactive.function.client.awaitExchange
import reactor.core.publisher.Mono
import com.reminders.config.AppConfig

@RestController
@RequestMapping("/api")
class ReminderController(
    private val reminderRepository: ReminderRepository,
    private val appConfig: AppConfig
) {

    private val webClient = WebClient.create()

    @GetMapping("/user-info")
    suspend fun getUserInfo(exchange: ServerWebExchange): UserInfoResponse {
        val username = exchange.username
        val name = exchange.name
        return UserInfoResponse(username, name)
    }

    @PostMapping("/reminders")
    suspend fun createReminder(
        @RequestBody request: ReminderCreateRequest,
        exchange: ServerWebExchange
    ): Reminder {
        val username = exchange.username
        val reminder = Reminder(
            username = username,
            reminder_message = request.reminderMessage
        )
        return reminderRepository.save(reminder)
    }

    @GetMapping("/reminders")
    suspend fun getReminders(exchange: ServerWebExchange): RemindersResponse {
        val username = exchange.username
        val reminders = reminderRepository.findAllByUsername(username)
        return RemindersResponse(reminders)
    }

    @DeleteMapping("/reminders/{id}")
    suspend fun deleteReminder(
        @PathVariable id: Long,
        exchange: ServerWebExchange
    ): ResponseEntity<Void> {
        val username = exchange.username
        val deletedCount = reminderRepository.deleteByIdAndUsername(id, username)
        return if (deletedCount > 0) {
            ResponseEntity.noContent().build()
        } else {
            ResponseEntity.notFound().build()
        }
    }

    @PutMapping("/trigger-reminders")
    suspend fun triggerReminders(): ResponseEntity<Void> {
        val reminders = reminderRepository.findAll()
        
        for (reminder in reminders) {
            try {
                var success = false
                var attempts = 0
                while (!success && attempts < 3) {
                    try {
                        val response = webClient.post()
                            .uri(appConfig.appriseUrl)
                            .bodyValue(mapOf("body" to reminder.reminder_message, "title" to "Reminder") as Any)
                            .exchangeToMono { res -> 
                                if (res.statusCode().is2xxSuccessful) {
                                    Mono.just(true)
                                } else {
                                    Mono.just(false)
                                }
                            }
                            .awaitFirstOrNull()
                            
                        if (response == true) {
                            success = true
                            reminder.id?.let { reminderRepository.deleteById(it) }
                        } else {
                            attempts++
                            kotlinx.coroutines.delay(1000L * attempts)
                        }
                    } catch (e: Exception) {
                        attempts++
                        kotlinx.coroutines.delay(1000L * attempts)
                    }
                }
            } catch (e: Exception) {
                // log error
            }
        }
        return ResponseEntity.ok().build()
    }
}
