<script lang="ts">
  import { useBilling } from '../../features/billing/state.svelte';
  import type { StylesSnapshot } from '../../features/styles/model';
  import StyleTerms from '../../features/styles/StyleTerms.svelte';
  import StyleBetaNotice from '../../features/styles/StyleBetaNotice.svelte';
  let {
    value,
    onready,
    onplans,
  }: { value: StylesSnapshot; onready: () => void; onplans: () => void } = $props();
  const store = useBilling();
  let billing = $derived(store.billing.value),
    dismissed = $state(false),
    submitted = $state(false);
  let refreshed = $state<string | null>(null);
  $effect(() => {
    if (billing?.signedIn && refreshed !== billing.scope && !store.busy) {
      refreshed = billing.scope;
      submitted = false;
      void store.act('refresh');
    }
    if (submitted && !store.busy && billing?.hasAccess && !billing.canActivateStarter) {
      submitted = false;
      onready();
    }
  });
  async function activate(selection: string | null) {
    if (!billing || store.busy || selection === null) return;
    submitted = true;
    await store.act('activateStarter', {
      confirmed: true,
      termsVersion: billing.termsVersion,
      selection,
    });
  }
</script>

{#if billing?.canActivateStarter && !dismissed && !value.termsAccepted}
  <StyleTerms
    selection={typeof value.selected === 'string' && value.styles.some((style) => style.id === value.selected)
      ? value.selected
      : 'formal'}
    busy={store.busy}
    error={store.error ?? billing.message}
    canCancel={!store.busy}
    onclose={() => {
      dismissed = true;
    }}
    onaccept={activate}
    ondocuments={() => store.act('legal', { document: 'onlineTerms' })}
  />
{:else}
  <section class="surface">
    <h2>Улучшение текста</h2>
    <StyleBetaNotice compact />
    {#if store.billing.failed || store.error}
      <p role="alert">Не удалось проверить доступ.</p>
      <button onclick={() => store.retry()}>Повторить</button>
    {:else if !billing || store.busy}<p role="status">Проверяем доступ…</p>
    {:else if billing.canActivateStarter}
      <p>Попробуйте улучшение текста бесплатно.</p>
      <button
        class="primary"
        disabled={value.termsAccepted && value.selected === null}
        onclick={() => (value.termsAccepted ? activate(value.selected) : (dismissed = false))}
        >Включить</button
      >
      {#if value.termsAccepted && value.selected === null}<p>Выберите стиль на главном экране.</p>{/if}
    {:else}
      <p>Выберите подписку, чтобы продолжить улучшать текст.</p>
      <button class="primary" onclick={onplans}>Посмотреть тарифы</button>
    {/if}
  </section>
{/if}
