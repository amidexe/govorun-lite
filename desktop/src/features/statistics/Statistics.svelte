<script lang="ts">
  import type { StatisticsSnapshot } from './model.ts';
  import { summarizeStatistics, weekComparison, words, duration } from './presentation';
  import UsageChart from './UsageChart.svelte';
  let { snapshot, compact = false }: { snapshot: StatisticsSnapshot; compact?: boolean } = $props();
  let summary = $derived(summarizeStatistics(snapshot));
</script>

<div class="statistics" class:compact>
  {#if snapshot.storageReadOnly}<p role="alert">Не удалось прочитать журнал статистики. Сохранённые счётчики показаны без изменений; новые записи и синхронизация пока недоступны.</p>
  {:else if snapshot.storageRecovered}<p role="status">Не удалось восстановить прежнюю статистику. Учёт начат заново; приложение готово к работе.</p>{/if}
  <div class="metrics">
    <div>
      <p>За 7 дней</p>
      <strong>{words(summary.words)}</strong>
    </div>
    <div class="today">
      <p>Сегодня</p>
      <strong>{words(summary.today.words)}</strong>
    </div>
  </div>
  <p class="comparison">{weekComparison(summary.words, summary.previousWords)}</p>
  <div class="details">
    <p>Время речи за 7 дней: <span>{duration(summary.seconds)}</span></p>
    <p>
      За всё время: <span
        >{words(snapshot.total.words)} · {duration(snapshot.total.seconds)} речи</span
      >
    </p>
  </div>
  <div class="chart">
    <p>Слова по дням <span>· Последние 7 дней, включая сегодня</span></p>
    {#if summary.words || !compact}<UsageChart days={summary.days} {compact} />
    {:else}<p class="empty">График появится после первой диктовки</p>{/if}
  </div>
</div>

<style>
  .metrics {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }
  .today {
    text-align: right;
  }
  .metrics strong {
    display: block;
    margin-top: 3px;
    font-size: 25px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .comparison {
    margin-top: 12px;
  }
  .details {
    margin: 12px 0;
    display: grid;
    gap: 4px;
  }
  .details span {
    color: var(--text);
  }
  .chart {
    border-top: 1px solid var(--stroke);
    padding-top: 12px;
  }
  .chart > p {
    margin-bottom: 12px;
  }
  .chart span {
    font-size: 11px;
  }
  .compact .metrics strong {
    font-size: 24px;
    font-weight: 500;
  }
  .compact .chart {
    border: 0;
    padding-top: 0;
  }
  .compact p {
    font-size: 11px;
  }
  .empty {
    padding: 12px 0;
  }
</style>
