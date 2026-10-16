<script lang="ts">
  import bird from '../../assets/bird.svg?raw';
  let { size = 24 }: { size?: number } = $props();
</script>

<!-- Trusted bundled vector, shared with the native tray; never user-supplied markup. -->
<span class="bird" style:width="{size}px" style:height="{size}px" aria-hidden="true"
  >{@html bird}</span
>

<style>
  .bird {
    display: inline-flex;
    flex-shrink: 0;
  }
  .bird :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
