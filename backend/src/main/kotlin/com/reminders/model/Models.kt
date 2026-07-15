package com.reminders.model

import tools.jackson.annotation.JsonProperty
import java.time.LocalDateTime

data class Reminder(
    @JsonProperty("id") val id: Long? = null,
    @JsonProperty("username") val username: String? = null,
    @JsonProperty("created_time") val creationTime: LocalDateTime? = null,
    @JsonProperty("reminder_message") val reminderMessage: String
)

data class ReminderCreateRequest(
    @JsonProperty("reminderMessage") val reminderMessage: String
)

data class RemindersResponse(
    @JsonProperty("reminders") val reminders: List<Reminder>
)

data class UserInfoResponse(
    @JsonProperty("username") val username: String,
    @JsonProperty("name") val name: String
)
