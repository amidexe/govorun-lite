import type { DayUsage, StatisticsSnapshot } from './model.ts';

export function summarizeStatistics(snapshot: StatisticsSnapshot) {
  // These are calendar keys produced by the native store, not elapsed 24-hour periods.
  const reference = Date.parse(`${snapshot.today}T12:00:00Z`);
  const byDate = new Map(snapshot.days.map((day) => [day.date, day]));
  const days: DayUsage[] = Array.from({ length: 14 }, (_, index) => {
    const date = new Date(reference + (index - 13) * 86_400_000).toISOString().slice(0, 10);
    return byDate.get(date) ?? { date, words: 0, seconds: 0, sessions: 0 };
  });
  const week = days.slice(7);
  return {
    days: week,
    today: days[13],
    words: week.reduce((sum, day) => sum + day.words, 0),
    seconds: week.reduce((sum, day) => sum + day.seconds, 0),
    previousWords: days.slice(0, 7).reduce((sum, day) => sum + day.words, 0),
  };
}

export function weekComparison(current: number, previous: number): string {
  if (previous === 0)
    return current === 0 ? 'Пока нет данных для сравнения' : 'В предыдущие 7 дней — 0 слов';
  if (current === previous) return 'Столько же, сколько за предыдущие 7 дней';
  const percent = Math.max(1, Math.round(Math.abs(current / previous - 1) * 100));
  return `На ${percent}% ${current > previous ? 'больше' : 'меньше'}, чем за предыдущие 7 дней`;
}

export function words(value: number): string {
  if (value < 1_000) {
    const form =
      value % 10 === 1 && value % 100 !== 11
        ? 'слово'
        : value % 10 >= 2 && value % 10 <= 4 && !(value % 100 >= 12 && value % 100 <= 14)
          ? 'слова'
          : 'слов';
    return `${value} ${form}`;
  }
  if (value < 1_000_000) {
    const number =
      value >= 10_000 || value % 1_000 === 0
        ? Math.floor(value / 1_000)
        : (value / 1_000).toFixed(1).replace('.', ',');
    return `${number} тыс. слов`;
  }
  return `${(value / 1_000_000).toFixed(1).replace('.', ',')} млн слов`;
}

export function duration(seconds: number): string {
  if (seconds < 60) return `${seconds} сек`;
  if (seconds < 3_600) return `${Math.floor(seconds / 60)} мин`;
  const hours = Math.floor(seconds / 3_600),
    minutes = Math.floor((seconds % 3_600) / 60);
  return minutes ? `${hours} ч ${minutes} мин` : `${hours} ч`;
}

export function statisticsScope(snapshot: StatisticsSnapshot): string {
  if (!snapshot.signedIn) return 'Данные хранятся на этом компьютере и доступны без интернета.';
  if (snapshot.syncMode === 'pending')
    return 'Пока показаны данные этого компьютера. Выберите, объединять ли их с аккаунтом.';
  if (snapshot.syncMode === 'localOnly')
    return 'Синхронизация выключена. Показаны данные этого компьютера.';
  return snapshot.hasCloudSnapshot
    ? 'Данные устройств обновляются автоматически.'
    : 'Пока показаны данные этого компьютера; данные аккаунта появятся после синхронизации.';
}
