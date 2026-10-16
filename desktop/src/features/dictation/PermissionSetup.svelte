<script lang="ts">
  import type { DictationSnapshot } from './model';
  import Icon from '../../shared/ui/Icon.svelte';
  let {
    permissions,
    busy,
    onopen,
    onrefresh,
  }: {
    permissions: DictationSnapshot['permissions'];
    busy: boolean;
    onopen: (permission: 'microphone' | 'accessibility') => void;
    onrefresh: () => Promise<void>;
  } = $props();
  $effect(() => {
    if (permissions.microphone === 'authorized' && permissions.accessibility) return;
    const timer = window.setInterval(() => {
      if (document.visibilityState === 'visible') void onrefresh();
    }, 2000);
    return () => window.clearInterval(timer);
  });
  let microphoneDetail = $derived(
    permissions.microphone === 'authorized'
      ? 'Запись голоса разрешена'
      : permissions.microphone === 'notDetermined'
        ? 'Нужно разрешить запись голоса'
        : permissions.microphone === 'restricted'
          ? 'Доступ ограничен системой. Проверьте системные ограничения.'
          : 'Разрешите запись голоса в системных настройках.',
  );
</script>

<div class="permission-row">
  <span class:success={permissions.microphone === 'authorized'}
    ><Icon
      name={permissions.microphone === 'authorized'
        ? 'ph_check_circle_fill'
        : 'ph_warning_circle_fill'}
    /></span
  >
  <div>
    <strong>Микрофон</strong>
    <p>{microphoneDetail}</p>
    {#if permissions.microphone !== 'authorized'}<button
        disabled={busy}
        onclick={() => onopen('microphone')}
        >{permissions.microphone === 'notDetermined'
          ? 'Разрешить микрофон'
          : 'Открыть настройки микрофона'}</button
      >{/if}
  </div>
</div>
<div class="permission-row">
  <span class:success={permissions.accessibility}
    ><Icon
      name={permissions.accessibility ? 'ph_check_circle_fill' : 'ph_warning_circle_fill'}
    /></span
  >
  <div>
    <strong>Специальные возможности</strong>
    <p>
      {permissions.accessibility
        ? 'Доступ разрешён'
        : 'Для горячей клавиши и вставки текста. Разрешите Говоруну универсальный доступ и мониторинг ввода в системных настройках.'}
    </p>
    {#if !permissions.accessibility}<button disabled={busy} onclick={() => onopen('accessibility')}
        >Открыть специальные возможности</button
      >{/if}
  </div>
</div>

<style>
  .permission-row {
    display: flex;
    gap: 10px;
    padding: 12px 0;
  }
  .permission-row + .permission-row {
    border-top: 1px solid var(--stroke);
  }
  .permission-row p {
    margin-top: 4px;
  }
  .permission-row button {
    margin-top: 8px;
  }
</style>
