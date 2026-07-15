package com.reminders.controller

import com.reminders.model.Reminder
import com.reminders.model.ReminderCreateRequest
import com.reminders.model.RemindersResponse
import com.reminders.model.UserInfoResponse
import com.reminders.repository.ReminderRepository
import com.reminders.util.name
import com.reminders.util.username
import org.springframework.http.ResponseEntity
import org.springframework.web.bind.annotation.*
import org.springframework.web.server.ServerWebExchange
import org.springframework.web.reactive.function.client.WebClient
import org.springframework.web.reactive.function.client.awaitExchange
import org.springframework.web.reactive.function.client.awaitBodilessEntity
import reactor.core.publisher.Mono
import com.reminders.config.AppConfig
import reactor.util.retry.Retry
import java.time.Duration
import org.springframework.web.reactive.function.client.ClientResponse
import org.slf4j.LoggerFactory

@RestController
@RequestMapping("/api")
class ReminderController(
    private val reminderRepository: ReminderRepository,
    private val appConfig: AppConfig
) {

    private val logger = LoggerFactory.getLogger(ReminderController::class.java)

    private val webClient = WebClient.builder()
        .filter { request, next ->
            next.exchange(request)
                .flatMap { response ->
                    if (response.statusCode().isError) {
                        response.createException().flatMap { Mono.error<ClientResponse>(it) }
                    } else {
                        Mono.just(response)
                    }
                }
                .retryWhen(Retry.backoff(3, Duration.ofSeconds(1)))
        }
        .build()

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
        logger.info("User {} is creating a new reminder", username)
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
        logger.info("User {} is deleting reminder {}", username, id)
        val deletedCount = reminderRepository.deleteByIdAndUsername(id, username)
        return if (deletedCount > 0) {
            ResponseEntity.noContent().build()
        } else {
            ResponseEntity.notFound().build()
        }
    }

    @PutMapping("/trigger-reminders")
    suspend fun triggerReminders(): ResponseEntity<Void> {
        logger.info("Triggering all pending reminders")
        val reminders = reminderRepository.findAll()
        
        for (reminder in reminders) {
            try {
                webClient.post()
                    .uri(appConfig.appriseUrl)
                    .bodyValue(mapOf("body" to reminder.reminder_message, "title" to "Reminder") as Any)
                    .retrieve()
                    .awaitBodilessEntity()
                    
                logger.debug("Successfully triggered reminder {}", reminder.id)
                reminder.id?.let { reminderRepository.deleteById(it) }
            } catch (e: Exception) {
                logger.error("Failed to trigger reminder ${reminder.id} after retries", e)
            }
        }
        return ResponseEntity.ok().build()
    }
}
