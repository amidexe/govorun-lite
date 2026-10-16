<script lang="ts">
  import { useDesktopNavigation } from '../../shared/state/desktop-navigation';
  import { useStyles } from '../../features/styles/state.svelte';
  import type { StylePreset, TextStyleOptions } from '../../features/styles/model';
  import { stylePresentation, styleStatus } from '../../features/styles/style-presentation';
  import { styleAccessReturn } from '../../features/styles/access-return';
  import { onDestroy } from 'svelte';
  import StyleChooser from '../../features/styles/StyleChooser.svelte';
  import StyleOptions from '../../features/styles/StyleOptions.svelte';
  import StylePresetEditor from '../../features/styles/StylePresetEditor.svelte';
  import StyleTerms from '../../features/styles/StyleTerms.svelte';
  import StyleBetaNotice from '../../features/styles/StyleBetaNotice.svelte';
  import StyleAccessCard from './StyleAccessCard.svelte';
  import ApplicationRules from '../../features/styles/ApplicationRules.svelte';
  import AccountPage from '../account/AccountPage.svelte';
  import StyleStorageRecovery from './StyleStorageRecovery.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import { slide } from 'svelte/transition';
  import { motionDuration } from '../../motion';
  const navigate = useDesktopNavigation();
  let { compact = false, processingStatus = 'idle' }: { compact?: boolean; processingStatus?: 'idle' | 'processing' | 'failed' } = $props();
  const stylesState = useStyles();
  let value = $derived(stylesState.styles.value),
    expanded = $state(false),
    showAccount = $state(false);
  let presentation = $derived(value ? stylePresentation(value) : null);
  let initialScope = $state<string | null>(null);
  let previouslyEnabled = false;
  $effect(() => {
    if (value) {
      if (initialScope !== value.scope) {
        initialScope = value.scope;
        expanded = !value.enabled || value.selected === null;
      } else if (!previouslyEnabled && value.enabled) expanded = true;
      previouslyEnabled = value.enabled;
    }
  });
  let noticeScope: string | null = null;
  let suppressOldNotice = $state(false);
  let readErrorDismissed = $state(false);
  $effect(() => {
    if (!stylesState.styles.failed) readErrorDismissed = false;
    // A failed panel read already presents its error. Recovering that read must
    // not reveal another notice for the dictation that preceded the recovery.
    if (stylesState.styles.failed && processingStatus === 'failed') suppressOldNotice = true;
    if (value && (noticeScope !== value.scope || presentation?.limit)) {
      if (noticeScope !== null || presentation?.limit) suppressOldNotice = true;
      noticeScope = value.scope;
    }
    if (processingStatus !== 'failed') suppressOldNotice = false;
  });
  let subscriptionScope: string | null = null;
  let disposed = false;
  async function connected() {
    if (!showAccount) return;
    showAccount = false;
    await stylesState.styles.refresh();
    if (
      !disposed &&
      !stylesState.styles.failed &&
      (value?.gate === 'terms' || value?.gate === 'ready')
    )
      select(value.selected);
  }
  onDestroy(() => {
    disposed = true;
    if (subscriptionScope) styleAccessReturn.schedule(subscriptionScope);
  });
  $effect(() => {
    if (compact && value && !stylesState.busy && styleAccessReturn.take(value))
      void stylesState.act('refresh');
  });
  async function openSubscription() {
    subscriptionScope = compact ? (value?.scope ?? null) : null;
    navigate('subscription');
  }
  let terms = $state<{ scope: string; version: number; selection: string } | null>(null);
  let editor = $state<{ scope: string; preset: StylePreset | null } | null>(null);
  $effect(() => {
    if (
      terms &&
      (stylesState.styles.failed ||
        value?.scope !== terms.scope ||
        value?.termsVersion !== terms.version)
    )
      terms = null;
    if (terms && value?.enabled && value.selected === terms.selection && value.gate === 'ready')
      terms = null;
    if (editor && (stylesState.styles.failed || value?.scope !== editor.scope)) editor = null;
  });
  function select(selection: string | null) {
    if (selection === null) { expanded = true; return; }
    if (!value) return;
    if (selection === 'off') void stylesState.act('disable');
    else if (value.gate === 'terms')
      terms = { scope: value.scope, version: value.termsVersion, selection };
    else void stylesState.act('select', { selection });
  }
  function closeTerms() {
    terms = null;
    void stylesState.act('cancel');
  }
  async function save(name: string, instruction: string, options: TextStyleOptions) {
    if (
      await stylesState.act('savePreset', {
        id: editor?.preset?.id ?? null,
        name,
        instruction,
        options,
      })
    )
      editor = null;
  }
  async function remove() {
    if (
      editor?.preset &&
      (await stylesState.act('deletePreset', { id: editor.preset.id, confirmed: true }))
    )
      editor = null;
  }
  const gateCopy: Record<string, string | null> = {
    account:
      'После записи текст можно дополнительно обработать через интернет. Для этого нужен вход. Обычная диктовка работает без него.',
    storage: 'Для подключения нужен доступ к защищённому хранилищу.',
    terms: 'Выберите стиль и примите условия интернет-обработки.',
    access: 'Улучшение текста пока недоступно для этого аккаунта.',
    unavailable: 'Стилизация временно недоступна. Диктовка остаётся доступна.',
    selfHosted: 'Текст обрабатывает ваш сервер. Подписка Говоруна не используется.',
    selfHostedSetup: 'Проверьте подключение и согласие на обработку своим сервером.',
    ready: '',
  };
</script>

{#if stylesState.styles.failed && !value}<section class="surface">
    <h2>Улучшение текста</h2>
    {#if !readErrorDismissed}<div class="processing-notice" role="alert">
      <Icon name="ph_info" size={18} /><span>Не удалось проверить улучшение</span>
      <button class="quiet icon-button" aria-label="Скрыть сообщение" onclick={() => { readErrorDismissed = true; }}><Icon name="ph_x" size={16} /></button>
    </div>{/if}
  </section>
{:else if value}
  {#if terms}<StyleTerms
      selection={terms.selection}
      busy={stylesState.busy}
      error={stylesState.error ?? value.message}
      onclose={closeTerms}
      ondocuments={() => stylesState.act('documents')}
      onaccept={(selection) => {
        if (terms)
          void stylesState.act('acceptTerms', {
            termsVersion: terms.version,
            selection,
            confirmed: true,
          });
      }}
    />
  {:else if showAccount}<section class="surface">
    <h2>Улучшение текста</h2>
    <AccountPage connectionOnly embedded onconnected={connected} />
  </section>
  {:else if value.gate === 'account'}
    <section class="surface promotion">
      <h2><Icon name="ph_sparkle" /> Улучшение текста</h2>
      <StyleBetaNotice compact />
      <p>
        Онлайн-нейросеть учитывает контекст, убирает повторы и превращает устную речь в связный
        текст. Меньше ручных правок после диктовки.
      </p>
      <button
        class="primary"
        onclick={() => {
          showAccount = true;
        }}>Попробовать</button
      >
    </section>
  {:else if value.gate === 'access'}
    <StyleAccessCard
      {value}
      onready={() => stylesState.styles.refresh()}
      onplans={openSubscription}
    />
  {:else if value.gate === 'terms' && value.selected !== null}
    <section class="surface activation-prompt">
      <div>
        <h2>Улучшение текста</h2>
        <StyleBetaNotice compact />
        <p role="status">{styleStatus(value)}</p>
      </div>
      <button class="primary" disabled={stylesState.busy} onclick={() => select(value.selected)}
        >Включить</button
      >
    </section>
  {:else}
    <section class="surface styles-panel" class:compact>
      {#if !compact && value.gate !== 'storage' && value.gate !== 'selfHosted' && value.gate !== 'selfHostedSetup'}<StyleBetaNotice
        />{/if}
      {#if !compact && value.gate !== 'storage'}<p
          class="style-status"
          class:active={presentation?.active && !value.onlineServicePaused && !stylesState.styles.failed}
          role="status"
        >
          {styleStatus(stylesState.styles.failed ? { ...value, displayState: 'checking' } : value)}
        </p>{/if}
      {#if compact && value.gate === 'storage'}<div class="expander">
          <Icon name="ph_sparkle" /><span>Улучшение текста</span><span class="style-summary"
            >Без стилизации</span
          >
        </div>
      {:else if compact}<button
          class="expander"
          aria-expanded={expanded}
          onclick={() => {
            expanded = !expanded;
          }}
          ><Icon name={expanded ? 'ph_caret_down' : 'ph_caret_right'} size={14} /><Icon
            name="ph_sparkle"
          /><span>Улучшение текста</span><span
            class="style-summary"
            >{stylesState.styles.failed || presentation?.preparing || value.displayState === 'checking' ? 'Проверяем…' : presentation?.summary}</span
          ></button
        >{:else}<h2>Стиль</h2>{/if}
      {#if !stylesState.styles.failed && value.onlineServicePaused && value.enabled && value.gate === 'ready' && value.displayState === 'active' && !value.message && !stylesState.error}<p
          class="outage-notice"
          role="status"
        >
          Улучшение текста временно недоступно. Обычная диктовка работает.
        </p>{/if}
      {#if !stylesState.styles.failed && compact && !presentation?.limit && !value.message && !stylesState.error && ['saved', 'checking', 'preparing'].includes(value.displayState)}<p class="outage-notice" role="status">{styleStatus(value)}</p>{/if}
      {#if !stylesState.styles.failed && value.enabled && value.displayState === 'active' && !value.onlineServicePaused && !value.message && !stylesState.error && value.gate !== 'storage' && !presentation?.limit && processingStatus === 'processing'}<p class="outage-notice" role="status">Улучшаем текст…</p>{/if}
      {#if !stylesState.styles.failed && !presentation?.limit && (value.message || stylesState.error)}<p class="error outage-notice" role="alert">{stylesState.error ?? value.message}</p>{/if}
      {#if value.gate === 'storage'}<div class="panel-content">
          <StyleStorageRecovery onrecovered={() => stylesState.styles.refresh()} onrestart={() => { showAccount = true; }} />
        </div>{/if}
      {#if !stylesState.styles.failed && compact && presentation?.limit}<div class="limit-notice">
          <div>
            <p>{presentation.limit}</p>
            <small>Диктовка остаётся доступна.</small>
          </div>
          <button class="quiet" onclick={openSubscription}>Подписка и лимиты</button>
        </div>{/if}
      {#if value.gate !== 'storage' && (!compact || expanded)}<div
          class="panel-content"
          transition:slide={{ duration: compact ? motionDuration() : 0 }}
        >
          {#if showAccount}<AccountPage connectionOnly onconnected={connected} /><button
              onclick={() => {
                showAccount = false;
              }}>Вернуться к выбору стиля</button
            >{:else}
            {#if !stylesState.styles.failed && gateCopy[value.gate] && !value.message && !stylesState.error && !(compact && presentation?.limit)}<div
                class="access-notice"
              >
                {#if !(compact && value.gate === 'unavailable' && ['saved', 'checking', 'preparing'].includes(value.displayState))}<p>
                  {value.gate === 'unavailable'
                    ? (value.availability ?? gateCopy.unavailable)
                    : gateCopy[value.gate]}
                </p>{/if}
                {#if value.gate === 'unavailable'}<div class="access-actions">
                    <button class="primary" onclick={openSubscription}>Подписка и лимиты</button
                    >{#if !compact}<button
                        disabled={stylesState.busy}
                        onclick={() => stylesState.act('refresh')}>Обновить доступ</button
                      >{/if}
                  </div>{/if}
              </div>{/if}
            <StyleChooser
              {value}
              {compact}
              busy={stylesState.busy}
              onselect={select}
              onedit={(preset) => {
                editor = { scope: value!.scope, preset };
              }}
              oncreate={() => {
                editor = { scope: value!.scope, preset: null };
              }}
            />
            {#if !compact && value.gate !== 'selfHosted' && value.gate !== 'selfHostedSetup'}<p
                class="length-hint"
              >
                Улучшение применяется после завершения записи к тексту от {value.processingLimits.minimumCharacters} до {new Intl.NumberFormat('ru-RU').format(value.processingLimits.maximumCharacters).replaceAll('\u00a0', ' ')} символов.
                Длинный текст может обрабатываться до {value.processingLimits.maximumProcessingSeconds} секунд. Более короткий или длинный текст
                вставляется без улучшения.
              </p>{/if}
            {#if !compact && value.enabled && value.gate !== 'selfHosted' && value.gate !== 'selfHostedSetup'}<div
                class="options"
              >
                <h3>Параметры стиля «{value.title}»</h3>
                <StyleOptions
                  options={value.options}
                  busy={stylesState.busy}
                  full
                  onchange={(change) => stylesState.act('option', change)}
                />
              </div>{/if}
          {/if}
          {#if compact}<button
              class="quiet settings-link"
              onclick={() => navigate('styles')}
              ><Icon name="ph_sliders_horizontal" size={16} />Настройки текста</button
            >{/if}
        </div>{/if}
      {#if (stylesState.styles.failed && !readErrorDismissed) || (!stylesState.styles.failed && value.enabled && ['ready', 'selfHosted'].includes(value.gate) && !value.onlineServicePaused && !value.message && !stylesState.error && !presentation?.limit && !suppressOldNotice && processingStatus === 'failed')}
        <div class="processing-notice" role={stylesState.styles.failed ? 'alert' : 'status'}>
          <Icon name="ph_info" size={18} /><span>{stylesState.styles.failed ? 'Не удалось проверить улучшение' : 'Последняя диктовка — без улучшения'}</span>
          <button class="quiet icon-button" aria-label="Скрыть сообщение" onclick={() => { if (stylesState.styles.failed) readErrorDismissed = true; else suppressOldNotice = true; }}><Icon name="ph_x" size={16} /></button>
        </div>
      {/if}
    </section>
  {/if}
  {#if !compact}<ApplicationRules state={stylesState} />
    <section class="surface">
      <h2>Защита данных</h2>
      <p>Аудио остаётся на устройстве. Проверка доступа и обработка текста разделены.</p>
      <button class="quiet" onclick={() => stylesState.act('documents')}>Документы</button>
    </section>{/if}
{:else}<section class="surface">
    <h2>Улучшение текста</h2>
    <p role="status">Проверяем…</p>
  </section>{/if}

{#if editor}<StylePresetEditor
    preset={editor.preset}
    busy={stylesState.busy}
    error={stylesState.error}
    onclose={() => {
      editor = null;
    }}
    onsave={save}
    ondelete={remove}
  />{/if}

<style>
  .activation-prompt {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .activation-prompt h2 {
    margin: 0;
    font-size: 13px;
  }
  .activation-prompt p {
    margin-top: 4px;
    font-size: 12px;
    color: var(--secondary);
  }
  .processing-notice {
    display: flex; align-items: center; gap: 8px; padding: 8px 14px;
    border-top: 1px solid var(--stroke); background: var(--field); color: var(--text); font-size: 13px;
  }
  .processing-notice span { flex: 1; }
  .processing-notice button { color: var(--secondary); }
  .style-status {
    margin-bottom: 12px;
    font-size: 13px;
  }
  .style-status.active {
    color: var(--green);
  }
  .length-hint {
    margin-top: 12px;
    color: var(--secondary);
    font-size: 12px;
  }
  .styles-panel.compact {
    padding: 0;
    overflow: hidden;
  }
  .compact .panel-content {
    padding: 0 16px 12px;
  }
  .expander {
    display: flex;
    align-items: center;
  }
  .expander span {
    flex: 1;
  }
  .expander .style-summary {
    flex: 0 1 auto;
    color: var(--secondary);
    font-size: 12px;
    text-align: right;
  }
  .access-notice {
    display: grid;
    justify-items: start;
    gap: 8px;
    margin-bottom: 12px;
  }
  .outage-notice {
    padding: 2px 16px 12px;
    color: var(--secondary);
    font-size: 12px;
  }
  .limit-notice {
    display: grid;
    justify-items: start;
    gap: 10px;
    padding: 2px 14px 12px;
  }
  .limit-notice p {
    color: var(--text);
    font-size: 13px;
  }
  .limit-notice small {
    display: block;
    color: var(--secondary);
    font-size: 11px;
    margin-top: 3px;
  }
  .limit-notice button {
    font-size: 12px;
  }
  .access-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .options h3 {
    font-size: 12px;
    margin: 6px 0;
  }
  .options {
    border-top: 1px solid var(--stroke);
    margin-top: 12px;
    padding-top: 6px;
  }
  .settings-link {
    margin-top: 6px;
    width: 100%;
    justify-content: space-between;
    color: var(--secondary);
  }
</style>
