# AI Agent Information

This document contains architectural details, technical constraints, and design philosophies to help AI agents understand the Reminders project.

## Architecture

- **Backend Stack**: Kotlin, Java 25, Spring Boot WebFlux (Coroutines via `suspend` functions), SLF4J for logging.
- **Frontend Stack**: React 18, TypeScript, Axios, Webpack (custom config, no Vite/CRA), Bootstrap, Bootstrap Icons.
- **Database**: SQLite 3 using standard JDBC (`spring-boot-starter-jdbc` and `org.xerial:sqlite-jdbc`). We do *not* use R2DBC because standard R2DBC drivers for SQLite conflict with Spring Boot's WebFlux auto-configuration. Database interactions run via `JdbcTemplate` strictly wrapped in `withContext(Dispatchers.IO)`.
- **API Communication**: The backend communicates with external APIs (Apprise) using Spring's reactive `WebClient`. Retries and backoffs are handled transparently via `ExchangeFilterFunction` configurations.

## Conventions

- **Data Models**: Kotlin data classes map directly to JSON payloads. Use `snake_case` in model variables to bypass the need for Jackson `@JsonProperty` annotations, keeping the codebase clean. Avoid `jackson-annotations` dependencies.
- **Authentication**: 
  - **Production** (`!dev` profile): Authentication is assumed to be handled at the proxy level (Authentik). The backend strictly reads `X-authentik-username` and `X-authentik-name` headers via a `WebFilter`.
  - **Development** (`dev` profile): A fallback `devAuthFilter` injects mock credentials into `ServerWebExchange.attributes` natively, driven by `DEV_USERNAME` and `DEV_NAME` configuration properties.
- **Extensions**: We use Kotlin extension properties (e.g., `ServerWebExchange.username`) for clean extraction of attributes injected by the security filters.

## Common Pitfalls

- **Do NOT add R2DBC starters**: Adding `spring-boot-starter-data-r2dbc` prevents `JdbcTemplate` from auto-configuring in this WebFlux setup, breaking the application on startup.
- **Frontend Assets**: Do NOT track the `backend/src/main/resources/public` directory in Git. It is meant to be a transient directory for Webpack compilation output during the multi-stage build process.
- **Jackson Dependencies**: We specifically use Jackson 3 (or the latest provided by Spring Boot 4) relying on implicit field naming rather than aggressive annotation parsing.

## Build Workflows

- The project relies on a multi-stage `Dockerfile` and GitHub Actions (`.github/workflows/main.yml`) to compile the frontend, copy it into the backend's resources, compile the JAR, package the container image, and push it to `ghcr.io`. Always test Docker compatibility when modifying the build lifecycle.
