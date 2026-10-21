//! Closed capture failures; OS messages, device names and audio are never stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation { Format, Open, Start }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause { Unavailable, Unsupported, InvalidArgument, Backend, Other }
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Native {
    pub operation: Operation,
    pub cause: Cause,
    pub os_code: Option<i32>,
}
const CODES: &[(&str, &str, &str)] = &[
    ("microphone_permission_required", "Доступ", "Система не разрешила запись голоса"),
    ("microphone_unavailable", "Открытие", "Микрофон недоступен"),
    ("microphone_device_busy", "Открытие", "Микрофон занят другим приложением"),
    ("microphone_audio_service_unavailable", "Открытие", "Служба звука Windows не запущена"),
    ("microphone_open_failed", "Открытие", "Не удалось открыть аудиопоток"),
    ("microphone_start_failed", "Запуск", "Не удалось запустить аудиопоток"),
    ("microphone_format_unavailable", "Формат", "Не удалось получить формат микрофона"),
    ("microphone_format_unsupported", "Формат", "Формат микрофона не поддерживается"),
    ("microphone_invalid_configuration", "Открытие", "Аудиосистема отклонила параметры потока"),
    ("microphone_callback_failed", "Запись", "Аудиопоток завершился с ошибкой"),
    ("microphone_timeout", "Первый звук", "Микрофон не прислал звук за время ожидания"),
    ("microphone_timestamp_unavailable", "Запись", "Не удалось определить время звука"),
    ("microphone_packet_invalid", "Запись", "Микрофон прислал некорректный блок звука"),
    ("microphone_buffer_overflow", "Запись", "Переполнен буфер звука"),
    ("microphone_enumeration_failed", "Открытие", "Не удалось получить список микрофонов"),
    ("microphone_executor_unavailable", "Запуск", "Служба записи не запустилась"),
    ("microphone_generation_exhausted", "Запуск", "Служба записи исчерпала счётчик попыток"),
    ("recording_cancelled", "Запуск", "Запись отменена до первого звука"),
    ("recording_boundary_invalid", "Запись", "Неверная граница записи"),
    ("recording_completion_invalid", "Запись", "Запись завершена в неверном порядке"),
    ("recording_generation_exhausted", "Запуск", "Исчерпан счётчик записей"),
    ("recording_timer_invalid", "Запуск", "Неверные параметры таймера записи"),
    ("recording_timer_unavailable", "Запуск", "Таймер записи недоступен"),
    ("dictation_prepare_unavailable", "Подготовка", "Распознавание не подготовлено"),
    ("dictation_settings_unavailable", "Подготовка", "Не удалось прочитать настройки диктовки"),
    ("recording_already_active", "Запуск", "Предыдущая запись ещё не завершилась"),
    ("recording_not_active", "Запись", "Запись уже не активна"),
    ("maintenance_busy", "Запуск", "Идёт обслуживание, запись временно недоступна"),
    ("hotkeys_capture_unavailable", "Запуск", "Горячая клавиша недоступна"),
    ("hotkeys_permission_required", "Доступ", "Нет разрешения на горячую клавишу"),
    ("hotkeys_unavailable", "Запуск", "Горячие клавиши не работают"),
    ("recognition_model_unavailable", "Подготовка", "Модель распознавания недоступна"),
    ("recognition_worker_failed", "Обработка", "Сбой службы распознавания"),
    ("recognition_job_consumed", "Обработка", "Задача распознавания уже использована"),
    ("dictation_unavailable", "Запуск", "Диктовка недоступна"),
    ("dictation_stopped", "Запуск", "Диктовка остановлена"),
    ("dictation_invalid_workflow", "Запуск", "Нарушен порядок шагов диктовки"),
    ("dictation_configuration_recovery_failed", "Подготовка", "Не удалось восстановить настройки диктовки"),
    ("dictation_invalid_preparation", "Подготовка", "Неверная подготовка диктовки"),
    ("dictation_preparation_stopped", "Подготовка", "Подготовка диктовки остановлена"),
    ("dictation_invalid_core_result", "Обработка", "Неверный результат обработки диктовки"),
    ("dictation_invalid_delivery", "Обработка", "Неверные данные для вставки текста"),
    ("dictation_invalid_facts", "Обработка", "Неверные данные о диктовке"),
    ("dictation_job_panicked", "Обработка", "Сбой задачи распознавания"),
    ("dictation_queue_exhausted", "Обработка", "Очередь распознавания переполнена"),
    ("dictation_queue_stopped", "Обработка", "Очередь распознавания остановлена"),
    ("dictation_queue_unavailable", "Обработка", "Очередь распознавания недоступна"),
    ("recognition_vad_unavailable", "Подготовка", "Не удалось создать детектор речи"),
    ("recognition_vad_failed", "Запись", "Сбой детектора речи"),
    ("recognition_runtime_unavailable", "Подготовка", "Движок распознавания не загрузился"),
    ("recognition_runtime_invalid", "Подготовка", "Движок распознавания повреждён"),
    ("recognition_runtime_abi_mismatch", "Подготовка", "Движок распознавания несовместим с приложением"),
    ("recognition_runtime_version_mismatch", "Подготовка", "Неверная версия движка распознавания"),
    ("recognition_compute_backend_unavailable", "Подготовка", "Не удалось запустить распознавание на этом устройстве"),
    ("recognition_backend_unavailable", "Подготовка", "Способ распознавания недоступен"),
    ("recognition_backend_invalid", "Подготовка", "Неверный способ распознавания"),
    ("recognition_devices_unavailable", "Подготовка", "Не удалось получить список устройств распознавания"),
    ("recognition_model_path_invalid", "Подготовка", "Неверный путь к модели распознавания"),
    ("recognition_queue_closed", "Обработка", "Очередь распознавания закрыта"),
    ("recognition_failed", "Обработка", "Распознавание завершилось ошибкой"),
    ("model_missing", "Подготовка", "Файлы модели распознавания не найдены"),
    ("initialization_failed", "Подготовка", "Не удалось инициализировать распознавание"),
    ("runtime_incompatible", "Подготовка", "Движок распознавания несовместим с процессором или системой"),
    ("recording_invalid_pcm", "Запись", "Получен повреждённый звук"),
    ("microphone_catalog_invalid", "Открытие", "Неверный список микрофонов"),
    ("microphone_permission_unavailable", "Доступ", "Не удалось проверить разрешение на микрофон"),
    ("dictation_setup_unavailable", "Подготовка", "Диктовка не настроена"),
    ("hotkeys_capture_unfocused", "Запуск", "Горячая клавиша не получила фокус"),
    ("hotkeys_invalid_binding", "Запуск", "Неверная горячая клавиша"),
    ("hotkeys_invalid_capture", "Запуск", "Неверный захват горячей клавиши"),
    ("hotkeys_unsupported_platform", "Запуск", "Горячие клавиши не поддерживаются"),
    ("recording_start_failed", "Запуск", "Не удалось начать запись"),
];
pub fn code(value: &str) -> Option<&'static str> { CODES.iter().find(|row| row.0 == value).map(|row| row.0) }
pub fn encode_detail(value: &str, native: Option<Native>) -> Option<String> {
    let code = code(value)?;
    Some(match native {
        Some(native) => format!("{code} {}", serde_json::to_string(&native).ok()?),
        None => code.into(),
    })
}
fn parse(value: &str) -> Option<(&'static str, Option<Native>)> {
    if value.len() > 256 { return None; }
    match value.split_once(' ') {
        Some((name, native)) => Some((code(name)?, Some(serde_json::from_str(native).ok()?))),
        None => Some((code(value)?, None)),
    }
}
pub fn valid_detail(value: &str) -> bool { parse(value).is_some() }
pub fn stored(value: &str) -> Option<String> { parse(value).map(|_| format!("capture_failure {value}")) }
pub fn from_stored(value: &str) -> Option<&str> {
    let detail = value.strip_prefix("capture_failure ")?;
    valid_detail(detail).then_some(detail)
}
pub fn failure_detail(value: &str) -> Option<String> {
    let (code, native) = parse(value)?;
    let (_, stage, reason) = CODES.iter().find(|row| row.0 == code)?;
    let mut detail = format!("{reason}. Этап: {stage}. Код: {code}.");
    if let Some(native) = native {
        let operation = match native.operation { Operation::Format => "формат", Operation::Open => "открытие", Operation::Start => "запуск" };
        let cause = match native.cause {
            Cause::Unavailable => "устройство недоступно", Cause::Unsupported => "формат не поддерживается",
            Cause::InvalidArgument => "неверные параметры", Cause::Backend => "отказ аудиосистемы", Cause::Other => "внутренняя ошибка",
        };
        detail.push_str(&format!(" CPAL: {operation}, {cause}."));
        if let Some(code) = native.os_code { detail.push_str(&format!(" Код системы: 0x{:08X}.", code as u32)); }
    }
    Some(detail)
}
