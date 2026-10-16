<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { dictionarySnapshot } from './model.ts';
  import { dictionaryText, parseDictionary } from './text';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import { dictionaryDraft as draft } from './draft.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  const api = useDesktopApi();
  const dictionary = useNativeSnapshot(
    'dictionary.snapshot',
    dictionarySnapshot,
    'dictionary.changed',
  );
  let busy = $state(false),
    message = $state(''),
    toggleError = $state(''),
    saved = $state(false);
  $effect(() => {
    if (dictionary.value && !draft.dirty) {
      draft.text = dictionaryText(dictionary.value.entries);
      draft.loaded = true;
    }
  });
  function edited() {
    draft.dirty = true;
    saved = false;
    message = '';
  }
  function discard() {
    if (!dictionary.value) return;
    draft.text = dictionaryText(dictionary.value.entries);
    draft.dirty = false;
    saved = false;
    message = '';
  }
  async function save() {
    message = '';
    let entries;
    try {
      entries = parseDictionary(draft.text, dictionary.value?.entries ?? []);
    } catch (error) {
      message = error instanceof Error ? error.message : 'Проверьте правила словаря.';
      return;
    }
    if (!dictionary.value) return;
    busy = true;
    try {
      const value = await api.invoke('dictionary.replace', {
        enabled: dictionary.value.enabled,
        revision: dictionary.value.revision,
        entries,
      });
      dictionary.accept(value);
      draft.dirty = false;
      saved = true;
    } catch {
      message = 'Не удалось сохранить словарь. Ваши изменения остались в редакторе.';
    } finally {
      busy = false;
    }
  }
  async function toggle(control: HTMLInputElement) {
    busy = true;
    toggleError = '';
    try {
      dictionary.accept(await api.invoke('dictionary.setEnabled', { enabled: control.checked, revision: dictionary.value!.revision }));
    } catch {
      toggleError = 'Не удалось применить настройку. Попробуйте ещё раз.';
    } finally {
      busy = false;
      control.checked = dictionary.value?.enabled ?? false;
    }
  }
</script>

{#if dictionary.failed}<div class="notice" role="alert">
    Не удалось обновить словарь. <button onclick={dictionary.refresh}>Повторить</button>
  </div>{/if}
{#if draft.loaded}
  <section class="surface">
    <label class="setting-row">
      <span
        ><strong>Использовать словарь замен</strong><small
          >Правила применяются после распознавания и до обработки текста</small
        ></span
      >
      <input
        type="checkbox"
        role="switch"
        checked={dictionary.value?.enabled ?? false}
        onchange={(event) => toggle(event.currentTarget)}
        disabled={busy || dictionary.failed || !dictionary.value}
      />
    </label>
    {#if toggleError}<p class="error" role="alert">{toggleError}</p>{/if}
  </section>
  <section class="surface">
    <div class="section-heading">
      <div>
        <h2>Словарь замен</h2>
        <p>Работает локально; синхронизация зависит от выбранного режима аккаунта.</p>
      </div>
    </div>
    <div class="editor-actions">
      <span role="status" class="success"
        >{saved ? 'Сохранено' : draft.dirty ? 'Есть несохранённые изменения' : ''}</span
      >
      <button onclick={discard} disabled={busy || !draft.dirty}
        ><Icon name="ph_arrow_counter_clockwise" size={14} />Сбросить</button
      >
      <button class="primary" onclick={save} disabled={busy || !draft.dirty}
        ><Icon name="ph_check" size={14} />{busy ? 'Сохраняем…' : 'Сохранить'}</button
      >
    </div>
    <textarea
      aria-label="Правила замен"
      spellcheck="false"
      bind:value={draft.text}
      oninput={edited}
      disabled={busy}></textarea>
    {#if message}<p class="error" role="alert">{message}</p>{/if}
  </section>
  <section class="surface">
    <h2>Формат</h2>
    <div class="example">
      <code>говорун = Говорун</code>
      <p>Заменить слово</p>
    </div>
    <div class="example">
      <code>эй ай, аи = AI</code>
      <p>Несколько вариантов через запятую</p>
    </div>
    <div class="example">
      <code>ну вот =</code>
      <p>Удалить выражение</p>
    </div>
  </section>
{:else if dictionary.loading}<p role="status">Загружаем словарь…</p>{/if}

<style>
  textarea {
    min-height: 220px;
    width: 100%;
    resize: vertical;
    font-family: ui-monospace, monospace;
    font-size: 12px;
    line-height: 1.6;
  }
  .editor-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 12px 0;
  }
  .editor-actions span {
    margin-right: auto;
    font-size: 11px;
  }
  .example {
    display: flex;
    justify-content: space-between;
    gap: 20px;
    padding: 12px 0;
    border-bottom: 1px solid var(--stroke);
  }
  .example:last-child {
    border: 0;
    padding-bottom: 0;
  }
  .example code {
    font-size: 12px;
  }
</style>
