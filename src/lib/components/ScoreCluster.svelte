<script lang="ts">
  import ScoreBadge from './ScoreBadge.svelte';
  import type { Game } from '$lib/types.js';

  let {
    game,
    size = 'sm',
  }: {
    game: Game;
    size?: 'sm' | 'md' | 'lg';
  } = $props();
</script>

<span style="display:inline-flex; gap:5px">
  <ScoreBadge value={game.metacritic} {size} />
  <ScoreBadge value={game.opencritic} {size} />
  <ScoreBadge value={game.userScore}  {size} />
</span>
