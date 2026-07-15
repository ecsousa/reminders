# Reminders

A full-stack web application for creating, managing, and triggering scheduled notifications via Apprise. Built with a Kotlin Spring Boot (WebFlux) backend and a React + Bootstrap frontend.

## Features

- **Personal Reminders**: Create, list, and delete your reminders.
- **Apprise Integration**: Send notifications to hundreds of services using the Apprise API.
- **Authentication**: Gate-kept by an Authentik proxy in production (via `X-authentik-username` / `X-authentik-name` headers) or standard dev fallbacks.
- **SQLite Database**: Self-contained local database, making it extremely easy to deploy.

## Project Structure

- `backend/`: Kotlin Spring Boot application (WebFlux) utilizing coroutines.
- `frontend/`: React + TypeScript SPA packed with Webpack and styled with Bootstrap.

## Configuration

The application can be configured via environment variables (or overridden via `application.yml` and `application-dev.yml`):

- `DB_FOLDER`: Directory where the SQLite database (`reminders.db3`) will be stored (default: `.`).
- `APPRISE_ENDPOINT`: The URL of your Apprise instance to trigger notifications (default: `http://localhost:8000/notify`).
- `DEV_USERNAME`: Override the default username in the `dev` profile (default: `dev_user`).
- `DEV_NAME`: Override the default name in the `dev` profile (default: `Dev User`).

## Local Development

### Backend

1. Navigate to `backend/`.
2. Run using Gradle:
   ```bash
   ./gradlew bootRun --args='--spring.profiles.active=dev'
   ```
   *Note: Using the `dev` profile injects dummy authentication credentials for local testing.*

### Frontend

1. Navigate to `frontend/`.
2. Install dependencies:
   ```bash
   npm install
   ```
3. Start the dev server (proxies `/api` to port `8080`):
   ```bash
   npm start
   ```

## Building and Running Locally

You can build the complete application (frontend assets bundled inside the backend JAR).

### Prerequisites
- Node.js (18+)
- Java 25+
- Gradle

### Build Steps

1. **Build the Frontend:**
   ```bash
   cd frontend
   npm install
   npm run build
   cd ..
   ```

2. **Copy Frontend Assets to Backend:**
   ```bash
   mkdir -p backend/src/main/resources/public
   cp -r frontend/dist/* backend/src/main/resources/public/
   ```

3. **Build the Backend JAR:**
   ```bash
   cd backend
   ./gradlew build -x test
   cd ..
   ```

### Running the JAR

Run the assembled JAR using the `java -jar` command, specifying the required environment variables:

```bash
DB_FOLDER=./data APPRISE_ENDPOINT=http://localhost:8000/notify java -jar backend/build/libs/reminders-0.1.jar
```

The application will be accessible at `http://localhost:8080`.

## Docker Deployment

The official Docker image is hosted at GitHub Container Registry: `ghcr.io/ecsousa/reminders`.

### Using Docker Compose (Recommended)

Create a `docker-compose.yml` file:

```yaml
version: '3.8'
services:
  reminders:
    image: ghcr.io/ecsousa/reminders:main
    ports:
      - "8080:8080"
    environment:
      - DB_FOLDER=/data
      - APPRISE_ENDPOINT=http://apprise:8000/notify
    volumes:
      - ./data:/data
    restart: unless-stopped
```

Run with:
```bash
docker-compose up -d
```
