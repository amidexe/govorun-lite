<script lang="ts">
  import { onMount } from 'svelte';
  import bird from '../../assets/bird.svg';
  import {
    acceptIndicator,
    indicatorLabel,
    indicatorPresentation,
    indicatorDot,
    type IndicatorFrame,
    type RecordingIndicatorApi,
  } from './indicator';
  let { api }: { api: RecordingIndicatorApi } = $props();
  let frame = $state<IndicatorFrame>({
    revision: 0,
    phase: 'idle',
    stage: 'recording',
    warning: false,
    issue: null,
  });
  const presentation = $derived(indicatorPresentation(frame.phase));
  let seconds = $state(0);
  function accept(value: unknown) {
    frame = acceptIndicator(frame, value);
  }
  onMount(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    const origin = performance.now();
    let animation: number;
    function animate(now: number) {
      seconds = (now - origin) / 1000;
      animation = requestAnimationFrame(animate);
    }
    animation = requestAnimationFrame(animate);
    void (async () => {
      const stop = await api.subscribe((payload) => {
        if (!disposed) accept(payload);
      });
      if (disposed) {
        stop();
        return;
      }
      unlisten = stop;
      const snapshot = await api.snapshot();
      if (!disposed) accept(snapshot);
    })().catch(() => {
      if (!disposed) frame = { ...frame, issue: 'captureUnavailable' };
    });
    return () => {
      disposed = true;
      cancelAnimationFrame(animation);
      unlisten?.();
    };
  });
  async function cancel() {
    try {
      accept(await api.cancel());
    } catch {
      frame = { ...frame, issue: 'captureUnavailable' };
    }
  }
</script>

<div
  class="capsule"
  class:warning={frame.warning}
  class:failure={frame.issue !== null || frame.phase === 'fallback'}
>
  <span class="logo"><img src={bird} alt="" /></span>
  <span
    class="dots"
    class:active={presentation.dots}
    role="status"
    aria-label={indicatorLabel(frame)}
  >
    {#each [0, 1, 2] as index}
      {@const dot = indicatorDot(seconds, index)}
      <i style:opacity={dot.opacity} style:transform={`scale(${dot.scale})`}></i>
    {/each}
  </span>
  <button
    onclick={cancel}
    disabled={!presentation.cancel}
    class:available={presentation.cancel}
    aria-label="Отменить запись"
    ><svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10"
      ><path
        d="M1 1 9 9M9 1 1 9"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
      /></svg
    ></button
  >
</div>

<style>
  :global(html),
  :global(body) {
    margin: 0;
    width: 112px;
    height: 44px;
    background: transparent;
    overflow: hidden;
  }
  .capsule {
    box-sizing: border-box;
    width: 112px;
    height: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 9px;
    border-radius: 22px;
    color: #555;
    background: #f3f3f3e8;
    border: 1px solid #5552;
    backdrop-filter: blur(18px);
    user-select: none;
  }
  .logo {
    width: 22px;
    height: 22px;
    display: grid;
    place-items: center;
  }
  img {
    width: 18px;
    height: 18px;
  }
  .dots {
    width: 22px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
  }
  i {
    display: block;
    width: 4.5px;
    height: 4.5px;
    background: currentColor;
    border-radius: 50%;
    opacity: 0.42;
  }
  .dots:not(.active) {
    visibility: hidden;
  }
  button {
    width: 22px;
    height: 22px;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    display: grid;
    place-items: center;
    opacity: 0.82;
    visibility: hidden;
    cursor: pointer;
  }
  button.available {
    visibility: visible;
  }
  .warning {
    color: #b57813;
  }
  .failure {
    animation: fallback-shake 464ms ease-in-out;
  }
  @keyframes fallback-shake {
    0%,
    25% {
      transform: translateX(-4.5px);
    }
    12.5%,
    37.5% {
      transform: translateX(4.5px);
    }
    50%,
    75% {
      transform: translateX(-2.5px);
    }
    62.5%,
    87.5% {
      transform: translateX(2.5px);
    }
    100% {
      transform: translateX(0);
    }
  }
  @media (prefers-color-scheme: dark) {
    .capsule {
      color: #fff;
      background: #4b4b4be8;
      border-color: #fff2;
    }
    img {
      filter: invert(1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .active i,
    .failure {
      animation: none;
      opacity: 0.7 !important;
      transform: none !important;
    }
  }
</style>
