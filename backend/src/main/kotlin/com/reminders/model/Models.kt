package com.reminders.model

import java.time.LocalDateTime

data class Reminder(
    val id: Long? = null,
    val username: String? = null,
    val created_time: LocalDateTime? = null,
    val reminder_message: String
)

data class ReminderCreateRequest(
    val reminderMessage: String
)

data class RemindersResponse(
    val reminders: List<Reminder>
)

data class UserInfoResponse(
    val username: String,
    val name: String
)
