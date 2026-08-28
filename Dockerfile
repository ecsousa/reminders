# Stage 1: Build Frontend
FROM node:18-alpine AS frontend-builder
WORKDIR /app
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Backend
FROM rust:1.80-slim-bookworm AS backend-builder
WORKDIR /app

# Install build dependencies for SQLite
RUN apt-get update && apt-get install -y libsqlite3-dev pkg-config

COPY backend/ ./
RUN cargo build --release

# Stage 3: Final Image
FROM debian:bookworm-slim
WORKDIR /app

# Install runtime dependencies for SQLite
RUN apt-get update && apt-get install -y libsqlite3-0 ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=backend-builder /app/target/release/reminders-backend /app/reminders-backend
COPY --from=frontend-builder /app/dist/ /app/public/

ENV PORT=8080
ENV DB_FOLDER=/data

EXPOSE 8080
ENTRYPOINT ["/app/reminders-backend"]
