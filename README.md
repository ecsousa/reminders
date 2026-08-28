# Reminders

A full-stack web application for creating, managing, and triggering scheduled notifications via Apprise. Built with a Rust (Axum) backend and a React + Bootstrap frontend.

## Features

- **Personal Reminders**: Create, list, and delete your reminders.
- **Apprise Integration**: Send notifications to hundreds of services using the Apprise API.
- **Authentication**: Gate-kept by an Authentik proxy in production (via `X-authentik-username` / `X-authentik-name` headers) or standard dev fallbacks.
- **SQLite Database**: Self-contained local database, making it extremely easy to deploy.

## Project Structure

- `backend/`: Rust application built with `axum`, `tokio`, and `rusqlite`.
- `frontend/`: React + TypeScript SPA packed with Webpack and styled with Bootstrap.

## Configuration

The application can be configured via environment variables:

- `APP_PROFILE`: Set to `dev` to bypass Authentik headers and use dummy credentials (default: `prod`).
- `DB_FOLDER`: Directory where the SQLite database (`reminders.db3`) will be stored (default: `.`).
- `APPRISE_ENDPOINT`: The URL of your Apprise instance to trigger notifications (default: `http://localhost:8000/notify`).
- `DEV_USERNAME`: Override the default username in the `dev` profile (default: `dev_user`).
- `DEV_NAME`: Override the default name in the `dev` profile (default: `Dev User`).
- `PORT`: The port the backend server listens on (default: `8080`).

## Local Development

### Backend

1. Navigate to `backend/`.
2. Run using Cargo, specifying the `dev` profile to inject dummy authentication credentials for local testing:
   ```bash
   APP_PROFILE=dev cargo run
   ```

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

You can build the complete application (frontend assets bundled to be served by the backend binary).

### Prerequisites
- Node.js (18+)
- Rust (1.80+)

### Build Steps

1. **Build the Frontend:**
   ```bash
   cd frontend
   npm install
   npm run build
   cd ..
   ```

2. **Copy Frontend Assets to Backend `public/` Directory:**
   ```bash
   mkdir -p public
   cp -r frontend/dist/* public/
   ```
   *Note: When running the backend binary, it looks for a `public/` directory in its current working directory to serve the frontend.*

3. **Build the Backend Binary:**
   ```bash
   cd backend
   cargo build --release
   cd ..
   ```

### Running the Application

Run the compiled binary from the project root (so it can find the `public/` directory), specifying the required environment variables:

```bash
DB_FOLDER=./data APPRISE_ENDPOINT=http://localhost:8000/notify ./backend/target/release/reminders-backend
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
