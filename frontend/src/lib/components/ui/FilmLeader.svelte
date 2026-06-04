<script lang="ts">
	// Decorative "unexposed film leader" — a short sprocketed strip with blank frames,
	// used as characterful artwork in empty states (no rolls / film yet). Purely
	// decorative; the whole strip is aria-hidden.
	import FilmStrip from './FilmStrip.svelte';
</script>

<div
	aria-hidden="true"
	class="relative w-full max-w-xs overflow-hidden rounded-md border border-border bg-surface-raised"
>
	<FilmStrip />
	<div class="flex gap-[3px] px-3 py-[18px]">
		{#each Array.from({ length: 4 }) as _unused}
			<div class="h-12 flex-1 rounded-sm border border-border-subtle bg-surface"></div>
		{/each}
	</div>
</div>
