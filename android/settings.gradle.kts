pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "govorun-lite"
include(":app")
include(":product-ui")
providers.gradleProperty("govorunSettingsExtension").orNull?.let { apply(from = it) }
