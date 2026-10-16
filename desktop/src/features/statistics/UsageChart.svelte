<script lang="ts">
  import type { DayUsage } from './model.ts';
  import { words } from './presentation';
  let { days, compact = false }: { days: DayUsage[]; compact?: boolean } = $props();
  const weekdays = new Intl.DateTimeFormat('ru', { weekday: 'short', timeZone: 'UTC' });
  const id = $props.id();
  let maximum = $derived(Math.max(1, ...days.map((day) => day.words)));
  let points = $derived(
    days.map((day, index) => ({
      ...day,
      x: 12 + index * 96,
      y: 154 - (day.words / maximum) * 132,
    })),
  );
  let line = $derived(points.map((point) => `${point.x},${point.y}`).join(' '));
</script>

<div class:compact>
  <svg viewBox="0 0 600 190" role="img" aria-label="Слова по дням за последние 7 дней">
    <defs
      ><linearGradient {id} x1="0" y1="0" x2="0" y2="1"
        ><stop stop-color="var(--accent)" stop-opacity="0.22" /><stop
          offset="1"
          stop-color="var(--accent)"
          stop-opacity="0.02"
        /></linearGradient
      ></defs
    >
    <path d={`M12 154 L${line.replaceAll(' ', ' L')} L588 154 Z`} fill={`url(#${id})`} />
    <polyline
      points={line}
      fill="none"
      stroke="var(--accent)"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    {#each points as point}
      <circle cx={point.x} cy={point.y} r={point.words ? 3.5 : 2.5} fill="var(--accent)"
        ><title>{point.date}: {words(point.words)}</title></circle
      >
      <text x={point.x} y="181" text-anchor="middle"
        >{weekdays.format(new Date(`${point.date}T12:00:00Z`))}</text
      >
    {/each}
  </svg>
  <table class="sr-only">
    <caption>Слова по дням</caption><tbody
      >{#each days as day}<tr><th>{day.date}</th><td>{day.words}</td></tr>{/each}</tbody
    >
  </table>
</div>

<style>
  svg {
    display: block;
    width: 100%;
    max-height: 210px;
    overflow: visible;
  }
  text {
    fill: var(--secondary);
    font-size: 12px;
    font-family: inherit;
  }
  .compact svg {
    max-height: 120px;
  }
</style>
