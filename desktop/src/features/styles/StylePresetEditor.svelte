<script lang="ts">
  import type { StylePreset, TextStyleOptions } from './model';
  import StyleOptions from './StyleOptions.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import { untrack } from 'svelte';
  let {
    preset,
    busy,
    error,
    onclose,
    onsave,
    ondelete,
  }: {
    preset: StylePreset | null;
    busy: boolean;
    error: string | null;
    onclose: () => void;
    onsave: (name: string, instruction: string, options: TextStyleOptions) => Promise<void>;
    ondelete: () => Promise<void>;
  } = $props();
  let name = $state(untrack(() => preset?.name ?? '')),
    instruction = $state(untrack(() => preset?.instruction ?? '')),
    deleting = $state(false);
  let options = $state<TextStyleOptions>(
    untrack(() => ({
      emojiMode: 'off',
      reducedPunctuation: false,
      profanityFilter: false,
      ...preset?.options,
    })),
  );
  let valid = $derived(
    name.trim().length > 0 &&
      [...name].length <= 24 &&
      [...instruction.trim()].length >= 3 &&
      [...instruction.trim()].length <= 200,
  );
</script>

<Modal
  title={deleting ? `Удалить стиль «${name}»?` : preset ? 'Изменить свой стиль' : 'Новый стиль'}
  {onclose}
>
  {#if deleting}<p>Стиль будет удалён. Это действие нельзя отменить.</p>
  {:else}<div class="fields">
      <label>Название<input aria-label="Название стиля" bind:value={name} disabled={busy} /></label>
      <label
        >Как оформлять текст<textarea
          aria-label="Инструкция стиля"
          bind:value={instruction}
          rows="3"
          disabled={busy}></textarea><small
          >Короткое правило оформления · от 3 до 200 символов</small
        ></label
      >
    </div>
    <div class="style-options">
      <h3>Параметры этого стиля</h3>
      <StyleOptions
        {options}
        {busy}
        full
        onchange={async (change) => {
          options = { ...options, [change.name]: change.value };
        }}
      />
    </div>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet actions()}
    {#if deleting}<button
        disabled={busy}
        onclick={() => {
          deleting = false;
        }}>Назад</button
      ><button class="danger" disabled={busy} onclick={ondelete}>Удалить</button>
    {:else}{#if preset}<button
          class="quiet destructive"
          disabled={busy}
          onclick={() => {
            deleting = true;
          }}>Удалить</button
        >{/if}<button disabled={busy} onclick={onclose}>Отмена</button><button
        class="primary"
        disabled={busy || !valid}
        onclick={() => onsave(name, instruction, options)}>Сохранить</button
      >{/if}
  {/snippet}
</Modal>

<style>
  .style-options {
    border-top: 1px solid var(--stroke);
    margin-top: 16px;
    padding-top: 12px;
  }
  h3 {
    font-size: 12px;
    margin: 0 0 6px;
  }
  .fields {
    display: grid;
    gap: 14px;
  }
  label {
    display: grid;
    gap: 6px;
  }
  input,
  textarea {
    width: 100%;
  }
  textarea {
    resize: vertical;
  }
</style>
