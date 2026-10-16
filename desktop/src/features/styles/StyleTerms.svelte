<script lang="ts">
  import type { StylesSnapshot } from './model';
  import StyleBetaNotice from './StyleBetaNotice.svelte';
  let {
    selection = 'formal',
    busy,
    error,
    canCancel = true,
    onclose,
    onaccept,
    ondocuments,
  }: {
    selection?: string;
    busy: boolean;
    error: string | null;
    canCancel?: boolean;
    onclose: () => void;
    onaccept: (selection: string, options?: StylesSnapshot['options']) => void;
    ondocuments: () => void;
  } = $props();
</script>

<section class="surface activation" aria-label="Включение улучшения текста" aria-busy={busy}>
  <h2>Включить улучшение текста?</h2>
  <StyleBetaNotice />
  <ul class="privacy-facts">
    <li>Аудио остаётся на устройстве.</li>
    <li>
      Проверка доступа и обработка текста разделены. Вместе с текстом не передаются имя, аккаунт,
      данные оплаты или устройства.
    </li>
    <li>Говорун не сохраняет исходный текст и полученный ответ.</li>
    <li>Не отправляйте пароли, платёжные реквизиты и другие секреты.</li>
  </ul>

  <p class="legal">
    Нажимая «Принять и включить», вы принимаете
    <button class="text-link" onclick={ondocuments}>Условия и политика</button>.
  </p>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="quiet" disabled={!canCancel} onclick={onclose}>Не сейчас</button>
    <button class="primary" disabled={busy} onclick={() => onaccept(selection)}
      >{busy ? 'Подключаем…' : 'Принять и включить'}</button
    >
  </div>
</section>

<style>
  .legal {
    color: var(--secondary);
    line-height: 1.55;
  }
  .text-link {
    display: inline;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-decoration: underline;
    text-align: left;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 16px;
  }
</style>
