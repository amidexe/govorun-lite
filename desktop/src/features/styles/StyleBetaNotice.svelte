<script lang="ts">
  let { compact = false }: { compact?: boolean } = $props();
</script>

<p class:compact class="beta-notice">
  {compact
    ? 'Бета-версия · сервис тестируется'
    : 'Улучшение текста — бета-версия. Сервис тестируется, возможны ошибки. По итогам тестирования мы решим, продолжать ли его работу и на каких условиях. Документы редакции 2.1 собраны на одной странице. Правила обработки текста и оплаты сохранены; повторно принимать эту редакцию не нужно.'}
</p>

<style>
  .beta-notice {
    margin: 8px 0;
    color: var(--secondary);
    line-height: 1.55;
  }
  .compact {
    font-size: 12px;
  }
</style>
