package com.reminders.repository

import com.reminders.model.Reminder
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.springframework.jdbc.core.JdbcTemplate
import org.springframework.stereotype.Repository
import java.sql.ResultSet
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

@Repository
class ReminderRepository(private val jdbcTemplate: JdbcTemplate) {

    private val formatter = DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss")

    suspend fun save(reminder: Reminder): Reminder = withContext(Dispatchers.IO) {
        val now = LocalDateTime.now()
        val formattedDate = now.format(formatter)
        
        val sql = "INSERT INTO reminders (username, creation_time, reminder_message) VALUES (?, ?, ?)"
        jdbcTemplate.update(sql, reminder.username, formattedDate, reminder.reminder_message)
        
        val id = jdbcTemplate.queryForObject("SELECT last_insert_rowid()", Long::class.java)!!
        
        reminder.copy(id = id, created_time = now)
    }

    suspend fun findAllByUsername(username: String): List<Reminder> = withContext(Dispatchers.IO) {
        val sql = "SELECT * FROM reminders WHERE username = ? ORDER BY creation_time ASC"
        jdbcTemplate.query(sql, { rs: ResultSet, _: Int ->
            Reminder(
                id = rs.getLong("id"),
                username = rs.getString("username"),
                created_time = LocalDateTime.parse(rs.getString("creation_time"), formatter),
                reminder_message = rs.getString("reminder_message")
            )
        }, username)
    }
    
    suspend fun findAll(): List<Reminder> = withContext(Dispatchers.IO) {
        val sql = "SELECT * FROM reminders"
        jdbcTemplate.query(sql) { rs: ResultSet, _: Int ->
            Reminder(
                id = rs.getLong("id"),
                username = rs.getString("username"),
                created_time = LocalDateTime.parse(rs.getString("creation_time"), formatter),
                reminder_message = rs.getString("reminder_message")
            )
        }
    }

    suspend fun deleteByIdAndUsername(id: Long, username: String): Int = withContext(Dispatchers.IO) {
        val sql = "DELETE FROM reminders WHERE id = ? AND username = ?"
        jdbcTemplate.update(sql, id, username)
    }
    
    suspend fun deleteById(id: Long): Int = withContext(Dispatchers.IO) {
        val sql = "DELETE FROM reminders WHERE id = ?"
        jdbcTemplate.update(sql, id)
    }
}
