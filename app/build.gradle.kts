import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

// Release signing config is read from keystore.properties at repo root.
// That file is gitignored (see keystore.properties.template for format).
// If the file is missing — release builds fall back to unsigned and print a
// warning, which is fine for CI / source-drop builds on other machines.
val keystoreProps = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.exists()) load(f.inputStream())
}

android {
    namespace = "com.govorun.lite"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.govorun.lite"
        minSdk = 33
        targetSdk = 35
        versionCode = 4
        versionName = "1.0.3"
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
            // Default Android debug signing.
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release")
        }
    }

    buildFeatures {
        buildConfig = true
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
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.activity:activity-ktx:1.9.0")
    implementation("androidx.fragment:fragment-ktx:1.8.5")
    implementation("androidx.viewpager2:viewpager2:1.1.0")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.constraintlayout:constraintlayout:2.2.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    // sherpa-onnx v1.12.34: offline ASR engine used with GigaAM v3.
    // AAR must be present at app/libs/sherpa-onnx.aar — see scripts/download-sherpa-onnx.sh.
    implementation(fileTree(mapOf("dir" to "libs", "include" to listOf("*.aar"))))
}
