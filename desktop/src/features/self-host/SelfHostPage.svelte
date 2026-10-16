<script lang="ts">
  import { choice } from '../../shared/forms/choice';
  import { useSelfHost } from './state.svelte';
  import {
    canConnect,
    configurationChanged,
    selfHostStatus,
    type SelfHostConfiguration,
    type SelfHostSnapshot,
  } from './model';
  import Modal from '../../shared/ui/Modal.svelte';
  const selfHost = useSelfHost();
  let value = $derived(selfHost.saved.value);
  let draft = $state<SelfHostConfiguration | null>(null),
    sourceScope = $state(''),
    key = $state(''),
    consent = $state(false);
  let confirmation = $state<{
    action: 'clear' | 'removeKey' | 'provider';
    provider?: string;
    scope: string;
  } | null>(null);
  let sourceConfiguration = $state<SelfHostConfiguration | null>(null);
  let dirty = $derived(configurationChanged(draft, sourceConfiguration));
  let stale = $derived(!!draft && !!value && sourceScope !== value.scope && (dirty || !!key));
  function adopt(snapshot: SelfHostSnapshot) {
    draft = { ...snapshot.configuration };
    sourceConfiguration = { ...snapshot.configuration };
    sourceScope = snapshot.scope;
    consent = snapshot.consent;
    key = '';
  }
  $effect(() => {
    if (value && (!draft || (sourceScope !== value.scope && !dirty && !key && !selfHost.pending)))
      adopt(value);
    if (confirmation && (selfHost.saved.failed || confirmation.scope !== value?.scope))
      confirmation = null;
  });
  function changedRecipient() {
    consent = false;
  }
  async function setConsent(checked: boolean) {
    consent = checked;
    if (!checked && value?.consent) {
      const result = await selfHost.act('revokeConsent');
      if (result) sourceScope = result.scope;
    }
  }
  async function connect() {
    if (!draft || !canConnect(draft, consent) || stale || selfHost.busy) return;
    const result = await selfHost.actForScope(sourceScope, 'connect', {
      configuration: { ...draft },
      key: key.trim(),
      confirmed: true,
    });
    if (result) adopt(result);
  }
  async function switchProvider(provider: string) {
    const result = await selfHost.act('provider', {
      provider: choice(provider, ['ollama', 'openai', 'gemini'] as const),
    });
    if (result) adopt(result);
  }
  async function confirm() {
    const current = confirmation;
    confirmation = null;
    if (!current || current.scope !== value?.scope) return;
    if (current.action === 'provider') await switchProvider(current.provider!);
    else {
      const result = await selfHost.act(current.action, { confirmed: true });
      if (result) adopt(result);
    }
  }
</script>

{#if selfHost.saved.failed}<div class="notice" role="alert">
    <p>Не удалось загрузить настройки сервера.</p>
    <button onclick={() => selfHost.saved.refresh()}>Попробовать ещё раз</button>
  </div>{/if}
{#if draft && value}
  <section class="surface connection">
    <h2>Подключение</h2>
    <label class="setting-row"
      ><span>Провайдер</span><select
        value={draft.provider}
        disabled={selfHost.busy}
        onchange={(event) => {
          const provider = event.currentTarget.value;
          event.currentTarget.value = draft!.provider;
          if (dirty || key) confirmation = { action: 'provider', provider, scope: value!.scope };
          else void switchProvider(provider);
        }}
        ><option value="ollama">OpenAI-совместимый</option><option value="openai">OpenAI</option
        ><option value="gemini">Google Gemini</option></select
      ></label
    >
    <label class="field"
      >Адрес сервера<input
        aria-label="Адрес сервера"
        type="url"
        placeholder="https://server.example/v1"
        bind:value={draft.endpoint}
        disabled={selfHost.busy}
        oninput={changedRecipient}
      /></label
    >
    <label class="field"
      >Модель<input
        aria-label="Модель своего сервера"
        placeholder="model-name"
        bind:value={draft.model}
        disabled={selfHost.busy}
      /></label
    >
    <label class="field"
      >API-ключ, если требуется<input
        aria-label="API-ключ своего сервера"
        type="password"
        autocomplete="off"
        spellcheck="false"
        placeholder={value.keyState === 'saved'
          ? 'Ключ сохранён · введите новый для замены'
          : 'Необязательный ключ'}
        bind:value={key}
        disabled={selfHost.busy}
      /><small>Хранится в защищённом хранилище устройства.</small></label
    >
    {#if value.keyState === 'inaccessible' || value.keyState === 'staleReference'}<p role="status">
        {value.keyState === 'inaccessible'
          ? 'Система не даёт доступ к сохранённому ключу. Введите его заново.'
          : 'Сохранённый ключ не найден. Введите его заново.'}
      </p>{/if}
    {#if value.keyState !== 'missing'}<button
        class="quiet destructive"
        disabled={selfHost.busy}
        onclick={() => {
          confirmation = { action: 'removeKey', scope: value!.scope };
        }}>Удалить сохранённый ключ</button
      >{/if}
    <label class="field"
      >Промпт<textarea
        aria-label="Промпт своего сервера"
        bind:value={draft.prompt}
        disabled={selfHost.busy}
        spellcheck="false"></textarea><small>Один полный промпт для всех диктовок.</small></label
    >
  </section>
  <section class="surface">
    <h2>Прокси, если требуется</h2>
    <label class="setting-row"
      ><span>Использовать прокси</span><input
        type="checkbox"
        role="switch"
        bind:checked={draft.proxyEnabled}
        disabled={selfHost.busy}
        onchange={changedRecipient}
      /></label
    >
    {#if draft.proxyEnabled}<label class="field"
        >Адрес прокси<input
          aria-label="Адрес прокси"
          placeholder="http://host:8080 или socks5://host:1080"
          bind:value={draft.proxyURL}
          disabled={selfHost.busy}
          oninput={changedRecipient}
        /></label
      >
      <p>HTTP или SOCKS5 без логина и пароля. Прокси также участвует в передаче данных.</p>{/if}
  </section>
  <section class="surface transfer">
    <h2>Передача данных</h2>
    <p>
      Распознанный текст, промпт и API-ключ (если задан) отправляются напрямую на указанный сервер.
      Аудио остаётся на устройстве. Вы выбираете получателя и отвечаете за право передавать данные.
      Хранение, страны обработки и защита данных зависят от этого сервера; гарантии приватности
      сервиса Говоруна здесь не действуют.
    </p>
    {#if draft.endpoint.trim().toLowerCase().startsWith('http://')}<p class="transport-notice">
        HTTP не шифрует текст и ключ при передаче.
      </p>{/if}
    <label class="acknowledgement"
      ><input
        type="checkbox"
        checked={consent}
        disabled={selfHost.busy}
        onchange={(event) => setConsent(event.currentTarget.checked)}
      /><span>Я доверяю этому серверу и согласен передавать ему данные</span></label
    >
    {#if dirty || key}<p>
        Изменения применятся после нажатия «Подключить». До этого действует сохранённая
        конфигурация.
      </p>{/if}
    {#if stale}<p role="alert">Настройки изменились. Обновите форму перед подключением.</p>{/if}
    {#if dirty || key}<button class="quiet" disabled={selfHost.busy} onclick={() => adopt(value!)}
        >Отменить изменения</button
      >{/if}
    {#if value.message}<p class:error={value.failed} role="status">{value.message}</p>
    {:else if !dirty && value.routeEnabled}<p role="status">{selfHostStatus(value)}</p>{/if}
    <div class="actions">
      <button
        class="primary"
        disabled={selfHost.busy || stale || !canConnect(draft, consent)}
        onclick={connect}
        >{value.busy
          ? value.operation === 'connect'
            ? 'Проверяем…'
            : 'Выполняем…'
          : 'Подключить'}</button
      >
      {#if value.operation === 'connect'}<button
          disabled={value.cancelling}
          onclick={selfHost.cancel}>{value.cancelling ? 'Отменяем…' : 'Отменить проверку'}</button
        >{/if}
      {#if value.routeEnabled}<button
          disabled={selfHost.busy}
          onclick={() => selfHost.act('disconnect')}>Отключить свой сервер</button
        >{/if}
    </div>
  </section>
  <section class="surface">
    <h2>Удаление настроек</h2>
    <button
      class="quiet destructive"
      disabled={selfHost.busy}
      onclick={() => {
        confirmation = { action: 'clear', scope: value!.scope };
      }}>Сбросить настройки сервера</button
    >
  </section>
{:else if selfHost.saved.loading}<p role="status">Загружаем настройки сервера…</p>{/if}
{#if selfHost.error}<p class="error" role="alert">{selfHost.error}</p>{/if}
{#if confirmation}<Modal
    title={confirmation.action === 'clear'
      ? 'Сбросить настройки сервера?'
      : confirmation.action === 'removeKey'
        ? 'Удалить сохранённый ключ?'
        : 'Сменить провайдера?'}
    onclose={() => {
      confirmation = null;
    }}
  >
    <p>
      {confirmation.action === 'clear'
        ? 'Сохранённые адреса, ключи, модели, промпт и прокси будут удалены с этого устройства. Стилизация отключится. Аккаунт, статистика и модель распознавания сохранятся. Данные на самом сервере не удаляются.'
        : confirmation.action === 'removeKey'
          ? 'Ключ будет удалён с этого устройства, обработка отключится. Остальные настройки сохранятся.'
          : 'Неприменённые изменения формы будут отменены. Сохранённые настройки каждого провайдера останутся.'}
    </p>
    {#snippet actions()}<button
        onclick={() => {
          confirmation = null;
        }}>Отмена</button
      ><button class={confirmation?.action === 'provider' ? 'primary' : 'danger'} onclick={confirm}
        >{confirmation?.action === 'provider' ? 'Сменить' : 'Удалить'}</button
      >{/snippet}
  </Modal>{/if}

<style>
  .connection,
  .transfer {
    display: grid;
    gap: 12px;
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .field input {
    width: 100%;
  }
  textarea {
    min-height: 110px;
    resize: vertical;
  }
  select {
    max-width: 230px;
  }
  .acknowledgement {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }
  .acknowledgement input {
    margin-top: 3px;
    flex-shrink: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .quiet {
    justify-self: start;
  }
  .transport-notice {
    color: var(--red);
  }
</style>
