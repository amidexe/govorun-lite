use super::{Facts, Measurement, STAGES};
const LABELS: &[&str] = &[
    "Примеры и контрольные суммы",
    "Загрузка модели",
    "Первое распознавание",
    "Повторное распознавание",
    "Длинная запись",
];
pub(super) fn suite(facts: &Facts) -> String {
    let passed = facts.devices.iter().filter(|device| device.state == "done").count();
    let result = if facts.devices.is_empty() || facts.state == "running" {
        facts.state.clone()
    } else {
        format!("{} · прошли проверку: {passed} из {}", facts.state, facts.devices.len())
    };
    let mut text=format!("Говорун — диагностический отчёт v2\n{}\nРезультат: {}\nВсе доступные устройства проверяются по очереди на одинаковых встроенных записях.\nНабор: gigaam-samples-v1 · 11290 / 71250 мс\nСкорость каждого устройства рассчитана по повторному и длинному замерам.\n",facts.system.render(),result);
    for device in &facts.devices {
        text += &format!(
            "\n{} · {} · {}\n",
            device.name,
            device.backend.to_uppercase(),
            device.state
        );
        let current = STAGES
            .iter()
            .position(|stage| *stage == device.stage)
            .unwrap_or(0);
        text += &LABELS
            .iter()
            .enumerate()
            .map(|(index, label)| {
                let status =
                    if device.state == "done" || device.state != "pending" && index < current {
                        "OK"
                    } else if index == current {
                        match device.state.as_str() {
                            "failed" => "ОШИБКА",
                            "running" => "выполняется",
                            "cancelled" => "отменено",
                            _ => "не проверено",
                        }
                    } else {
                        "не проверено"
                    };
                format!("{label}: {status}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        if let Some(result) = &device.result {
            text += "\n";
            text += &measurement(result);
        }
        if let Some(failure) = &device.failure_code {
            text += &format!("\nКод: {failure} · этап: {}", device.stage);
            if let Some(detail) = &device.failure_detail {
                text += &format!(" · причина: {detail}");
            }
        }
        text += "\n";
    }
    if facts.devices.is_empty() {
        text += &facts
            .failure_code
            .as_ref()
            .map(|code| format!("\nКод: {code} · этап: {}", facts.stage))
            .unwrap_or_else(|| "\nУстройства ещё не проверены.".into());
    } else if facts.devices.iter().all(|device| device.backend == "cpu") {
        text += &format!(
            "\nДоступная через {} видеокарта не найдена.",
            facts.system.gpu_api()
        );
    }
    text+="\nGPU проверяется без перехода на CPU. Неудачный тест не мешает проверке остальных устройств.\nМикрофон, горячие клавиши, вставка и онлайн-обработка не тестировались. Текст записей в отчёт не включён.\nУспешное распознавание подтверждает эту нагрузку, а не исправность всех драйверов.";
    text
}
pub(super) fn measurement(result: &Measurement) -> String {
    let speed = (result.short_audio_ms + result.long_audio_ms)
        / (result.warm.milliseconds + result.longer.milliseconds);
    let numeric = |value: f64| {
        if result.platform == "macos" {
            format!("{:.0}", value.trunc())
        } else {
            format!("{value:.0}")
        }
    };
    let platform = if result.platform == "macos" {
        "macOS"
    } else {
        "Windows"
    };
    let model = if result.platform == "macos" {
        "GigaAM v3 E2E RNNT · GGUF Q8_0 · transcribe.cpp 0.2.1"
    } else {
        &result.model
    };
    format!("Говорун — тест скорости · {}\n{platform} · {model}\nРежим: {}\nСкорость: ×{speed:.1}\nПервый запуск: {} мс ({})\nЗагрузка модели: {} мс\nПовторный: {} мс ({})\nДлинный: {} мс ({})\nАудио: {} / {} мс\nМикрофон не использовался. Только встроенные примеры.",result.suite,result.requested_backend,
        numeric(result.cold.milliseconds),result.cold.backend,numeric(result.load_ms),numeric(result.warm.milliseconds),result.warm.backend,
        numeric(result.longer.milliseconds),result.longer.backend,numeric(result.short_audio_ms),numeric(result.long_audio_ms))
}
