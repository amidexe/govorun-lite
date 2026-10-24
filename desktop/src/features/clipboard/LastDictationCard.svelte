<script lang="ts">
  import { useDesktopApi } from '../../shared/state/desktop-context';
  import { useNativeSnapshot } from '../../shared/state/native-snapshot.svelte';
  import Icon from '../../shared/ui/Icon.svelte';
  import { slide } from 'svelte/transition';
  import { motionDuration } from '../../motion';
  import { lastDictationSnapshot } from './model';
  // The «Статистика» card with other content: the same header, expander and footer rules; ordinary
  // quiet buttons copy. Shown only while «Помнить последнюю диктовку» is on and a dictation happened.
  let { phase }: { phase?: string } = $props();
  const api = useDesktopApi();
  // clipboard.changed carries the settings, not the text, so it only triggers a re-read.
  const last = useNativeSnapshot(
    'lastDictation.snapshot',
    lastDictationSnapshot,
    'clipboard.changed',
  );
  $effect(() => {
    if (phase === 'idle') void last.refresh();
  });
  let improved = $derived(last.value?.text ?? null);
  let original = $derived(last.value?.original ?? null);
  let expanded = $state(false),
    copied = $state<'improved' | 'original' | null>(null),
    cut = $state({ improved: false, original: false });
  // Six lines at most; a longer text says so, the copy is always whole.
  function measure(node: HTMLElement, which: 'improved' | 'original') {
    const check = () => {
      cut = { ...cut, [which]: node.scrollHeight > node.clientHeight + 1 };
    };
    const observer = new ResizeObserver(check);
    observer.observe(node);
    check();
    return { destroy: () => observer.disconnect() };
  }
  const forms: Record<string, string> = {
    one: 'слово',
    few: 'слова',
    many: 'слов',
    other: 'слова',
  };
  function words(text: string) {
    const count = text.split(/\s+/).filter(Boolean).length;
    return `${count.toLocaleString('ru-RU')} ${forms[new Intl.PluralRules('ru').select(count)]}`;
  }
  async function copy(which: 'improved' | 'original') {
    try {
      const done = (await api.invoke('lastDictation.copy', { original: which === 'original' }))
        .copied;
      copied = done ? which : null;
    } catch {
      copied = null;
    }
    if (copied) setTimeout(() => (copied = null), 1500);
  }
</script>

{#if improved}
  <section class="surface last-dictation">
    <button
      class="expander"
      aria-expanded={expanded}
      aria-controls="last-dictation"
      onclick={() => {
        expanded = !expanded;
      }}
    >
      <span class:expanded class="chevron"><Icon name="ph_caret_right" size={14} /></span><Icon
        name="ph_microphone"
      />
      <strong>Последняя диктовка · {words(improved)}</strong>
    </button>
    <button
      class="quiet icon-button quick"
      aria-label="Скопировать"
      title="Скопировать"
      onclick={() => copy('improved')}
      ><Icon name={copied === 'improved' ? 'ph_check' : 'ph_copy'} size={16} /></button
    >
    {#if expanded}<div
        id="last-dictation"
        class="expanded-content"
        transition:slide={{ duration: motionDuration() }}
      >
        {#if original}<div class="version">
            <p>Улучшенный · {words(improved)}</p>
            <button
              class="quiet icon-button"
              aria-label="Скопировать улучшенный"
              title="Скопировать улучшенный"
              onclick={() => copy('improved')}
              ><Icon name={copied === 'improved' ? 'ph_check' : 'ph_copy'} size={16} /></button
            >
          </div>{/if}
        <p class="dictation selectable" use:measure={'improved'}>{improved}</p>
        {#if cut.improved}<p class="note">Показано начало · копируется весь текст</p>{/if}
        {#if original}<div class="footer">
            <div class="version">
              <p>Оригинал · {words(original)}</p>
              <button
                class="quiet icon-button"
                aria-label="Скопировать оригинал"
                title="Скопировать оригинал"
                onclick={() => copy('original')}
                ><Icon name={copied === 'original' ? 'ph_check' : 'ph_copy'} size={16} /></button
              >
            </div>
            <p class="dictation selectable" use:measure={'original'}>{original}</p>
            {#if cut.original}<p class="note">Показано начало · копируется весь текст</p>{/if}
          </div>{/if}
      </div>{/if}
  </section>
{/if}

<style>
  /* The rules of HomeStatistics; only the six-line limit is this card's own. */
  .last-dictation {
    position: relative;
    padding: 0;
    overflow: hidden;
  }
  /* On the header row, centred like the settings gear beside the title. */
  .quick {
    position: absolute;
    top: 10px;
    right: 10px;
  }
  .expanded-content {
    padding: 14px;
    border-top: 1px solid var(--stroke);
  }
  .footer {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--stroke);
  }
  .footer p,
  .note,
  .dictation {
    color: var(--secondary);
    font-size: 12px;
  }
  .dictation {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 6;
    line-clamp: 6;
    overflow: hidden;
    white-space: pre-wrap;
  }
  /* A version label with its copy icon, the same icon button as on the header row. */
  .version {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: -6px -4px 2px 0;
  }
  .chevron {
    font-size: 21px;
    line-height: 1;
    transition: transform 0.12s;
  }
  .expanded {
    transform: rotate(90deg);
  }
</style>
