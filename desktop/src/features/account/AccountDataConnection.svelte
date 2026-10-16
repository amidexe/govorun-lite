<script lang="ts">
  let { busy, act }: { busy: boolean; act: (action: 'merge' | 'keepLocal') => void } = $props();
</script>

<div class="data-connection">
  <h2>Объединить данные с аккаунтом?</h2>
  <p>Статистика, словарь и свои стили — на всех ваших устройствах.</p>
  <div class="actions">
    <button class="primary" disabled={busy} onclick={() => act('merge')}>Объединить</button>
    <button class="secondary" disabled={busy} onclick={() => act('keepLocal')}
      >Оставить на устройстве</button
    >
  </div>
</div>

<style>
  .data-connection {
    display: grid;
    gap: 8px;
  }
  h2,
  p {
    margin: 0;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
</style>
