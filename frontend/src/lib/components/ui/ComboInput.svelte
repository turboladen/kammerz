<script lang="ts">
	interface Props {
		label?: string;
		hint?: string;
		placeholder?: string;
		value: string;
		options: string[];
	}

	let { label, hint, placeholder, value = $bindable(), options }: Props = $props();

	let showDropdown = $state(false);
	let highlightIndex = $state(-1);
	let inputEl: HTMLInputElement | undefined = $state();

	const filtered = $derived(
		value
			? options.filter((o) => o.toLowerCase().includes(value.toLowerCase()) && o !== value)
			: options
	);

	function handleFocus() {
		showDropdown = true;
		highlightIndex = -1;
	}

	function handleBlur() {
		// Delay to allow click on dropdown option to register
		setTimeout(() => {
			showDropdown = false;
			highlightIndex = -1;
		}, 150);
	}

	function selectOption(option: string) {
		value = option;
		showDropdown = false;
		highlightIndex = -1;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (!showDropdown || filtered.length === 0) {
			if (e.key === 'ArrowDown' && filtered.length > 0) {
				showDropdown = true;
				highlightIndex = 0;
				e.preventDefault();
			}
			return;
		}

		switch (e.key) {
			case 'ArrowDown':
				e.preventDefault();
				highlightIndex = Math.min(highlightIndex + 1, filtered.length - 1);
				break;
			case 'ArrowUp':
				e.preventDefault();
				highlightIndex = Math.max(highlightIndex - 1, 0);
				break;
			case 'Enter':
				if (highlightIndex >= 0 && highlightIndex < filtered.length) {
					e.preventDefault();
					selectOption(filtered[highlightIndex]);
				}
				break;
			case 'Escape':
				if (showDropdown) {
					// Only close the dropdown — don't let the containing dialog see this Escape.
					e.stopPropagation();
					showDropdown = false;
					highlightIndex = -1;
				}
				break;
		}
	}
</script>

<div class="relative flex flex-col gap-1.5">
	{#if label}
		<span class="text-xs font-medium text-text-muted">{label}</span>
	{/if}
	<input
		bind:this={inputEl}
		bind:value
		{placeholder}
		onfocus={handleFocus}
		onblur={handleBlur}
		onkeydown={handleKeydown}
		autocomplete="off"
		class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
			transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
	/>
	{#if showDropdown && filtered.length > 0}
		<div class="absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto rounded-lg border border-border bg-surface-raised shadow-xl">
			{#each filtered as option, i}
				<button
					type="button"
					class="w-full px-3 py-2 text-left text-sm transition-colors
						{i === highlightIndex ? 'bg-accent/15 text-accent' : 'text-text hover:bg-surface-overlay'}"
					onmousedown={() => selectOption(option)}
				>
					{option}
				</button>
			{/each}
		</div>
	{/if}
	{#if hint}
		<span class="text-xs text-text-faint">{hint}</span>
	{/if}
</div>
