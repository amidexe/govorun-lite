<script lang="ts">
  import { hotkeySnapshot } from './model';
  import { statisticsSnapshot } from '../statistics/model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  const hotkeys = useNativeSnapshot('hotkeys.snapshot', hotkeySnapshot, 'hotkeys.changed');
  const stats = useNativeSnapshot('statistics.snapshot', statisticsSnapshot, 'statistics.changed');
  const key = 'govorun.hotkeyHintClosed';
  // Per-device convenience: a blocked store only brings the card back until the first dictation.
  let closed = $state(read());
  function read() {
    try {
      return localStorage.getItem(key) === '1';
    } catch {
      return false;
    }
  }
  function close() {
    closed = true;
    try {
      localStorage.setItem(key, '1');
    } catch {
      // The card stays hidden for this run.
    }
  }
  let keys = $derived(hotkeys.value?.dictation ?? []);
  let visible = $derived(!closed && keys.length > 0 && stats.value?.total.sessions === 0);
</script>

{#if visible}
  <section class="surface hint" aria-label="Подсказка">
    <Icon name="ph_microphone" />
    <p>
      Диктовка — {#each keys as token}<kbd>{token}</kbd>{/each}. Нажмите в любом поле ввода и
      говорите. Сочетание можно изменить в настройках.
    </p>
    <button class="quiet icon-button" aria-label="Скрыть подсказку" onclick={close}
      ><Icon name="ph_x" size={16} /></button
    >
  </section>
{/if}

<style>
  .hint {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  p {
    flex: 1;
    margin: 0;
    color: var(--secondary);
    line-height: 1.5;
  }
  kbd {
    font: inherit;
    font-weight: 600;
    color: var(--text, inherit);
    margin: 0 2px;
  }
</style>
