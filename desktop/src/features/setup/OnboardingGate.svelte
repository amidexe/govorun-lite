<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { RuntimeSnapshot } from '../dictation/runtime';
  import { useSetup } from './state.svelte';
  import { initialSetup } from './model';
  import SetupWizard from './SetupWizard.svelte';
  import { statisticsSnapshot } from '../statistics/model';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Modal from '../../shared/ui/Modal.svelte';
  let {
    runtime,
    failure,
    reconnecting,
    onreconnect,
    children,
  }: {
    runtime: RuntimeSnapshot | null;
    failure: boolean;
    reconnecting: boolean;
    onreconnect: () => void;
    children: Snippet;
  } = $props();
  const setup = useSetup();
  const stats = useNativeSnapshot('statistics.snapshot', statisticsSnapshot, 'statistics.changed');
  let migrating = $state(false),
    migrationAttempted = $state(false),
    resetVisible = $state(false);
  async function preserveExisting() {
    migrating = true;
    migrationAttempted = true;
    await setup.save({ ...initialSetup, step: 'done' });
    migrating = false;
  }
  // An earlier dictation marks an existing installation; the host does not report it.
  let history = $derived(
    runtime?.hasLocalDictation ?? (stats.value ? stats.value.total.sessions > 0 : null),
  );
  $effect(() => {
    if (
      !failure &&
      !setup.saved.failed &&
      setup.saved.value?.progress === null &&
      history === true &&
      !migrationAttempted
    )
      void preserveExisting();
  });
  let finished = $derived(setup.saved.value?.progress?.step === 'done');
  // Inspect-only workers have no readiness and deliberately cannot reveal first-run state.
  // An unreadable history keeps the app usable instead of blocking it behind setup.
  let inspector = $derived(
    !!runtime &&
      setup.saved.value?.progress === null &&
      (runtime.readiness == null || (history === null && stats.failed)),
  );
</script>

{#if finished || inspector}
  {@render children()}
{:else if setup.saved.failed || failure}
  <main class="setup-recovery surface">
    <h1>Продолжим настройку</h1>
    <p role="alert">
      {failure
        ? 'Не удалось включить диктовку. Попробуйте ещё раз.'
        : 'Не удалось прочитать сохранённый шаг настройки.'}
    </p>
    <button
      class="primary"
      disabled={reconnecting}
      onclick={() => {
        if (failure) onreconnect();
        else void setup.saved.refresh();
      }}>Попробовать ещё раз</button
    >
    {#if !failure}<button
        onclick={() => {
          resetVisible = true;
        }}>Начать настройку заново</button
      >{/if}
    {#if setup.error}<p class="error" role="alert">{setup.error}</p>{/if}
  </main>
{:else if setup.saved.loading || !runtime || migrating || history === null}
  <main class="setup-recovery surface"><p role="status">Подготавливаем Говорун…</p></main>
{:else if history === true && !setup.saved.value?.progress}
  <main class="setup-recovery surface">
    <p role="alert">{setup.error || 'Не удалось сохранить состояние настройки.'}</p>
    <button onclick={preserveExisting}>Попробовать ещё раз</button>
  </main>
{:else}
  <SetupWizard {setup} {runtime} />
{/if}
{#if resetVisible}<Modal
    title="Начать настройку заново?"
    onclose={() => {
      resetVisible = false;
    }}
  >
    <p>
      Сбросится только шаг приветствия. Аккаунт, статистика, словарь, стили и разрешения сохранятся.
    </p>
    {#snippet actions()}<button
        onclick={() => {
          resetVisible = false;
        }}>Отмена</button
      ><button
        class="primary"
        disabled={setup.pending}
        onclick={async () => {
          await setup.reset();
          migrationAttempted = true;
          resetVisible = false;
        }}>Начать заново</button
      >{/snippet}
  </Modal>{/if}

<style>
  .setup-recovery {
    margin: 14px;
    display: grid;
    gap: 12px;
  }
</style>
