<script lang="ts">
  import type { RuntimeSnapshot } from '../dictation/runtime';
  import type { useSetup } from './state.svelte';
  import { initialSetup, dictationReady, type SetupStep } from './model';
  import SetupDictation from './SetupDictation.svelte';
  import Bird from '../../shared/ui/Bird.svelte';
  import ThemeChooser from '../appearance/ThemeChooser.svelte';
  let { setup, runtime }: { setup: ReturnType<typeof useSetup>; runtime: RuntimeSnapshot | null } =
    $props();
  let step = $derived((setup.saved.value?.progress ?? initialSetup).step);
  let ready = $derived(dictationReady(runtime));
  const labels = ['Знакомство', 'Подготовка'];
  const groups = { welcome: 0, dictation: 1, trial: 1, done: 1 };
  const titles = {
    welcome: 'Привет! Я — Говорун',
    dictation: 'Подготовка диктовки',
    trial: 'Подготовка диктовки',
    done: 'Готово',
  };
  let currentGroup = $derived(groups[step]);
  async function go(target: SetupStep) {
    if (!setup.pending) await setup.save({ step: target });
  }
</script>

<main class="wizard" aria-label="Первый запуск Говоруна">
  <header><span class="brand"><Bird size={28} />Говорун</span><ThemeChooser /></header>
  <ol class="progress" aria-label="Ход настройки">
    {#each labels as label, index}<li
        class:current={index === currentGroup}
        class:complete={index < currentGroup}
        aria-current={index === currentGroup ? 'step' : undefined}
      >
        <span>{index + 1}</span>{label}
      </li>{/each}
  </ol>
  <div class="scroll" data-setup-step={step}>
    {#key step}<div class="step-content">
        <div class="intro" class:welcome={step === 'welcome'}>
          {#if step === 'welcome'}<Bird size={88} />{/if}
          <h1>{titles[step]}</h1>
          {#if step === 'welcome'}<p>Голосовой ввод в ваших приложениях.</p>
            <p>Диктовка работает без интернета и аккаунта.</p>
          {:else if step === 'dictation'}<p>
              Проверьте разрешения и выберите удобную клавишу. Позже это можно изменить в
              настройках.
            </p>{/if}
        </div>
        {#if step === 'dictation'}<SetupDictation {runtime} />{/if}
      </div>{/key}
  </div>
  <footer>
    {#if setup.error}<p class="error" role="alert">{setup.error}</p>{/if}
    <div class="footer-actions">
      {#if step !== 'welcome'}<button
          disabled={setup.pending ||
            runtime?.phase === 'starting' ||
            runtime?.phase === 'recording' ||
            runtime?.phase === 'processing'}
          onclick={() => go('welcome')}>Назад</button
        >{/if}
      <span class="spacer"></span>
      <button
        class="primary"
        disabled={setup.pending || (step !== 'welcome' && !ready)}
        onclick={() => go(step === 'welcome' ? 'dictation' : 'done')}
        >{setup.pending ? 'Сохраняем…' : step === 'welcome' ? 'Начать' : 'Начать работу'}</button
      >
    </div>
  </footer>
</main>

<style>
  .wizard {
    height: 100dvh;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 18px 28px 12px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 17px;
    font-weight: 650;
  }
  .progress {
    list-style: none;
    display: flex;
    justify-content: center;
    gap: 32px;
    padding: 12px 24px 18px;
    margin: 0;
    border-bottom: 1px solid var(--stroke);
  }
  .progress li {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--secondary);
    font-size: 12px;
  }
  .progress span {
    border: 1px solid var(--stroke);
    border-radius: 50%;
    width: 23px;
    height: 23px;
    display: grid;
    place-items: center;
  }
  .progress .current {
    color: var(--text);
    font-weight: 600;
  }
  .current span {
    color: var(--accent-text);
    border-color: var(--accent);
  }
  .progress .complete span {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .scroll {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
    padding: 24px 32px;
  }
  .step-content {
    max-width: 640px;
    margin: 0 auto;
    animation: dialog-enter var(--motion) var(--ease) both;
  }
  .intro {
    margin-bottom: 22px;
  }
  h1 {
    font-size: 24px;
    margin: 0 0 10px;
    line-height: 1.25;
  }
  .intro p {
    line-height: 1.5;
  }
  .welcome {
    text-align: center;
    padding-top: 36px;
    display: grid;
    justify-items: center;
    gap: 12px;
  }
  .welcome h1 {
    margin-top: 16px;
  }
  footer {
    border-top: 1px solid var(--stroke);
    padding: 16px 28px;
    background: var(--surface);
  }
  .footer-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .spacer {
    flex: 1;
  }
  footer > p {
    margin-bottom: 10px;
  }
  @media (max-width: 500px) {
    header,
    footer {
      padding: 14px;
    }
    .scroll {
      padding: 18px 14px;
    }
    .progress {
      gap: 12px;
      padding: 10px 14px;
    }
    .progress li {
      font-size: 11px;
      gap: 4px;
    }
    .footer-actions {
      flex-wrap: wrap;
      gap: 8px;
    }
    .spacer {
      display: none;
    }
    .footer-actions button {
      flex: 1;
    }
    .welcome {
      padding-top: 12px;
    }
  }
</style>
