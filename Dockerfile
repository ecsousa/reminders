# Stage 1: Build Frontend
FROM node:18-alpine AS frontend-builder
WORKDIR /app
COPY frontend/package*.json ./
RUN npm install
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Backend
FROM eclipse-temurin:25-jdk AS backend-builder
WORKDIR /app

COPY backend/ ./
# Copy frontend dist to backend resources
RUN mkdir -p src/main/resources/public
COPY --from=frontend-builder /app/dist/* src/main/resources/public/
RUN chmod +x gradlew
RUN ./gradlew copyAgent
RUN ./gradlew build -x test

# Stage 3: Final Image
FROM eclipse-temurin:25-jre
WORKDIR /app
COPY --from=backend-builder /app/build/agent/*.jar /app/reactor-tools.jar
COPY --from=backend-builder /app/build/libs/reminders-0.1.jar app.jar

ENV PORT=8080
ENV DB_FOLDER=/data

EXPOSE 8080
ENTRYPOINT ["java", "-javaagent:/app/reactor-tools.jar", "-jar", "app.jar"]
