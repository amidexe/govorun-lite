//! One presentation of measured stages; nested server samples are not added twice.
use super::{support, timing::Fields};
fn number(f: &Fields<'_>, key: &str) -> u64 { f.get(key).and_then(|v| v.parse().ok()).unwrap_or(0) }
pub(crate) fn local_only(f: &Fields<'_>) -> bool {
    f.contains_key("full_ms") && (f["result"] == "success" || f["result"] == "local_error" && f["phase"] == "local")
        && number(f,"self_hosted") == 0
        && ["starts", "polls", "start_ms", "wait_ms", "transport_errors", "transport_timeouts", "http_errors"]
            .iter().all(|key| number(f, key) == 0)
}
pub(crate) fn compact(f: &Fields<'_>) -> Option<String> {
    f.get("full_ms")?;
    let online = number(f, "starts") > 0 || number(f,"self_hosted") == 1;
    let processing = number(f, "processing_ms");
    let device = number(f, "full_ms") - if online { processing } else { 0 };
    Some(if online {format!("На устройстве {device} мс · онлайн {processing} мс")}
        else {format!("На устройстве {device} мс")})
}
pub(crate) fn summary(f: &Fields<'_>) -> String {
    let result=f["result"];
    let title=if result=="cancelled" {"Стилизация отменена"} else {"Распознавание завершено"};
    let code=support::result(result);
    // Older desktop traces measure only processing. Never pass that value off
    // as the full duration of dictation.
    let duration=f.get("full_ms").map(|ms|format!("{ms} мс"))
        .unwrap_or_else(||format!("улучшение {} мс",f["elapsed_ms"]));
    [Some(title),code,code.map(|_|support::label(f["phase"])),Some(&duration)].into_iter().flatten().collect::<Vec<_>>().join(" · ")
}
pub(crate) fn skipped(reason: &str) -> Option<&'static str> {
    match reason {
        "stylization_skipped_short" | "Стилизация пропущена: короткий текст." => Some("Короткая фраза."),
        "stylization_skipped_long" | "Стилизация пропущена: текст превышает защитный лимит." => Some("Текст превышает защитный лимит."),
        _ => None,
    }
}
fn part(lines: &mut Vec<String>, label: &str, ms: u64) {
    if ms > 0 { lines.push(format!("  {label} — {ms} мс")); }
}
fn wire(lines: &mut Vec<String>, f: &Fields<'_>, ms: u64) {
    let server = f.get("server_elapsed_ms").map(|_| number(f, "server_elapsed_ms"));
    let model = f.get("server_forward_ms").map(|_| number(f, "server_forward_ms"));
    match (server, model) {
        (Some(server), Some(model)) if f.get("server_sample") == Some(&"completed") && model <= server && server <= ms => {
            part(lines, "Обмен вне сервера", ms - server);
            part(lines, "Шлюз и нейросеть", model);
            let rest = ["server_admission_ms", "server_queue_ms", "server_finalize_ms"];
            if rest.iter().all(|key| f.contains_key(key)) && rest.iter().map(|key| number(f, key)).sum::<u64>() == server - model {
                for (key, label) in rest.into_iter().zip(["Допуск сервера", "Очередь сервера", "Завершение сервера"]) {
                    lines.push(format!("  {label} — {} мс", number(f, key)));
                }
            } else { part(lines, "Остальные этапы сервера", server - model); }
        }
        _ => { part(lines, "Запрос", ms); }
    }
}
fn processing(lines: &mut Vec<String>, f: &Fields<'_>, total: u64) {
    if number(f,"self_hosted") == 1 {return;}
    let local = number(f, "local_ms");
    let exchange = number(f, "start_ms") + number(f, "wait_ms");
    let finalize = number(f, "finalize_ms");
    let measured = local + exchange + finalize;
    if measured > total {
        lines.push("  Подэтапы не согласованы с общим замером.".into());
        return;
    }
    part(lines, "Подготовка запроса", local);
    wire(lines, f, exchange);
    part(lines, "Завершение", finalize);
    part(lines, "Обработка вокруг вызова", total - measured);
}
fn technical(lines: &mut Vec<String>, f: &Fields<'_>) {
    if number(f,"self_hosted") == 1 {
        if let Some(status)=f.get("last_http") {lines.push(format!("  HTTP {status}."));}
        if f["result"] != "success" {lines.push(format!("  {}. Использован исходный текст.",support::label(f["result"])));}
        return;
    }
    if number(f, "deadline_ms") == 0 && number(f, "starts") == 0 && f.get("result") == Some(&"success") { return; }
    let get = |key: &str| f.get(key).copied().unwrap_or("—");
    lines.push(format!("  Сеть. Попыток {}; запросов статуса {}. HTTP {}.", get("starts"), get("polls"), get("last_http")));
    if number(f, "polls") > 0 && f.contains_key("last_response_ms") {
        lines.push(format!("  Последний запрос — {} мс.", get("last_response_ms")));
    }
    if f.contains_key("a2_ms") || f.get("a1_stage").is_some_and(|stage| *stage != "complete") || f.get("a1_result").is_some_and(|result| *result != "success") {
        for i in [1, 2] {
            let k = |suffix| format!("a{i}_{suffix}");
            if let Some(ms) = f.get(k("ms").as_str()) {
                let stage = super::lookup(super::timing::STAGES, get(k("stage").as_str())).unwrap_or("сеть");
                lines.push(format!("  Попытка {i}: {ms} мс; {stage}; {}; HTTP {}.", support::label(get(k("result").as_str())), get(k("http").as_str())));
            }
        }
    } else if let Some(http) = f.get("a1_http").filter(|http| Some(*http) != f.get("last_http")) { lines.push(format!("  Запрос принят: HTTP {http}.")); }
    if let Some(stage)=f.get("access_stage") {
        let label = super::lookup(&[("key","получение ключа"),("issue","получение доступа"),("return","возврат доступа"),("retire","обновление резерва"),("allowance","проверка резерва")],stage).unwrap_or(stage);
        let reason = super::lookup(&[("http","ответ сервера"),("timeout","превышено время ожидания"),("connection","не удалось соединиться"),("storage","хранилище недоступно"),("request","запрос отклонён на устройстве"),("response","ответ не удалось проверить"),("bridge","общий модуль недоступен"),("exhausted","резерв доступа исчерпан"),("issuance_capacity_unavailable","серверная выдача доступа недоступна; лимит использования сохранён"),("reserve_not_prepared","резерв доступа не подготовлен"),("local","локальный сбой")],get("access_reason")).unwrap_or("локальный сбой");
        let request = f.get("access_attempts").map(|n|format!("Запрос Accounts: {label}; попыток: {n}")).unwrap_or_else(||format!("Фоновая подготовка: {label}"));
        let http=f.get("access_http").map(|n|format!(" HTTP {n}")).unwrap_or_default();
        lines.push(format!("  {request}; {reason}{http}."));
    }
    if let Some(count) = f.get("on_demand_issues") { lines.push(format!("  Выдач по требованию: {count}.")); }
    let failures = ["transport_errors", "transport_timeouts", "http_errors"];
    if failures.iter().all(|k| number(f, k) == 0) && get("result") == "success" {
        lines.push(format!("  Ошибки. Не зафиксированы. Предел — {} мс.", get("deadline_ms")));
    } else {
        let code = support::result(get("result")).map(|v| format!("Код: {v}. ")).unwrap_or_default();
        lines.push(format!("  Ошибки. {code}{}; Этап: {}. Сеть {}; тайм-ауты {}; HTTP {}. Предел — {} мс.",
            support::label(get("result")),support::label(get("phase")),get("transport_errors"),get("transport_timeouts"),get("http_errors"),get("deadline_ms")));
    }
    if let Some(limit) = f.get("timeout_limit_ms") { lines.push(format!("  Сработавший предел — {limit} мс.")); }
}
fn quantity(n: u64, one: &str, few: &str, many: &str) -> String {
    let word = if (11..=14).contains(&(n%100)) {many} else {match n%10 {1=>one,2..=4=>few,_=>many}};
    format!("{n} {word}")
}
fn recording_facts(lines: &mut Vec<String>, f: &Fields<'_>) {
    let mut facts = Vec::new();
    if let Some(ms) = f.get("audio_ms").and_then(|v| v.parse::<u64>().ok()) {
        let seconds = ms / 1000;
        let mut duration = Vec::new();
        if seconds >= 60 { duration.push(quantity(seconds / 60, "минута", "минуты", "минут")); }
        if seconds % 60 > 0 || duration.is_empty() { duration.push(quantity(seconds % 60, "секунда", "секунды", "секунд")); }
        facts.push(duration.join(" "));
    }
    if f.contains_key("offline_words") { facts.push(quantity(number(f, "offline_words"), "слово", "слова", "слов")); }
    if let Some(input) = f.get("offline_chars") {
        facts.push(match f.get("output_chars").filter(|output| *output != input) {
            Some(output) => format!("{input} → {output} символов"),
            None => quantity(number(f, "offline_chars"), "символ", "символа", "символов"),
        });
    }
    if !facts.is_empty() { lines.push(format!("Запись: {}.", facts.join(" · "))); }
    if let Some(sent) = f.get("sent_chars").filter(|sent| !local_only(f) && Some(*sent) != f.get("offline_chars")) { lines.push(format!("Отправлено на стилизацию: {sent} символов.")); }
    if let Some(cached) = f.get("model_cached") {
        let mut model = format!("Модель: {}", if *cached == "1" { "в памяти" } else { "загружена при записи" });
        if let Some(ms) = f.get("model_load_ms").filter(|ms| **ms != "0") { model += &format!("; загрузка {ms} мс во время записи"); }
        lines.push(model + ".");
    }
}
pub(crate) fn report(f: &Fields<'_>) -> String {
    let mut lines = Vec::new();
    let complete=f.contains_key("full_ms");
    let local=local_only(f);
    if complete {
        let preparation=number(f,"delivery_prep_ms") + if local {number(f,"processing_ms")} else {0};
        let recognition=number(f,"tail_ms")+number(f,"recognition_ms")+preparation;
        lines.push(format!("1. Распознавание — {recognition} мс"));
        for (key,label) in [("recognition_ms","Офлайн-модель"),("tail_ms","Завершение записи")] {part(&mut lines,label,number(f,key));}
        part(&mut lines,"Подготовка",preparation);
    } else {lines.push("1. Распознавание — нет замера".into());}
    if !local {
        let total=number(f,if complete {"processing_ms"} else {"elapsed_ms"});
        lines.push(format!("2. Стилизация — {total} мс"));
        processing(&mut lines,f,total);technical(&mut lines,f);
    }
    let insertion=if local {2} else {3};
    lines.push(f.get("insertion_ms").map(|ms|format!("{insertion}. Вставка — {ms} мс")).unwrap_or_else(||format!("{insertion}. Вставка — нет замера")));
    if complete {recording_facts(&mut lines,f);}
    lines.join("\n")
}
