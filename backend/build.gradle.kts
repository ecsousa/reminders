plugins {
    id("org.springframework.boot") version "4.1.0"
    kotlin("jvm") version "2.3.21"
    kotlin("plugin.spring") version "2.3.21"
}

group = "com.reminders"
version = if (project.hasProperty("releaseVersion")) project.property("releaseVersion")!! else "0.1"

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
}

repositories {
    mavenCentral()
}

val reactorAgent by configurations.creating

dependencies {
    implementation(enforcedPlatform("org.springframework.boot:spring-boot-dependencies:4.1.0"))
    reactorAgent(enforcedPlatform("org.springframework.boot:spring-boot-dependencies:4.1.0"))

    implementation("org.springframework.boot:spring-boot-starter-webflux")
    reactorAgent("io.projectreactor:reactor-tools") {
        isTransitive = false
    }

    // Kotlin support
    implementation("org.jetbrains.kotlin:kotlin-reflect")
    implementation("tools.jackson.module:jackson-module-kotlin")
    
    // SQLite and JDBC
    implementation("org.springframework.boot:spring-boot-starter-data-r2dbc")
    implementation("io.r2dbc:r2dbc-spi")
    implementation("org.xerial:sqlite-jdbc:3.45.2.0")

    // Kotlin coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-reactor")
    implementation("io.projectreactor.kotlin:reactor-kotlin-extensions")

    // For SQLite R2DBC support, we might need a specific driver or we can use spring-data-r2dbc.
    // Wait, there is no official r2dbc sqlite driver, but we could use standard JDBC with Kotlin Coroutines 
    // running on Dispatchers.IO, or use `io.r2dbc:r2dbc-sqlite` which exists?
    // Let's use JDBC for SQLite with Spring Boot Data JDBC since SQLite doesn't natively support R2DBC very well,
    // or just use `spring-boot-starter-jdbc` and query directly.
    implementation("org.springframework.boot:spring-boot-starter-jdbc")

    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testImplementation("org.jetbrains.kotlin:kotlin-test-junit5")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test")
}

tasks.named<Test>("test") {
    useJUnitPlatform()
}

tasks.register<Copy>("copyAgent") {
    from(reactorAgent)
    into(layout.buildDirectory.dir("agent"))
}

tasks.named<org.springframework.boot.gradle.tasks.run.BootRun>("bootRun") {
    dependsOn("copyAgent")
    doFirst {
        val agentJar = reactorAgent.singleFile
        jvmArgs("-javaagent:${agentJar.absolutePath}")
    }
}
