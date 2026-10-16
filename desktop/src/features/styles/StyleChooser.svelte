<script lang="ts">
  import type { StylesSnapshot, StylePreset } from './model';
  import { stylePresentation } from './style-presentation';
  import Icon from '../../shared/ui/Icon.svelte';
  let {
    value,
    compact = false,
    busy,
    onselect,
    onedit,
    oncreate,
  }: {
    value: StylesSnapshot;
    compact?: boolean;
    busy: boolean;
    onselect: (selection: string) => void;
    onedit: (preset: StylePreset) => void;
    oncreate: () => void;
  } = $props();
  let selfHosted = $derived(value.gate === 'selfHosted' || value.gate === 'selfHostedSetup');
  let canSelect = $derived(!busy && (value.gate === 'ready' || value.gate === 'terms'));
  let presentation = $derived(stylePresentation(value));
</script>

<div class="choices" class:compact aria-label="Стиль текста">
  <button
    class:chosen={!value.enabled}
    aria-pressed={!value.enabled}
    disabled={busy}
    onclick={() => onselect('off')}
    >Без стилизации{#if !value.enabled}<Icon name="ph_check" size={16} />{/if}</button
  >
  {#if selfHosted}<button
      class:chosen={value.enabled}
      aria-pressed={value.enabled}
      disabled={busy || value.gate !== 'selfHosted'}
      onclick={() => onselect('selfHosted')}
      >Свой сервер{#if value.enabled}<Icon name="ph_check" size={16} />{/if}</button
    >{/if}
  {#each value.styles as style (style.id)}<button
      class:chosen={!selfHosted && presentation.active && value.selected === style.id}
      aria-pressed={!selfHosted && presentation.active && value.selected === style.id}
      disabled={!canSelect}
      onclick={() => onselect(style.id)}
    >
      <span>{style.title}</span
      >{#if !selfHosted && presentation.active && value.selected === style.id}<Icon
          name="ph_check"
          size={16}
        />{:else if !selfHosted && presentation.saved && value.selected === style.id}<span
          class="saved-label">Сохранён</span
        >{/if}
    </button>{/each}
</div>
{#if !compact || value.presets.length}<div class="custom-styles" class:compact>
    <h3>
      Свои стили {#if !compact && value.maxPresets > 0}<span>{value.presets.length} из {value.maxPresets}</span>{/if}
    </h3>
    {#each value.presets as preset (preset.id)}<div class="preset-row">
        <button
          class="preset-choice"
          class:chosen={!selfHosted &&
            presentation.active &&
            value.selected === `custom:${preset.id}`}
          aria-pressed={!selfHosted &&
            presentation.active &&
            value.selected === `custom:${preset.id}`}
          disabled={!canSelect}
          onclick={() => onselect(`custom:${preset.id}`)}
          ><span>{preset.name}</span
          >{#if !selfHosted && presentation.active && value.selected === `custom:${preset.id}`}<Icon
              name="ph_check"
              size={16}
            />{:else if !selfHosted && presentation.saved && value.selected === `custom:${preset.id}`}<span
              class="saved-label">Сохранён</span
            >{/if}</button
        >
        {#if !compact}<button
            class="quiet icon-button"
            aria-label={`Изменить стиль ${preset.name}`}
            disabled={busy || selfHosted}
            onclick={() => onedit(preset)}><Icon name="ph_pencil_simple" size={16} /></button
          >{/if}
      </div>{/each}
    {#if !compact}<button
        class="quiet create"
        disabled={busy || !value.canCreatePreset}
        onclick={oncreate}><Icon name="ph_plus" size={16} />Создать свой стиль</button
      >{/if}
  </div>{/if}

<style>
  .choices {
    display: grid;
    gap: 2px;
  }
  .choices > button,
  .preset-choice {
    width: 100%;
    justify-content: space-between;
    text-align: left;
    min-height: 36px;
    background: transparent;
    border-color: transparent;
  }
  button.chosen {
    background: var(--accent-soft);
    color: var(--accent-text);
    font-weight: 600;
  }
  .custom-styles {
    border-top: 1px solid var(--stroke);
    margin-top: 12px;
    padding-top: 12px;
  }
  h3 {
    font-size: 12px;
    margin: 0 0 6px;
  }
  h3 span {
    color: var(--secondary);
    font-weight: 400;
    margin-left: 6px;
  }
  .preset-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .create {
    margin-top: 6px;
    color: var(--accent-text);
  }
  .compact h3 {
    padding: 0 12px;
    color: var(--secondary);
    font-weight: 500;
  }
  .compact.custom-styles {
    margin-top: 8px;
    padding-top: 8px;
  }
  .compact button:disabled {
    opacity: 1;
    color: var(--secondary);
  }
  .saved-label {
    font-size: 11px;
    font-weight: 400;
    color: var(--secondary);
  }
</style>
