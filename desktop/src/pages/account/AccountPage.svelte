<script lang="ts">
  import { useAccount } from '../../features/account/state.svelte';
  import type { AccountDevice } from '../../features/account/model';
  import AccountDataConnection from '../../features/account/AccountDataConnection.svelte';
  import AccountStorageNotice from '../../features/account/AccountStorageNotice.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import AccountSubscription from './AccountSubscription.svelte';
  import Modal from '../../shared/ui/Modal.svelte';

  let {
    connectionOnly = false,
    embedded = false,
    subscriptionOpen = false,
    onconnected,
  }: { connectionOnly?: boolean; embedded?: boolean; subscriptionOpen?: boolean; onconnected?: () => void } = $props();

  const accountState = useAccount();
  let account = $derived(accountState.account.value);
  let busy = $derived(
    (accountState.account.failed && account?.storage.available !== false) ||
      accountState.pending ||
      account?.busy === true,
  );
  $effect(() => {
    if (connectionOnly && account?.status === 'signedIn' && !busy) onconnected?.();
  });
  let confirmation = $state<{
    scope: string;
    action: 'deleteAccount' | 'revokeDevice' | 'replaceDevice';
    title: string;
    body: string;
    device?: AccountDevice;
  } | null>(null);
  $effect(() => {
    if (
      confirmation &&
      (accountState.account.failed ||
        !account?.storage.available ||
        confirmation.scope !== account.scope)
    )
      confirmation = null;
  });
  function confirmDevice(device: AccountDevice) {
    if (!account) return;
    const replacing = account.replacingDevice;
    confirmation = {
      scope: account.scope,
      action: replacing ? 'replaceDevice' : 'revokeDevice',
      device,
      title: `${replacing ? 'Заменить' : 'Удалить'} «${device.name}»?`,
      body: device.confirmation,
    };
  }
  function completeConfirmation() {
    const pending = confirmation;
    confirmation = null;
    if (pending && !busy && pending.scope === account?.scope) {
      if (pending.action === 'deleteAccount')
        void accountState.act('deleteAccount', { confirmed: true });
      else if (pending.device)
        void accountState.act(pending.action, { confirmed: true, deviceId: pending.device.id });
    }
  }
</script>

{#if subscriptionOpen && !connectionOnly && account?.storage.available && account.status === 'signedIn'}<AccountSubscription
    expanded
  />
{:else}
  {#if accountState.account.failed && account?.storage.available !== false}<section
      class="surface notice"
      role="alert"
    >
      <p>Не удалось загрузить аккаунт.</p>
      <button onclick={() => accountState.account.refresh()}>Попробовать ещё раз</button>
    </section>{/if}
  {#if account}
    {#if !account.storage.available}
      <AccountStorageNotice
        storage={account.storage}
        scope={account.scope}
        {busy}
        onrecover={() => accountState.act('recoverStorage')}
        onrestart={() => accountState.act('restartStorage', { confirmed: true })}
      />
    {:else}
      <section class="account-card" class:surface={!embedded}>
        <h2>{connectionOnly ? 'Вход в аккаунт' : 'Аккаунт и синхронизация'}</h2>
        <div class="account-row">
          <Icon name="ph_user_circle" />
          <div class="identity">
            <strong
              >{account.status === 'signedIn'
                ? account.displayName
                : 'Говорун на всех устройствах'}</strong
            >
            <p>
              {account.status === 'signedIn'
                ? 'Устройства, статистика, словарь и свои стили'
                : connectionOnly
                  ? 'Войдите, чтобы попробовать улучшение текста.'
                  : 'Статистика, словарь и свои стили — на всех ваших устройствах.'}
            </p>
          </div>
          {#if account.status === 'signedIn' && !connectionOnly}<button
              class="secondary"
              disabled={busy}
              onclick={() => accountState.act('signOut')}>Выйти</button
            >{/if}
        </div>
        {#if account.status === 'signingIn'}
          <p role="status">{account.busy ? 'Завершаем вход…' : 'Завершите вход в браузере'}</p>
          <button
            class="secondary"
            disabled={accountState.pending || accountState.account.failed}
            onclick={() => accountState.act('cancelSignIn')}>Отменить вход</button
          >
        {:else if account.status !== 'signedIn' && !account.replacingDevice}
          <div class="providers">
            {#each account.providers as provider}
              <button
                class="primary"
                disabled={busy || !provider.enabled || !account.storage.available}
                onclick={() => accountState.act('signIn', { provider: provider.id })}
                >{provider.id === 'yandex' ? 'Яндекс' : 'VK ID'}</button
              >
            {/each}
          </div>
          {#if account.providerNotice}<p>{account.providerNotice}</p>{/if}
          <p class="legal">
            Продолжая вход, вы принимаете
            <button class="text-link" onclick={() => accountState.act('openLink', { link: 'agreement' })}
              >Условия и политика</button>.
          </p>
        {/if}
        {#if account.message}<p role="status">{account.message}</p>{/if}
        {#if !connectionOnly && account.status === 'signedIn' && account.syncMode === 'pending'}
          <hr />
          <AccountDataConnection {busy} act={accountState.act} />
        {:else if account.status === 'signedIn' && account.syncMode === 'evaluating'}
          <p role="status">Проверяем данные аккаунта…</p>
        {:else if account.status === 'signedIn' && account.syncMode === 'localOnly'}
          <hr />
          <h3>Данные остаются на этом устройстве</h3>
          <p>Аккаунт и подписка работают без синхронизации.</p>
          <button class="secondary" disabled={busy} onclick={() => accountState.act('requestSync')}
            >Включить синхронизацию</button
          >
        {/if}
        {#if account.status === 'signedIn' && !connectionOnly}
          <hr />
          <div class="actions">
            <button class="secondary" disabled={busy} onclick={() => accountState.act('refresh')}
              >Обновить</button
            >
            <button
              class="secondary destructive"
              disabled={busy}
              onclick={() => {
                if (account)
                  confirmation = {
                    scope: account.scope,
                    action: 'deleteAccount',
                    title: 'Удалить аккаунт?',
                    body: 'Серверные данные, синхронизация и доступ к онлайн-функциям будут удалены. Локальная диктовка и данные на этом устройстве останутся.',
                  };
              }}>Удалить аккаунт</button
            >
          </div>
        {/if}
      </section>
      {#if !connectionOnly && account.status === 'signedIn'}<AccountSubscription />{/if}
      {#if (!connectionOnly && account.devices.length) || account.replacingDevice}<section
          class="surface devices"
        >
          <div class="section-heading">
            <h2>{account.replacingDevice ? 'Выберите устройство для замены' : 'Устройства'}</h2>
            <span>{account.devices.length} из {account.deviceLimit}</span>
          </div>
          {#if account.replacingDevice}<p>
              Выберите устройство, которое больше не используете. Новый вход займёт его место.
            </p>{/if}
          {#each account.devices as device (device.id)}<div class="device-row">
              <Icon name="ph_hard_drives" />
              <div class="identity">
                <strong>{device.name}</strong>
                <p>{device.subtitle}</p>
              </div>
              {#if account.replacingDevice || !device.current}
                <button
                  class="secondary"
                  disabled={busy}
                  aria-label={`${account.replacingDevice ? 'Заменить' : 'Удалить'} ${device.name}`}
                  onclick={() => confirmDevice(device)}
                  >{account.replacingDevice ? 'Заменить' : 'Удалить'}</button
                >
              {/if}
            </div>{/each}
          <p>Неактивные устройства освобождают место через 90 дней.</p>
          {#if account.replacingDevice}<button
              class="secondary"
              disabled={accountState.pending || accountState.account.failed}
              onclick={() => accountState.act('cancelSignIn')}>Отменить вход</button
            >{/if}
        </section>{/if}
    {/if}
  {:else if accountState.account.loading}<section class="surface">
      <p role="status">Загружаем аккаунт…</p>
    </section>{/if}
  {#if accountState.error}<p class="notice" role="alert">{accountState.error}</p>{/if}
  {#if confirmation}<Modal
      title={confirmation.title}
      onclose={() => {
        confirmation = null;
      }}
    >
      <p>{confirmation.body}</p>
      {#snippet actions()}
        <button
          onclick={() => {
            confirmation = null;
          }}>Отмена</button
        >
        <button class="danger" onclick={completeConfirmation}
          >{confirmation?.action === 'deleteAccount'
            ? 'Удалить аккаунт'
            : confirmation?.action === 'replaceDevice'
              ? 'Заменить'
              : 'Удалить'}</button
        >
      {/snippet}
    </Modal>{/if}
{/if}

<style>
  .account-card {
    display: grid;
    gap: 12px;
  }
  .account-row,
  .device-row {
    display: flex;
    gap: 12px;
    align-items: center;
    min-height: 52px;
  }
  .identity {
    flex: 1;
    min-width: 0;
  }
  .identity strong {
    overflow-wrap: anywhere;
  }
  .identity p {
    margin: 3px 0 0;
  }
  .providers {
    display: flex;
    gap: 8px;
  }
  .providers button {
    flex: 1;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .text-link {
    display: inline;
    padding: 0;
    min-height: 0;
    background: transparent;
    border: 0;
    border-radius: 0;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    text-align: left;
  }
  .legal {
    line-height: 1.6;
  }
  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .device-row {
    border-top: 1px solid var(--stroke);
    padding: 10px 0;
  }
  .devices > p {
    margin-top: 10px;
  }
  hr {
    width: 100%;
    border: 0;
    border-top: 1px solid var(--stroke);
  }
  .destructive {
    color: var(--danger, #d44848);
  }
</style>
