plugins {
    id("java")
    id("org.jetbrains.intellij") version "1.17.4"
}

group = "dev.kontrocode"
version = "0.1.0"

repositories {
    mavenCentral()
}

intellij {
    version.set("2024.1")
    type.set("IC")
    plugins.set(listOf("com.intellij.java"))
}

tasks {
    patchPluginXml {
        sinceBuild.set("241")
        untilBuild.set("242.*")
    }
}
