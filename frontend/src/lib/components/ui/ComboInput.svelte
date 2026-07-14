<script lang="ts">
	interface Props {
		label?: string;
		hint?: string;
		placeholder?: string;
		value: string;
		options: string[];
		/** Applied to the committed value on blur — e.g. strip an f/ prefix. */
		normalize?: (v: string) => string;
		/** Amber advisory shown below the input when set and the field is unfocused. */
		warning?: string;
		/** Render the input text in the mono data font. */
		mono?: boolean;
	}

	let {
		label,
		hint,
		placeholder,
		value = $bindable(),
		options,
		normalize,
		warning,
		mono = false
	}: Props = $props();

	let showDropdown = $state(false);
	let highlightIndex = $state(-1);
	let inputEl: HTMLInputElement | undefined = $state();

	// Unique per-instance id base so input/listbox/option ids don't collide
	// when multiple ComboInputs render on one page (e.g. Brand + Purchased From).
	const uid = $props.id();
	const inputId = `${uid}-input`;
	const listboxId = `${uid}-listbox`;
	const optionId = (i: number) => `${uid}-option-${i}`;

	const filtered = $derived(
		value ? options.filter((o) => o.toLowerCase().includes(value.toLowerCase()) && o !== value) : options
	);

	const expanded = $derived(showDropdown && filtered.length > 0);

	// Keep the highlight in range as the filtered list shrinks while typing —
	// otherwise aria-activedescendant points at an option that's no longer
	// rendered and the visual highlight lands on nothing.
	$effect(() => {
		if (highlightIndex >= filtered.length) {
			highlightIndex = filtered.length - 1;
		}
	});

	function handleFocus() {
		showDropdown = true;
		highlightIndex = -1;
	}

	function handleBlur() {
		// Delay to allow click on dropdown option to register
		setTimeout(() => {
			showDropdown = false;
			highlightIndex = -1;
			if (normalize) value = normalize(value);
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
		<label for={inputId} class="text-xs font-medium text-text-muted">{label}</label>
	{/if}
	<input
		bind:this={inputEl}
		bind:value
		id={inputId}
		{placeholder}
		onfocus={handleFocus}
		onblur={handleBlur}
		onkeydown={handleKeydown}
		autocomplete="off"
		role="combobox"
		aria-expanded={expanded}
		aria-autocomplete="list"
		aria-controls={listboxId}
		aria-activedescendant={highlightIndex >= 0 ? optionId(highlightIndex) : undefined}
		class="rounded-lg border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
			transition-colors focus:outline-none focus:ring-1 focus:ring-accent/50
			{mono ? 'font-mono' : ''}
			{warning && !showDropdown ? 'border-amber-500/60 focus:border-amber-500' : 'border-border focus:border-accent'}"
	/>
	{#if expanded}
		<div
			id={listboxId}
			role="listbox"
			class="absolute left-0 right-0 top-full z-50 mt-1 max-h-48 overflow-y-auto rounded-lg border border-border bg-surface-raised shadow-xl"
		>
			{#each filtered as option, i}
				<button
					type="button"
					id={optionId(i)}
					role="option"
					aria-selected={i === highlightIndex}
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
	{#if warning && !showDropdown}
		<span class="text-xs text-amber-400">{warning}</span>
	{/if}
</div>
