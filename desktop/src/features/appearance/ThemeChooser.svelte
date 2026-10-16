<script lang="ts">
  import { useAppearance, type ThemeMode } from './state.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  const appearance = useAppearance();
  const choices: { mode: ThemeMode; title: string; icon: string }[] = [
    { mode: 'system', title: 'Как в системе', icon: 'ph_desktop' },
    { mode: 'light', title: 'Светлая', icon: 'ph_sun' },
    { mode: 'dark', title: 'Тёмная', icon: 'ph_moon' },
  ];
</script>

<div class="theme-chooser" role="group" aria-label="Оформление">
  {#each choices as choice}<button
      class:chosen={appearance.mode === choice.mode}
      aria-pressed={appearance.mode === choice.mode}
      aria-label={choice.title}
      title={choice.title}
      disabled={appearance.pending}
      onclick={() => appearance.set(choice.mode)}><Icon name={choice.icon} size={17} /></button
    >{/each}
</div>
{#if appearance.error}<p class="error" role="alert">{appearance.error}</p>{/if}

<style>
  .theme-chooser {
    display: flex;
    gap: 3px;
    background: var(--field);
    border: 1px solid var(--stroke);
    border-radius: 10px;
    padding: 3px;
  }
  button {
    flex: 1;
    min-height: 30px;
    padding: 5px;
    background: transparent;
    border-color: transparent;
  }
  button.chosen {
    background: var(--surface);
    color: var(--accent-text);
    box-shadow: var(--control-shadow);
  }
</style>
