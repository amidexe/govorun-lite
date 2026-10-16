import type { RuntimeSnapshot } from './runtime';

export function runtimeStatus(snapshot: RuntimeSnapshot | null) {
  if (!snapshot) return { title: 'Запускаем Говорун…', detail: '', retry: false };
  if (snapshot.phase === 'starting')
    return { title: 'Подготовка микрофона…', detail: 'Дождитесь начала записи', retry: false };
  if (snapshot.phase === 'recording') return { title: 'Идёт запись', detail: '', retry: false };
  if (snapshot.phase === 'processing')
    return { title: 'Обрабатываем диктовку…', detail: 'Готовим текст для вставки', retry: false };
  if (snapshot.issue === 'captureUnavailable')
    return { title: 'Не удалось начать запись', detail: '', retry: false };
  if (snapshot.phase === 'fallback')
    return {
      title: 'Последняя диктовка — без улучшения',
      detail: 'Сохранён распознанный текст',
      retry: false,
    };
  const ready = snapshot.readiness;
  if (!ready) return { title: 'Проверяем готовность…', detail: '', retry: false };
  if (ready.microphoneAvailable === false)
    return {
      title: 'Микрофон не подключён',
      detail: 'Подключите микрофон или включите его в настройках звука системы.',
      retry: false,
    };
  if (!ready.microphone)
    return {
      title: 'Нужен доступ к микрофону',
      detail: 'Разрешите Говоруну записывать вашу речь.',
      retry: false,
    };
  if (!ready.accessibility)
    return {
      title: 'Нужен доступ к вставке текста',
      detail: 'Разрешите Говоруну доступ в «Универсальном доступе».',
      retry: false,
    };
  if (ready.model === 'failed')
    return { title: 'Не удалось включить диктовку', detail: 'Попробуйте ещё раз.', retry: true };
  if (!['ready', 'standby'].includes(ready.model))
    return { title: 'Подготавливаем диктовку…', detail: '', retry: false };
  if (!ready.hotkey)
    return {
      title: 'Горячая клавиша недоступна',
      detail: 'Проверьте доступ в специальных возможностях',
      retry: false,
    };
  return { title: 'Всё готово', detail: 'Говорун работает', retry: false };
}
