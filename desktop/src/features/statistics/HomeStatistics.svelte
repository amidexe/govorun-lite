<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { statisticsSnapshot } from './model.ts';
  import { summarizeStatistics, words, statisticsScope } from './presentation';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import { useAccount } from '../account/state.svelte';
  import AccountDataConnection from '../account/AccountDataConnection.svelte';
  const account = useAccount();
  let mergePending = $derived(
    account.account.value?.status === 'signedIn' && account.account.value.syncMode === 'pending',
  );
  import Statistics from './Statistics.svelte';
  import { onMount } from 'svelte';
  import { slide } from 'svelte/transition';
  import { motionDuration } from '../../motion';
  const api = useDesktopApi();
  const stats = useNativeSnapshot('statistics.snapshot', statisticsSnapshot, 'statistics.changed');
  let expanded = $state(new URLSearchParams(location.search).get('route') === 'stats');
  onMount(() =>
    api.subscribe((event) => {
      if (
        event.event === 'navigation' &&
        event.data &&
        typeof event.data === 'object' &&
        'route' in event.data &&
        event.data.route === 'stats'
      )
        expanded = true;
    }),
  );
  let week = $derived(stats.value ? summarizeStatistics(stats.value).words : null);
</script>

<section class="surface home-statistics">
  <button
    class="expander"
    aria-expanded={expanded}
    aria-controls="home-statistics"
    onclick={() => {
      expanded = !expanded;
    }}
  >
    <span class:expanded class="chevron"><Icon name="ph_caret_right" size={14} /></span><Icon
      name="ph_chart_line"
    />
    <strong>Статистика{week !== null ? ` · ${words(week)} за 7 дней` : ''}</strong>
  </button>
  {#if mergePending}<div class="merge">
      <AccountDataConnection
        busy={account.pending || account.account.failed || account.account.value?.busy === true}
        act={account.act}
      />
    </div>{/if}
  {#if expanded}<div
      id="home-statistics"
      class="expanded-content"
      transition:slide={{ duration: motionDuration() }}
    >
      {#if stats.failed}<p role="alert">
          Не удалось обновить статистику. <button onclick={stats.refresh}>Повторить</button>
        </p>{/if}
      {#if stats.value}<Statistics snapshot={stats.value} compact />
        <div class="statistics-footer"><p>{statisticsScope(stats.value)}</p></div>
      {:else if stats.loading}<p>Загружаем статистику…</p>{/if}
    </div>{/if}
</section>

<style>
  .merge {
    padding: 14px;
    border-top: 1px solid var(--stroke);
  }
  .statistics-footer {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--stroke);
  }
  .statistics-footer p {
    color: var(--secondary);
    font-size: 12px;
  }
  .home-statistics {
    padding: 0;
    overflow: hidden;
  }
  .expanded-content {
    padding: 14px;
    border-top: 1px solid var(--stroke);
  }
  .chevron {
    font-size: 21px;
    line-height: 1;
    transition: transform 0.12s;
  }
  .expanded {
    transform: rotate(90deg);
  }
</style>
