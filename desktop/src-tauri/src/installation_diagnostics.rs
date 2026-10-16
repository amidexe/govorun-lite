use govorun_core::operational_trace::OperationalTrace;
use serde_json::Value;

// Import only the installer's closed numeric receipt, never arbitrary files or OS logs.
pub fn current() -> Option<Value> {
    #[cfg(windows)] {
        use std::{fs, io::Read, time::UNIX_EPOCH};
        // Development/acceptance profiles must not read or clear the owner's installer receipt.
        if cfg!(debug_assertions) { return None; }
        let path = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("Govorun/install-diagnostic.txt");
        let metadata = fs::metadata(&path).ok()?;
        if metadata.len() > 512 { return None; }
        let at = i64::try_from(metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()?;
        let mut text = String::new();
        fs::File::open(path).ok()?.take(513).read_to_string(&mut text).ok()?;
        return parse(&text, at);
    }
    #[cfg(not(windows))] { None }
}

pub fn clear() -> Result<(), &'static str> {
    #[cfg(windows)] {
        if cfg!(debug_assertions) { return Ok(()); }
        let path = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("diagnostics_save_failed")?)
            .join("Govorun/install-diagnostic.txt");
        match std::fs::remove_file(path) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err("diagnostics_save_failed"),
        }
    }
    Ok(())
}

#[cfg_attr(not(windows), allow(dead_code))]
fn parse(text: &str, at: i64) -> Option<Value> {
    if text.len() > 512 { return None; }
    let mut lines = text.lines();
    let version = lines.next()?.strip_prefix("Govorun ")?;
    if version.is_empty() || version.len() > 32 || !version.bytes().all(|c| c.is_ascii_digit() || c == b'.') { return None; }
    let exit = lines.next()?.strip_prefix("exit_code=")?.parse::<u32>().ok()?;
    let stage = lines.next()?.strip_prefix("stage_code=")?.parse::<u32>().ok()?;
    let code = lines.next()?.strip_prefix("win32_code=")?.parse::<u32>().ok()?;
    if lines.next().is_some() || exit == 0 && stage == 0 && code == 0 { return None; }
    let stage_name = match stage {
        13 => "Распаковка файлов", 15 => "Создание временной папки",
        14 => "Замена файлов приложения", _ => "Установка приложения",
    };
    let reason = match code {
        39 | 112 => "Недостаточно места на диске", 8 | 14 | 1455 => "Недостаточно памяти или системных ресурсов",
        5 => "Windows отказала в доступе", 32 | 33 => "Файлы заняты другим процессом",
        126 => "Не найдена библиотека или одна из её зависимостей", 193 | 216 => "Несовместимый исполняемый файл",
        _ => "Точная причина не определена; сохранены коды установщика",
    };
    let mut row = OperationalTrace::new("runtime.connect", Some("installation_failed"), at, 0)?.row(0)?;
    row["category"] = "Установка".into(); row["summary"] = "Установка приложения завершилась ошибкой".into();
    row["details"] = format!("Версия: {version}\nЭтап: {stage_name}\n{reason}\nКод завершения: {exit}\nКод этапа: {stage}\nКод Windows: {code}").into();
    Some(row)
}
