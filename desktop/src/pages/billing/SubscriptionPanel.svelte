<script lang="ts">
  import { useDesktopNavigation } from '../../shared/state/desktop-navigation';
  import { promoProgramDescription } from '../../features/billing/promo-program';
  import type { useBilling } from '../../features/billing/state.svelte';
  import { useSubscription } from '../../features/billing/subscription.svelte';
  import PromoDialogs from '../../features/billing/PromoDialogs.svelte';
  import CheckoutConfirmation from '../../features/billing/CheckoutConfirmation.svelte';
  import CheckoutRecovery from '../../features/billing/CheckoutRecovery.svelte';
  import PaymentBindingQR from '../../features/billing/PaymentBindingQR.svelte';
  import StyleTerms from '../../features/styles/StyleTerms.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  import { subscriptionOverview } from '../../features/billing/subscription-overview';
  import { useStyles } from '../../features/styles/state.svelte';
  const navigate = useDesktopNavigation();
  const styles = useStyles();
  let { store }: { store: ReturnType<typeof useBilling> } = $props();
  const subscription = useSubscription(
    () => store,
    (request) => styles.act('acceptTerms', { ...request, confirmed: true }),
  );
  const controller = subscription.controller;
  let value = $derived(subscription.view.value);
  let selectedOffer = $derived(subscription.view.selectedOffer);
  let confirm = $derived(subscription.view.confirmation);
  let purchaseConfirmed = $derived(subscription.view.purchaseConfirmed);
  let program = $derived(
    value?.promoProgram?.visible &&
      !['paid', 'trial', 'plan_grant'].includes(value.accessType ?? '')
      ? value.promoProgram
      : null,
  );
  let overview = $derived(value ? subscriptionOverview(value) : null);
  // A late checkout success is no longer current after a confirmed refund.
  // Explicit action errors still remain visible in that state.
  let message = $derived(value?.status.state === 'refunded' && value.message === 'Подписка подключена'
    ? null : value?.message);
</script>

<button class="quiet back" onclick={() => navigate('account')}>← К аккаунту</button>
{#if store.billing.failed && !value}<section class="surface">
    <h2>Подписка и лимиты</h2>
    <button disabled={store.billing.refreshing} onclick={() => store.retry()}
      >Обновить</button
    >
  </section><section class="surface tariff-card"><p>Тарифы временно недоступны</p></section>{/if}
{#if value && value.signedIn}
  {#if confirm?.action === 'activateStarter' && value}<StyleTerms
      selection={styles.styles.value?.styles.some(
        (style) => style.id === styles.styles.value?.selected,
      )
        ? styles.styles.value!.selected ?? 'formal'
        : 'formal'}
      busy={store.busy}
      error={store.error ?? value.message}
      canCancel={!store.busy}
      onclose={() => {
        controller.closeConfirmation();
      }}
      ondocuments={() => store.act('legal', { document: 'onlineTerms' })}
      onaccept={(selection, activationOptions) =>
        controller.activateStarter(selection, activationOptions)}
    />
  {:else if value.promoAccess?.kind === 'acceptTerms'}
    <StyleTerms
      selection={styles.styles.value?.selected ?? 'formal'}
      busy={store.busy || styles.busy}
      error={styles.error ?? store.error}
      canCancel={!store.busy && !styles.busy}
      onclose={() => {
        void controller.dismissPromoAccess();
      }}
      ondocuments={() => store.act('legal', { document: 'onlineTerms' })}
      onaccept={(selection) => {
        void controller.acceptPromoTerms(selection);
      }}
    />
  {:else}
    <section class="surface subscription-overview" aria-busy={store.busy}>
      <div class="heading">
        <h2>
          {value.status.pendingCheckout
            ? 'Подключение подписки'
            : program
              ? 'Промо'
              : overview?.title}
        </h2>
        {#if !value.status.pendingCheckout}<button
            disabled={store.busy}
            onclick={() => store.retry()}>{store.refreshingByUser ? 'Обновляем…' : 'Обновить'}</button
          >{/if}
      </div>
      {#if program}
        {#if promoProgramDescription(program)}<p>{promoProgramDescription(program)}</p>{/if}
        {#if program.message}<p role="status">{program.message}</p>{/if}
        {#if program.state === 'active' && !program.enrollmentOpen && !program.participated}<p>
            Набор участников временно закрыт
          </p>{/if}
      {/if}
      {#if overview?.status && !value.status.pendingCheckout}<p class="subscription-status">
          {overview.status}
        </p>{/if}
      {#if overview?.payment && !value.status.pendingCheckout}<p>{overview.payment}</p>{/if}
      <p class="refresh-status" role="status">
        {value.status.pendingCheckout
          ? (store.error ?? value.message ?? 'Ожидаем завершения оплаты в банке.')
          : (store.error ?? message ??
              (store.refreshingByUser ? 'Обновляем сведения о подписке…' : null) ??
              (purchaseConfirmed
                ? 'Подписка оформлена. Можно пользоваться.'
                : value.status.state === 'refunded'
                  ? ''
                  : null) ??
              '')}
      </p>
      {#if value.promoAccess?.kind === 'refresh'}
        <button
          disabled={store.busy}
          onclick={() => {
            void controller.refreshPromoAccess();
          }}>Проверить доступ</button
        >
      {/if}
      {#if value.status.bindingUrl}{#key value.status.bindingUrl}<PaymentBindingQR
            url={value.status.bindingUrl}
          />{/key}{/if}
      {#if value.status.pendingCheckout}<CheckoutRecovery
          expiresAt={value.status.checkoutExpiresAt}
          canResume={value.status.canResumeCheckout && !value.status.bindingUrl}
          failed={store.billing.failed}
          busy={store.busy}
          canCancel={value.status.canRestartBinding}
          oncancel={() => {
            void store.act('restartBinding');
          }}
          onresume={() => {
            void store.act('resumeCheckout');
          }}
        />{/if}
      {#if value.period && overview?.showBalances}<p>{value.period}</p>{/if}
      {#if value.balances.length && overview?.showBalances}<div class="balances">
          {#each value.balances as balance}<div>
              <strong>{balance.title}</strong>
              <p class="balance-value">{balance.value}</p>
              {#if balance.percent != null}<progress max="100" value={balance.percent}
                  aria-label={balance.title} aria-valuetext={balance.value}></progress>{/if}
              {#if balance.detail}<small>{balance.detail}</small>{/if}
            </div>{/each}
        </div>{/if}
      {#if value.canActivateStarter}
        <button
          class="primary starter"
          disabled={store.busy || styles.styles.failed || !styles.styles.value}
          onclick={() =>
            styles.styles.value?.termsAccepted && styles.styles.value.selected !== null
              ? controller.activateWithAcceptedTerms(styles.styles.value.selected)
              : controller.requestConfirmation('activateStarter')}
          >{program ? 'Подключить' : 'Попробовать бесплатно'}</button
        >
        {#if styles.styles.failed}<p role="alert">Не удалось загрузить стили.</p>
          <button onclick={() => styles.styles.refresh()}>Загрузить стили</button>{/if}
      {/if}
    </section>
    <section class="surface tariff-card">
      {#if value.showPlanIntro}<div class="plan-intro">
        <h2>Продолжите улучшать текст</h2>
        <p>Выберите тариф. Распознавание речи работает офлайн, а подписка нужна только для улучшения текста.</p>
      </div>{/if}
      {#if value.showPlans}
      {#if value.storefront?.state === 'unavailable'}<p role="status">
          Тарифы временно недоступны
        </p>
      {:else if value.plans.length}<div class="plans">
          {#each value.plans as plan (plan.id)}<button
              class:selected={selectedOffer === plan.id}
              aria-pressed={selectedOffer === plan.id}
              onclick={() => {
                controller.selectOffer(plan.id);
              }}
            >
              <span
                ><strong>{plan.title} {plan.multiplier ?? ''}</strong><small>{plan.limit}</small
                ></span
              ><span class="price"
                >{#if plan.trial}<strong>0 ₽</strong><s>{plan.price}</s>{:else}<strong
                    >{plan.price}</strong
                  ><small>{plan.period}</small>{/if}</span
              >
              {#if plan.trial}<span class="trial"
                  ><strong>{plan.trial}</strong><small>Затем {plan.price} {plan.period}.</small
                  ></span
                >{/if}
            </button>{/each}
        </div>
        <button
          class="primary plan-action"
          disabled={store.busy ||
            !value.canPreviewCheckout ||
            !selectedOffer ||
            value.plans.find((item) => item.id === selectedOffer)?.current}
          onclick={() => store.act('previewCheckout', { offerId: selectedOffer })}
          >{value.plans.find((item) => item.id === selectedOffer)?.current
            ? 'Текущий тариф'
            : value.planAction}</button
        >
      {:else}<p role="status">Тарифы временно недоступны</p>{/if}
      {#if value.plans.length && value.storefront?.state !== 'unavailable'}<p class="cancel-disclosure">
          {value.planDisclosure ?? 'Отменить подписку можно в любое время.'}
        </p>{/if}
      {#if value.storefront?.state !== 'unavailable' && value.plans.length && value.catalogState !== 'error' && !value.canPreviewCheckout}<p
          class="availability"
        >
          Оформление подписки сейчас недоступно.
        </p>{/if}
      {/if}
    </section>
    <div class="promo-entry">
      <button
        class="quiet promo"
        disabled={store.busy}
        onclick={() => {
          controller.setPromoEntry(true);
        }}>Ввести промокод</button
      >
    </div>
    {#if value.showPaymentDetails}
      <section class="surface">
        <h2>Способ оплаты и списания</h2>
        <div class="details">
          <strong
            >{value.status.pendingCheckout ? 'Незавершённая оплата' : value.status.title}</strong
          >
          <p>
            {value.status.pendingCheckout
              ? (value.message ?? 'Ожидаем завершения оплаты в банке.')
              : value.status.loading && !value.status.fresh
                ? 'Проверяем способ оплаты и списания…'
                : value.status.payment}
          </p>
          {#if value.status.periodEnd}<p>Текущий период — до {value.status.periodEnd}</p>{/if}
          {#if value.status.renewalNotice}<p>{value.status.renewalNotice}</p>{/if}
          {#if value.status.card}<p>{value.status.card}</p>{/if}
          {#if !value.status.fresh && !value.status.pendingCheckout}<button
              disabled={store.busy}
              onclick={() => store.retry()}>Проверить состояние</button
            >{/if}
          <div class="actions">
            {#if value.status.canCancelChange}<button
                disabled={store.busy}
                onclick={() => controller.requestConfirmation('cancelPlanChange')}
                >Отменить смену тарифа</button
              >{/if}
            {#if value.status.canRemoveCard}<button
                disabled={store.busy}
                onclick={() => controller.requestConfirmation('removeCard')}
                >Отключить способ оплаты</button
              >{/if}
            {#if value.status.canCancelRenewal}<button
                disabled={store.busy}
                onclick={() => controller.requestConfirmation('cancelRenewal')}
                >Отменить подписку</button
              >{/if}
          </div>
          <button class="quiet support" disabled={store.busy} onclick={() => store.act('support')}
            >Написать в поддержку</button
          >
        </div>
      </section>
    {/if}
  {/if}
{:else if store.billing.loading}<p role="status">Загружаем подписку…</p>{/if}
{#if value?.checkout}{#key value.scope + value.checkout.id}<CheckoutConfirmation
      preview={value.checkout}
      busy={store.busy}
      message={store.error ?? value.checkout.message ?? null}
      onclose={() => store.act('dismissPreview')}
      onconfirm={(email) => {
        if (value?.checkout)
          void store.act('confirmCheckout', {
            previewId: value.checkout.id,
            email,
            confirmed: true,
          });
      }}
      onmethod={(method) => {
        if (value?.checkout)
          void store.act('choosePaymentMethod', { previewId: value.checkout.id, method });
      }}
      onlegal={(document) => store.act('legal', { previewId: value?.checkout?.id, document })}
    />{/key}{/if}
<PromoDialogs
  bind:entryOpen={() => subscription.view.promoEntry, controller.setPromoEntry}
  bind:code={() => subscription.view.code, controller.setCode}
  offerId={selectedOffer}
  promo={value?.promo ?? null}
  message={value?.message ?? null}
  busy={store.busy}
  error={store.error}
  act={store.act}
/>
{#if confirm && confirm.action !== 'activateStarter'}<Modal
    title={confirm.title}
    onclose={() => {
      controller.closeConfirmation();
    }}
  >
    <p>{confirm.body}</p>
    {#snippet actions()}<button
        disabled={store.busy}
        onclick={() => {
          controller.closeConfirmation();
        }}>Отмена</button
      ><button class="danger" disabled={store.busy} onclick={() => controller.confirm()}
        >{confirm?.actionTitle}</button
      >{/snippet}
  </Modal>{/if}

<style>
  .subscription-status {
    font-weight: 600;
    margin-top: 12px;
  }
  .balance-value {
    font-size: 20px;
    font-weight: 600;
  }
  .back {
    margin: -8px 0 12px;
    padding-left: 0;
  }
  .heading,
  .actions {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }
  .heading {
    justify-content: space-between;
  }
  .heading h2 {
    margin: 0;
  }
  .refresh-status {
    min-height: 20px;
    margin-top: 8px;
  }
  .heading > button {
    min-width: 108px;
  }
  .balances {
    display: flex;
    gap: 24px;
    margin-top: 16px;
  }
  .balances progress {
    width: 100%;
    height: 6px;
    accent-color: var(--accent);
  }
  .balances > div {
    flex: 1;
  }
  .details {
    display: grid;
    gap: 10px;
    justify-items: start;
  }
  .plan-intro {
    text-align: center;
  }
  .plans {
    display: grid;
    gap: 10px;
  }
  .plans > button {
    display: grid;
    grid-template-columns: 1fr auto;
    text-align: left;
    padding: 14px;
  }
  .plans > button.selected {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .price {
    text-align: right;
  }
  .trial {
    grid-column: 1/-1;
    color: var(--accent-text);
  }
  .trial small {
    color: var(--secondary);
  }
  .price s {
    display: block;
    color: var(--secondary);
  }
  .cancel-disclosure {
    text-align: center;
    margin-top: 12px;
    color: var(--secondary);
  }
  .plan-action,
  .starter {
    margin-top: 14px;
  }
  .plan-action {
    width: 100%;
  }
  .support,
  .promo {
    color: var(--accent-text);
    padding-left: 0;
  }
  .promo,
  .availability {
    margin-top: 12px;
  }
  .promo-entry {
    margin: 0 0 14px;
  }
</style>
