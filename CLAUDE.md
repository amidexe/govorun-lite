# Говорун Lite

Минимальное open-source Android-приложение для голосового ввода на русском. Офлайн-распознавание через GigaAM v3 (Sber, MIT). Цель — публикация в RuStore.

## Пакет и структура

Package: `com.govorun.lite` — не путать с `com.govorun.app` (Pro в `/home/tttt/projects/govorun/`, остаётся нетронутым).

```
com.govorun.lite/
├── LiteApp.kt                  — Application (DynamicColors)
├── model/                      — GigaAmModel (hardcoded config), ModelDownloader
├── overlay/BubbleView.kt       — упрощённый Canvas-бабл
├── service/
│   └── LiteAccessibilityService.kt  — VAD-only, commitText + clipboard fallback
├── transcriber/                — Transcriber interface, VadRecorder, OfflineTranscriber
├── ui/
│   ├── OnboardingActivity.kt   — 8-шаговый мастер (пока stub)
│   └── MainActivity.kt         — пост-онбординг экран (пока stub)
└── util/AppLog.kt              — файловый журнал
```

## Что есть и чего нет

**Есть:**
- Один режим bubble — **VAD** (тап старт → паузы дают абзацы → тап стоп)
- Accessibility service для вставки текста в любое поле
- Офлайн-распознавание GigaAM v3 через sherpa-onnx (NNAPI → CPU fallback)
- Silero VAD бандлится в `assets/` (629 КБ) — не скачивается

**Нет и не будет:**
- Push-to-talk / hold-режима, кнопок ✓/✗ (в Lite нет корректировки текста)
- AI-форматирования, облачных транскрайберов
- Диктофона, базы данных, словаря автозамен
- Смены стилей/цветов бабла
- Зависимостей от Google Play Services (требование RuStore)

## Сборка

### Первая сборка

```bash
# Один раз — скачать AAR нативного рантайма (~47 МБ, gitignored)
./scripts/download-sherpa-onnx.sh

# Проверь local.properties — там должен быть путь к Android SDK
cat local.properties
# sdk.dir=/path/to/android-sdk

./gradlew assembleDebug
```

### Текущая сборка + установка

```bash
./gradlew assembleDebug
adb install --user 0 -r app/build/outputs/apk/debug/app-debug.apk
```

`--user 0` обязательно — иначе APK встанет в рабочий профиль (если он есть).

## Правила разработки

### UI — ТОЛЬКО Material Design 3

Всё UI строго на Material Design 3 с Dynamic Colors. Никаких старых виджетов:
- **Кнопки:** `MaterialButton` (не `Button`)
- **Карточки:** `MaterialCardView` (не `CardView`)
- **Диалоги:** `MaterialAlertDialogBuilder` (не `AlertDialog.Builder`)
- **Переключатели:** `MaterialSwitch`, `MaterialCheckBox`, `MaterialButtonToggleGroup`
- **Прогресс:** `LinearProgressIndicator` / `CircularProgressIndicator`
- **Текст:** `TextInputLayout` + `TextInputEditText`
- **Списки:** `RecyclerView`
- **Цвета:** только `?attr/` токены, никогда `#RRGGBB`
- **Типография:** M3 type scale (`?attr/textAppearanceTitleSmall` и т.д.)

Тема: `Theme.Material3.DayNight.NoActionBar`. `DynamicColors.applyToActivityIfAvailable()` вызывать в `onCreate()` каждой Activity до `setContentView()`.

### Язык интерфейса — ТОЛЬКО русский

Никаких `values-en/`. Все пользовательские строки — в `values/strings.xml` по-русски. Комментарии в коде и документация — по-английски.

### Инкрементальные изменения

Не заменять целые экраны за один раз. Только маленькие инкрементальные добавления. Старую работающую функциональность не удалять.

### Сборка — всегда debug во время разработки

Debug даёт логи в logcat, быстрее собирается. Release — только для финальной публикации в RuStore.

## Ключевые технические детали

- **minSdk:** 26 (Android 8.0)
- **targetSdk:** 35
- **Архитектура:** только `arm64-v8a`
- **Модель GigaAM v3:** ~312 МБ, **вшита в APK** (`app/src/main/assets/models/gigaam-v3/`). Ничего не качается — ни при установке, ни при первом запуске. Разрешение `INTERNET` в манифесте не заявлено. Файлы были сконвертированы под sherpa-onnx из официальных исходников `github.com/salute-developers/GigaAM` (MIT)
- **Silero VAD:** 629 КБ, в `app/src/main/assets/silero_vad.onnx`
- **sherpa-onnx.aar:** 47 МБ, gitignored, качается скриптом
- **Accessibility config:** `flagInputMethodEditor` обязателен для `commitText` на Android 13+
- **Логирование:** `AppLog.log(context, message)` — файловый журнал. Не использовать `Toast.makeText()`
- **SharedPreferences:** файл `govorun_lite_prefs` (не `govorun_prefs` — это Pro)

## RuStore-требования (учесть заранее)

- Нет зависимостей от Google Play Services (Firebase/Analytics/и т.п. не добавлять)
- Политика конфиденциальности обязательна даже для офлайн-приложения
- Возрастной рейтинг и описание на русском
- Минимум 5 скриншотов, один — с работающим баблом
- Signing key отдельный от Pro (см. `keystore.properties.template` когда появится)

## Наименование

- Маркетинговое: **Говорун** (в лаунчере), **Говорун: Голосовой ввод на базе Giga** (полное название)
- Пакет: `com.govorun.lite`
- В коде и ресурсах: `GovorunLite` (темы, классы)
