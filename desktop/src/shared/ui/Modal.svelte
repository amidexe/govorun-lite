<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  let {
    title,
    children,
    actions,
    onclose,
  }: { title: string; children: Snippet; actions: Snippet; onclose: () => void } = $props();
  let element: HTMLDialogElement;
  const id = $props.id();
  onMount(() => {
    element.showModal();
    return () => element.close();
  });
</script>

<dialog
  bind:this={element}
  aria-labelledby={id}
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
>
  <h2 {id}>{title}</h2>
  <div class="content">{@render children()}</div>
  <div class="actions">{@render actions()}</div>
</dialog>

<style>
  dialog {
    background: var(--surface);
    color: var(--text);
    border: 1px solid var(--stroke);
    border-radius: 18px;
    padding: 22px;
    width: min(430px, calc(100vw - 32px));
    max-height: calc(100dvh - 32px);
    box-shadow: 0 20px 70px #0005;
    animation: dialog-enter var(--motion) var(--ease) both;
  }
  dialog::backdrop {
    background: #0006;
  }
  h2 {
    margin: 0 0 12px;
    font-size: 16px;
  }
  .content {
    line-height: 1.5;
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 22px;
  }
</style>
