package com.reminders.controller

import com.reminders.model.Reminder
import com.reminders.model.ReminderCreateRequest
import com.reminders.model.RemindersResponse
import com.reminders.model.UserInfoResponse
import com.reminders.repository.ReminderRepository
import org.springframework.http.HttpStatus
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.*
import org.springframework.web.server.ServerWebExchange
import kotlinx.coroutines.reactive.awaitFirstOrNull
import org.springframework.web.reactive.function.client.WebClient
import org.springframework.web.reactive.function.client.awaitExchange
import org.springframework.web.reactive.function.client.awaitBodilessEntity

@RestController
@RequestMapping("/api")
class ReminderController(private val reminderRepository: ReminderRepository) {

    private val webClient = WebClient.create()

    @GetMapping("/user-info")
    suspend fun getUserInfo(exchange: ServerWebExchange): UserInfoResponse {
        val username = exchange.attributes["username"] as String
        val name = exchange.attributes["name"] as String
        return UserInfoResponse(username, name)
    }

    @PostMapping("/reminders")
    suspend fun createReminder(
        @RequestBody request: ReminderCreateRequest,
        exchange: ServerWebExchange
    ): Reminder {
        val username = exchange.attributes["username"] as String
        val reminder = Reminder(
            username = username,
            reminderMessage = request.reminderMessage
        )
        return reminderRepository.save(reminder)
    }

    @GetMapping("/reminders")
    suspend fun getReminders(exchange: ServerWebExchange): RemindersResponse {
        val username = exchange.attributes["username"] as String
        val reminders = reminderRepository.findAllByUsername(username)
        return RemindersResponse(reminders)
    }

    @DeleteMapping("/reminders/{id}")
    suspend fun deleteReminder(
        @PathVariable id: Long,
        exchange: ServerWebExchange
    ): ResponseEntity<Void> {
        val username = exchange.attributes["username"] as String
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
        val appriseEndpoint = System.getenv("APPRISE_ENDPOINT") ?: "http://localhost:8000/notify"
        
        for (reminder in reminders) {
            try {
                var success = false
                var attempts = 0
                while (!success && attempts < 3) {
                    try {
                        val response = webClient.post()
                            .uri(appriseEndpoint)
                            .bodyValue(mapOf("body" to reminder.reminderMessage, "title" to "Reminder"))
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
