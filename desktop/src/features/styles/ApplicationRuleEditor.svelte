<script lang="ts">
  import { untrack } from 'svelte';
  import type { ApplicationRule, EmojiMode } from './application-rules';
  import type { StylesActions } from '../../ipc/actions';
  import type { StylesSnapshot } from './model';
  import Modal from '../../shared/ui/Modal.svelte';
  let {
    rule,
    existing,
    value,
    busy,
    error,
    onclose,
    onsave,
    ondelete,
  }: {
    rule: ApplicationRule;
    existing: boolean;
    value: StylesSnapshot;
    busy: boolean;
    error: string | null;
    onclose: () => void;
    onsave: (params: StylesActions['saveApplicationRule']) => Promise<void>;
    ondelete: () => Promise<void>;
  } = $props();
  let style = $state(untrack(() => rule.style ?? ''));
  let customOptions = $state(
    untrack(() => rule.emojiMode !== null || rule.reducedPunctuation !== null),
  );
  let emojiMode = $state<EmojiMode>(untrack(() => rule.emojiMode ?? value.options.emojiMode));
  let reducedPunctuation = $state(
    untrack(() => rule.reducedPunctuation ?? value.options.reducedPunctuation),
  );
  let deleting = $state(false);
  let knownStyle = $derived(
    style === '' ||
      style === 'off' ||
      value.styles.some((item) => item.id === style) ||
      value.presets.some((item) => `custom:${item.id}` === style),
  );
</script>

<Modal title={deleting ? `Удалить правило для «${rule.name}»?` : rule.name} {onclose}>
  {#if deleting}<p>Приложение снова будет использовать общий стиль и оформление.</p>
  {:else}
    <label class="setting-row"
      ><span>Стиль текста</span><select
        class="setting-control"
        aria-label="Стиль для приложения"
        bind:value={style}
        disabled={busy}
      >
        <option value="">Как в общих настройках</option><option value="off">Без стилизации</option>
        {#each value.styles as item}<option value={item.id}>{item.title}</option>{/each}
        {#each value.presets as item}<option value={`custom:${item.id}`}>{item.name}</option>{/each}
        {#if !knownStyle}<option value={style}>Стиль недоступен — выберите другой</option>{/if}
      </select></label
    >
    <label class="setting-row"
      ><span>Свои параметры для приложения</span><input
        type="checkbox"
        role="switch"
        bind:checked={customOptions}
        disabled={busy}
      /></label
    >
    {#if customOptions}
      <label class="setting-row"
        ><span>Эмодзи</span><select
          class="setting-control"
          aria-label="Эмодзи для приложения"
          bind:value={emojiMode}
          disabled={busy}
          ><option value="off">Выкл.</option><option value="restrained">Сдержанно</option><option
            value="lively">Живо</option
          ><option value="bright">Ярко</option></select
        ></label
      >
      <label class="setting-row"
        ><span>Меньше пунктуации</span><input
          type="checkbox"
          role="switch"
          bind:checked={reducedPunctuation}
          disabled={busy}
        /></label
      >
    {:else}<p>Эмодзи и пунктуация наследуются из общих настроек.</p>{/if}
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#snippet actions()}
    {#if deleting}<button
        disabled={busy}
        onclick={() => {
          deleting = false;
        }}>Назад</button
      ><button class="danger" disabled={busy} onclick={ondelete}>Удалить правило</button>
    {:else}{#if existing}<button
          class="quiet destructive"
          disabled={busy}
          onclick={() => {
            deleting = true;
          }}>Удалить правило</button
        >{/if}<button disabled={busy} onclick={onclose}>Отмена</button><button
        class="primary"
        disabled={busy || !knownStyle}
        onclick={() =>
          onsave({
            id: rule.id,
            style: style || null,
            customOptions,
            emojiMode,
            reducedPunctuation,
          })}>Сохранить</button
      >{/if}
  {/snippet}
</Modal>

<style>
  .setting-row {
    padding: 8px 0;
  }
  .setting-row + .setting-row {
    border-top: 1px solid var(--stroke);
  }
</style>
