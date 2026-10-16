import type { StylesSnapshot } from './model';

// The core provides the state. This module only formats translated labels.
const exhaustedReasons: Record<string, string | null> = { none: null, period: 'Лимит закончился', daily: 'Лимит на сегодня закончился' };

export function stylePresentation(value: StylesSnapshot) {
  const limit = exhaustedReasons[value.limitReason];
  const preparing = value.displayState === 'preparing';
  const active = value.displayState === 'active';
  return {
    limit,
    active,
    preparing,
    saved: value.displayState === 'saved' || value.displayState === 'preparing' || value.displayState === 'checking',
    summary: preparing ? 'Без улучшения' : active
      ? value.title +
        (value.gate === 'ready' && value.options.emojiMode !== 'off' ? ' · с эмодзи' : '')
      : 'Без стилизации',
  };
}

export function styleStatus(value: StylesSnapshot): string {
  if (value.displayState === 'disabled') return 'Улучшение текста выключено';
  if (value.displayState === 'checking') return 'Проверяем состояние улучшения текста · выбор сохранён';
  if (value.displayState === 'preparing')
    return 'Улучшение подключится автоматически · выбор сохранён';
  if (value.displayState === 'saved')
    return 'Улучшение текста временно недоступно · выбор сохранён';
  return `Улучшение текста включено · ${value.title}`;
}
