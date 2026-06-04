<script lang="ts">
	// Decorative sprocket-edge rails — the "film strip" signature motif. Drop this
	// INSIDE a `relative overflow-hidden` container (a roll/film-stock card or row)
	// and give that container enough padding to clear the rails (~16px on the railed
	// side). Horizontal renders top + bottom rails; vertical renders a single left
	// rail. Purely decorative: aria-hidden + pointer-events-none, no text overlay, so
	// it can't affect contrast. Used only on entities that ARE film (rolls, film
	// stocks) — never on gear (cameras, lenses).
	interface Props {
		orientation?: 'horizontal' | 'vertical';
	}

	let { orientation = 'horizontal' }: Props = $props();
</script>

{#if orientation === 'horizontal'}
	<div aria-hidden="true" class="film-perfs-x pointer-events-none absolute inset-x-0 top-0 h-[14px]"></div>
	<div aria-hidden="true" class="film-perfs-x pointer-events-none absolute inset-x-0 bottom-0 h-[14px]"></div>
{:else}
	<div aria-hidden="true" class="film-perfs-y pointer-events-none absolute inset-y-0 left-0 w-[14px]"></div>
{/if}
