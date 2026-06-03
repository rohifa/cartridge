<script lang="ts">
  import { PLAT } from '$lib/constants.js';
  import type { Platform } from '$lib/types.js';

  let {
    platform,
    size = 8,
    ring = true,
  }: {
    platform: Platform;
    size?: number;
    ring?: boolean;
  } = $props();
</script>

<span style="
  width: {size}px;
  height: {size}px;
  border-radius: 50%;
  flex: none;
  background: {PLAT[platform].color};
  display: inline-block;
  box-shadow: {ring ? '0 0 0 2px oklch(0 0 0/0.35)' : 'none'};
"></span>
