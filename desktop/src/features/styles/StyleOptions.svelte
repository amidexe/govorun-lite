<script lang="ts">
  import type { StylesActions } from '../../ipc/actions';
  import { choice } from '../../shared/forms/choice';
  import type { StylesSnapshot } from './model';
  let {
    options,
    busy,
    onchange,
    full = false,
  }: {
    options: StylesSnapshot['options'];
    busy: boolean;
    full?: boolean;
    onchange: (change: StylesActions['option']) => Promise<unknown>;
  } = $props();
  async function change(
    control: HTMLInputElement | HTMLSelectElement,
    update: StylesActions['option'],
  ) {
    await onchange(update);
    const name = update.name;
    if (control instanceof HTMLInputElement) control.checked = options[name] === true;
    else control.value = String(options[name]);
  }
</script>

<label class="setting-row"
  ><span>Эмодзи<small>Добавлять эмодзи по смыслу текста</small></span>
  <select
    value={options.emojiMode}
    disabled={busy}
    onchange={(event) =>
      change(event.currentTarget, {
        name: 'emojiMode',
        value: choice(event.currentTarget.value, [
          'off',
          'restrained',
          'lively',
          'bright',
        ] as const),
      })}
    ><option value="off">Выкл.</option><option value="restrained">Сдержанно</option><option
      value="lively">Живо</option
    ><option value="bright">Ярко</option></select
  >
</label>
<label class="setting-row"
  ><span>Меньше пунктуации<small>Делает пунктуацию проще и реже, как в живой переписке</small></span
  ><input
    type="checkbox"
    role="switch"
    checked={options.reducedPunctuation}
    disabled={busy}
    onchange={(event) =>
      change(event.currentTarget, {
        name: 'reducedPunctuation',
        value: event.currentTarget.checked,
      })}
  /></label
>
{#if full}<label class="setting-row"
    ><span>Убирать ненормативную лексику<small>Заменять грубые слова нейтральными</small></span
    ><input
      type="checkbox"
      role="switch"
      checked={options.profanityFilter}
      disabled={busy}
      onchange={(event) =>
        change(event.currentTarget, {
          name: 'profanityFilter',
          value: event.currentTarget.checked,
        })}
    /></label
  >{/if}

<style>
  .setting-row + .setting-row {
    border-top: 1px solid var(--stroke);
  }
  .setting-row {
    padding: 8px 0;
  }
</style>
