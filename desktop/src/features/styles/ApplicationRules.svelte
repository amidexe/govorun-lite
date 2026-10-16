<script lang="ts">
  import type { useStyles } from './state.svelte';
  import type { ApplicationChoice, ApplicationRule } from './application-rules';
  import ApplicationRuleEditor from './ApplicationRuleEditor.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  let { state: stylesState }: { state: ReturnType<typeof useStyles> } = $props();
  let value = $derived(stylesState.styles.value),
    selecting = $state<string | null>(null),
    search = $state('');
  let editor = $state<{ scope: string; rule: ApplicationRule; existing: boolean } | null>(null);
  let hosted = $derived(value?.gate === 'selfHosted' || value?.gate === 'selfHostedSetup');
  let choices = $derived(
    value?.applications.filter((item) =>
      item.name.toLocaleLowerCase().includes(search.toLocaleLowerCase().trim()),
    ) ?? [],
  );
  $effect(() => {
    if (stylesState.styles.failed || hosted || (selecting && selecting !== value?.scope))
      selecting = null;
    if (editor && (stylesState.styles.failed || hosted || editor.scope !== value?.scope))
      editor = null;
  });
  function edit(choice: ApplicationChoice) {
    if (!value) return;
    const rule = value.applicationRules.find((item) => item.id === choice.id);
    editor = {
      scope: value.scope,
      existing: !!rule,
      rule: rule ?? { ...choice, style: null, emojiMode: null, reducedPunctuation: null },
    };
    selecting = null;
  }
  function summary(rule: ApplicationRule) {
    const title =
      rule.style === null
        ? 'Как в общих настройках'
        : rule.style === 'off'
          ? 'Без стилизации'
          : (value?.styles.find((item) => item.id === rule.style)?.title ??
            value?.presets.find((item) => `custom:${item.id}` === rule.style)?.name ??
            'Стиль недоступен');
    return (
      title +
      (rule.emojiMode !== null || rule.reducedPunctuation !== null ? ' · свои параметры' : '')
    );
  }
</script>

{#if value}<section class="surface application-rules">
    <h2>Правила приложений</h2>
    {#if hosted}<p>
        Со своим сервером используется один стиль для всех приложений. Правила Говоруна снова
        применятся после его отключения.
      </p>
    {:else if !value.applicationRules.length}<p>
        По умолчанию стиль и оформление одинаковы во всех приложениях.
      </p>{/if}
    {#each value.applicationRules as rule (rule.id)}<button
        class="rule quiet"
        disabled={stylesState.busy || hosted}
        aria-label={`Изменить правило: ${rule.name}`}
        onclick={() => edit(rule)}
        ><span><strong>{rule.name}</strong><small>{summary(rule)}</small></span><Icon
          name="ph_caret_right"
          size={14}
        /></button
      >{/each}
    <button
      class="quiet add"
      disabled={stylesState.busy || hosted}
      onclick={() => {
        selecting = value!.scope;
        search = '';
        void stylesState.act('loadApplications');
      }}><Icon name="ph_plus" size={14} />Добавить приложение</button
    >
  </section>{/if}
{#if selecting}<Modal
    title="Выберите приложение"
    onclose={() => {
      selecting = null;
    }}
  >
    <input
      class="search"
      aria-label="Поиск приложения"
      placeholder="Поиск приложения"
      bind:value={search}
    />
    {#if stylesState.busy}<p role="status">Загружаем приложения…</p>
    {:else}<div class="applications">
        {#each choices as item (item.id)}<button
            class="quiet"
            aria-label={`Выбрать приложение: ${item.name}`}
            onclick={() => edit(item)}>{item.name}<Icon name="ph_caret_right" size={14} /></button
          >{/each}
      </div>
      {#if !choices.length}<p>Приложение не найдено. Откройте его и обновите список.</p>{/if}
    {/if}
    {#if stylesState.error}<p class="error" role="alert">{stylesState.error}</p>{/if}
    {#snippet actions()}<button
        disabled={stylesState.busy}
        onclick={() => stylesState.act('loadApplications')}>Обновить список</button
      ><button
        onclick={() => {
          selecting = null;
        }}>Отмена</button
      >{/snippet}
  </Modal>{/if}
{#if editor && value}<ApplicationRuleEditor
    rule={editor.rule}
    existing={editor.existing}
    {value}
    busy={stylesState.busy}
    error={stylesState.error}
    onclose={() => {
      editor = null;
    }}
    onsave={async (params) => {
      if (await stylesState.act('saveApplicationRule', params)) editor = null;
    }}
    ondelete={async () => {
      if (
        editor &&
        (await stylesState.act('deleteApplicationRule', { id: editor.rule.id, confirmed: true }))
      )
        editor = null;
    }}
  />{/if}

<style>
  .rule {
    width: 100%;
    justify-content: space-between;
    text-align: left;
    padding: 10px 0;
    border-radius: 0;
    border-bottom: 1px solid var(--stroke);
  }
  .rule strong {
    font-weight: 500;
  }
  .add {
    margin-top: 8px;
    padding-left: 0;
    color: var(--accent-text);
  }
  .search {
    width: 100%;
    margin-bottom: 12px;
  }
  .applications {
    max-height: 280px;
    overflow: auto;
  }
  .applications button {
    width: 100%;
    justify-content: space-between;
    text-align: left;
  }
</style>
