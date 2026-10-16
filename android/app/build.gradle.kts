import java.time.Instant
import java.time.ZoneOffset
import java.time.format.DateTimeFormatter
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// Signing inputs stay outside source for private delivery. Source builds may
// omit them and produce an unsigned release, while an explicit path must exist.
val externalSigningProperties = providers.environmentVariable("GOVORUN_SIGNING_PROPERTIES_FILE").orNull
val signingPropertiesFile = externalSigningProperties?.let { file(it) }
    ?: rootProject.file("keystore.properties")
require(externalSigningProperties == null || signingPropertiesFile.isFile) {
    "Explicit release signing properties are unavailable"
}
val keystoreProps = Properties().apply {
    if (signingPropertiesFile.isFile) signingPropertiesFile.inputStream().use { load(it) }
}

// Optional machine-local settings for closed test builds. local.properties
// is gitignored and contains only public Govorun service origins. Explicit
// -P values still take precedence.
val localBuildProps = Properties().apply {
    val f = rootProject.file("local.properties")
    if (f.exists()) load(f.inputStream())
}

// Public origins for the account/statistics service and the isolated official
// styles service. These are routing configuration, not secrets. Empty values
// keep source builds fully local and hide account-only UI until a closed-test
// build explicitly supplies the endpoints.
fun publicBuildUrl(name: String): String = providers.gradleProperty(name)
    .orElse(localBuildProps.getProperty(name, ""))
    .get()
    .trim()
    .replace("\\", "\\\\")
    .replace("\"", "\\\"")

val accountApiUrl = publicBuildUrl("GOVORUN_ACCOUNT_API_URL")
val notificationsApiUrl = publicBuildUrl("GOVORUN_NOTIFICATIONS_API_URL").ifBlank { accountApiUrl }
val officialStylesApiUrl = publicBuildUrl("GOVORUN_OFFICIAL_STYLES_API_URL")
val billingDemoEnabled = providers.gradleProperty("GOVORUN_BILLING_DEMO_ENABLED")
    .orElse(localBuildProps.getProperty("GOVORUN_BILLING_DEMO_ENABLED", "false"))
    .get().toBooleanStrictOrNull() ?: false
val buildId = providers.gradleProperty("GOVORUN_BUILD_ID")
    .orElse(providers.environmentVariable("GOVORUN_BUILD_ID"))
    .orNull
    ?.trim()
    ?.takeIf(String::isNotEmpty)
    ?: DateTimeFormatter.ofPattern("yyyyMMdd.HHmm'Z'")
        .withZone(ZoneOffset.UTC)
        .format(Instant.now())

require(Regex("^[0-9]{8}\\.[0-9]{4}Z$").matches(buildId)) {
    "GOVORUN_BUILD_ID must use the UTC format yyyyMMdd.HHmmZ"
}

val publicReleaseRequested = gradle.startParameter.taskNames.any {
    it.substringAfterLast(':').let { task -> task == "assembleRelease" || task == "bundleRelease" }
}
require(!(billingDemoEnabled && publicReleaseRequested)) {
    "The local billing demo must be disabled in a public release build"
}

android {
    sourceSets.getByName("main").java.srcDir(project.buildFile.canonicalFile.parentFile.resolve("../../shared/schema/kotlin"))
    namespace = "com.govorun.lite"
    compileSdk = 36


    defaultConfig {
        applicationId = "com.govorun.lite"
        minSdk = 31
        targetSdk = 35
        versionCode = 197
        versionName = "2.0.25"
        buildConfigField("String", "BUILD_ID", "\"$buildId\"")
        buildConfigField("String", "ACCOUNT_API_URL", "\"$accountApiUrl\"")
        buildConfigField("String", "NOTIFICATIONS_API_URL", "\"$notificationsApiUrl\"")
        buildConfigField("String", "TBANK_RETURN_SCHEME", "\"govorun\"")
        buildConfigField("String", "ACCOUNT_RETURN_SCHEME", "\"govorun\"")
        buildConfigField("String", "BILLING_BROWSER_PACKAGE", "\"\"")
        buildConfigField("String", "OFFICIAL_STYLES_API_URL", "\"$officialStylesApiUrl\"")
        buildConfigField("boolean", "BILLING_DEMO_ENABLED", billingDemoEnabled.toString())
        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    signingConfigs {
        if (keystoreProps.containsKey("storeFile")) {
            create("release") {
                storeFile = rootProject.file(keystoreProps.getProperty("storeFile"))
                storePassword = keystoreProps.getProperty("storePassword")
                keyAlias = keystoreProps.getProperty("keyAlias")
                keyPassword = keystoreProps.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            versionNameSuffix = "-debug"
            // Default Android debug signing.
            buildConfigField("boolean", "FORCE_ACCESSIBILITY_FALLBACK", "false")
        }

        release {
            buildConfigField("boolean", "FORCE_ACCESSIBILITY_FALLBACK", "false")
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release")
        }

    }

    buildFeatures {
        buildConfig = true
        aidl = true
        compose = true
    }

    // Bundled GigaAM .onnx files are already highly-compressed tensor data;
    // compressing them in the APK adds ~5% at best and forces a full unpack
    // to /data/app_extracted on install. Skipping compression lets the PM
    // store them uncompressed and means our filesDir copy can use plain
    // buffered IO without extra decompression.
    androidResources {
        noCompress += listOf("onnx")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

dependencies {
    implementation(platform("com.google.firebase:firebase-bom:34.18.0"))
    implementation("com.google.firebase:firebase-messaging")
    implementation("com.squareup.okhttp3:okhttp:5.4.0")
    implementation(project(":product-ui"))
    implementation(platform("androidx.compose:compose-bom:2026.05.01"))
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.13.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    implementation("com.adamglin:phosphor-icon:1.0.0")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.browser:browser:1.8.0")
    implementation("androidx.activity:activity-ktx:1.9.0")
    implementation("com.google.android.material:material:1.12.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    // sherpa-onnx v1.13.0: offline ASR engine used with GigaAM v3.
    // AAR must be present at app/libs/sherpa-onnx.aar — see scripts/download-sherpa-onnx.sh.
    implementation(fileTree(mapOf("dir" to "libs", "include" to listOf("*.aar"))))
}

// Optional build customization supplied explicitly by the caller.
providers.gradleProperty("govorunBuildExtension").orNull?.let { apply(from = it) }
apply(from = rootProject.file("settings.gradle.kts").canonicalFile.parentFile.resolve("app/firebase.gradle"))
apply(from = rootProject.file("settings.gradle.kts").canonicalFile.parentFile.resolve("app/shared-business.gradle"))
