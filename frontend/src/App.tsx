import React, { useState, useEffect } from 'react';
import axios from 'axios';

interface UserInfo {
  username: string;
  name: string;
}

interface Reminder {
  id: number;
  created_time: string;
  reminder_message: string;
}

const App: React.FC = () => {
  const [userInfo, setUserInfo] = useState<UserInfo | null>(null);
  const [reminders, setReminders] = useState<Reminder[]>([]);
  const [newMessage, setNewMessage] = useState('');
  const [selectedReminderId, setSelectedReminderId] = useState<number | null>(null);

  useEffect(() => {
    fetchUserInfo();
    fetchReminders();
  }, []);

  const fetchUserInfo = async () => {
    try {
      const response = await axios.get('/api/user-info');
      setUserInfo(response.data);
    } catch (error) {
      console.error('Error fetching user info:', error);
    }
  };

  const fetchReminders = async () => {
    try {
      const response = await axios.get('/api/reminders');
      setReminders(response.data.reminders || []);
    } catch (error) {
      console.error('Error fetching reminders:', error);
    }
  };

  const handleAddReminder = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newMessage.trim()) return;
    try {
      await axios.post('/api/reminders', { reminderMessage: newMessage });
      setNewMessage('');
      fetchReminders();
    } catch (error) {
      console.error('Error adding reminder:', error);
    }
  };

  const handleDeleteReminder = async (id: number) => {
    try {
      await axios.delete(`/api/reminders/${id}`);
      fetchReminders();
    } catch (error) {
      console.error('Error deleting reminder:', error);
    }
  };

  return (
    <div className="container app-container">
      <h1 className="mb-4 text-center">
        Reminders for {userInfo?.name || '...'}
      </h1>

      <form onSubmit={handleAddReminder} className="mb-4">
        <div className="input-group">
          <input
            type="text"
            className="form-control"
            placeholder="Create new reminder"
            value={newMessage}
            onChange={(e) => setNewMessage(e.target.value)}
          />
          <button className="btn btn-primary" type="submit">
            <i className="bi bi-plus-lg"></i>
          </button>
        </div>
      </form>

      <ul className="list-group">
        {reminders.map((reminder) => (
          <li
            key={reminder.id}
            className={`list-group-item d-flex justify-content-between align-items-center reminder-item ${selectedReminderId === reminder.id ? 'active-item' : ''}`}
            onClick={() => setSelectedReminderId(selectedReminderId === reminder.id ? null : reminder.id)}
            style={{ cursor: 'pointer' }}
          >
            <div>
              <span className="d-block fw-medium" style={{ fontSize: '1.1rem' }}>{reminder.reminder_message}</span>
              <small className="text-muted">
                {new Date(reminder.created_time).toLocaleString()}
              </small>
            </div>
            {selectedReminderId === reminder.id && (
              <button
                className="btn btn-danger btn-sm"
                onClick={(e) => {
                  e.stopPropagation();
                  handleDeleteReminder(reminder.id);
                }}
              >
                <i className="bi bi-trash-fill"></i>
              </button>
            )}
          </li>
        ))}
      </ul>
      {reminders.length === 0 && (
        <div className="text-center empty-state mt-5 mb-3">
          <i className="bi bi-calendar-x" style={{ fontSize: '3rem', color: '#adb5bd' }}></i>
          <p className="mt-2 text-muted" style={{ fontSize: '1.1rem' }}>No reminders yet. Create one above!</p>
        </div>
      )}
    </div>
  );
};

export default App;
