<script lang="ts">
  import { toCanvas } from 'qrcode';
  import { validSBPBindingURL } from './payment-source';
  let { url }: { url: string } = $props();
  let canvas: HTMLCanvasElement | undefined = $state(),
    failed = $state(false);
  $effect(() => {
    if (!canvas) return;
    if (!validSBPBindingURL(url)) {
      failed = true;
      return;
    }
    let current = true;
    failed = false;
    void toCanvas(canvas, url, { width: 240, margin: 4, errorCorrectionLevel: 'M' }).catch(() => {
      if (current) failed = true;
    });
    return () => {
      current = false;
    };
  });
</script>

<section class="binding" aria-label="Привязка счёта СБП">
  <h3>Подключить СБП</h3>
  <canvas bind:this={canvas} hidden={failed} aria-label="QR-код для привязки счёта СБП"></canvas>
  {#if failed}<p role="alert">Не удалось показать QR-код. Обновите состояние подписки.</p>
  {:else}<p>
      Откройте приложение вашего банка и отсканируйте QR-код. Подтвердите привязку счёта для
      подписки.
    </p>
    <small>После подтверждения состояние обновится автоматически.</small>
  {/if}
</section>

<style>
  .binding {
    display: grid;
    justify-items: center;
    gap: 12px;
    text-align: center;
    margin-top: 16px;
  }
  canvas {
    max-width: 100%;
    border-radius: 8px;
  }
  .binding h3,
  .binding p {
    margin: 0;
  }
</style>
