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

@RestController
@RequestMapping("/api")
class ReminderController(
    private val reminderRepository: ReminderRepository,
    private val appConfig: AppConfig
) {

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
                webClient.post()
                    .uri(appConfig.appriseUrl)
                    .bodyValue(mapOf("body" to reminder.reminder_message, "title" to "Reminder") as Any)
                    .retrieve()
                    .awaitBodilessEntity()
                    
                reminder.id?.let { reminderRepository.deleteById(it) }
            } catch (e: Exception) {
                // log error
            }
        }
        return ResponseEntity.ok().build()
    }
}
