<script lang="ts">
  let { name, size = 18 }: { name: string; size?: number } = $props();
</script>

<span
  aria-hidden="true"
  style={`--icon: url('${new URL(`${import.meta.env.BASE_URL}generated/${name}.svg`, document.baseURI).href}'); width: ${size}px; height: ${size}px`}
></span>

<style>
  span {
    display: inline-block;
    flex-shrink: 0;
    background: currentColor;
    mask: var(--icon) center / contain no-repeat;
  }
</style>
