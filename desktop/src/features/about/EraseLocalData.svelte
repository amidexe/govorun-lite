<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import Modal from '../../shared/ui/Modal.svelte';
  const api = useDesktopApi();
  let confirming = $state(false),
    busy = $state(false),
    failed = $state(false);
  async function erase() {
    if (busy) return;
    busy = true;
    failed = false;
    try {
      // The host closes the application and removes the data once it has exited.
      await api.invoke('data.erase', { confirmed: true });
    } catch {
      failed = true;
      confirming = false;
    } finally {
      busy = false;
    }
  }
</script>

<section class="surface">
  <h2>Удаление с этого Mac</h2>
  <p>
    Приложение, его настройки, словарь, статистика, вход в аккаунт, разрешения и модель
    распознавания будут удалены с этого компьютера. Аккаунт и данные на сервере останутся.
  </p>
  <button
    class="danger quiet"
    disabled={busy}
    onclick={() => {
      failed = false;
      confirming = true;
    }}>Удалить Говорун с этого Mac</button
  >
  {#if failed}<p class="error" role="alert">Не удалось удалить приложение. Попробуйте ещё раз.</p>{/if}
</section>
{#if confirming}<Modal
    title="Удалить Говорун с этого Mac?"
    onclose={() => {
      if (!busy) confirming = false;
    }}
  >
    <p>Приложение закроется и будет перенесено в Корзину вместе со своими данными. Чтобы установить его снова, понадобится новый установщик.</p>
    <p>
      Чтобы освободить место в списке устройств аккаунта, сначала выйдите из аккаунта или уберите
      это устройство в разделе «Аккаунт». Данные удаляются сразу и безвозвратно.
    </p>
    {#snippet actions()}<button
        class="quiet"
        disabled={busy}
        onclick={() => {
          confirming = false;
        }}>Отмена</button
      >
      <button class="danger" disabled={busy} onclick={erase}>Удалить и закрыть</button
      >{/snippet}
  </Modal>{/if}

<style>
  h2 {
    margin: 0 0 12px;
  }
  p {
    margin: 12px 0;
  }
</style>
