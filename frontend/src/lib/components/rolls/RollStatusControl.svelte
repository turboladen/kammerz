<script lang="ts">
	import { CircleHelp } from 'lucide-svelte';
	import { statusConfig } from '$lib/utils/status';
	import type { RollStatus } from '$lib/types';
	import type { DevPath } from '$lib/utils/status';

	interface Props {
		statusFlow: RollStatus[];
		currentStatus: RollStatus;
		currentStatusIdx: number;
		devPath: DevPath;
		pathLabel: string | null;
		hintFor: (status: RollStatus) => string;
		onmove: (status: RollStatus) => void;
		onchoosepath: (path: 'lab' | 'self') => void;
	}

	let { statusFlow, currentStatus, currentStatusIdx, devPath, pathLabel, hintFor, onmove, onchoosepath }: Props =
		$props();

	let showStatusHelp = $state(false);
	let showDevPathMenu = $state(false);
</script>

<div class="mb-6">
	<div class="mb-3 flex items-center gap-2">
		<h2 class="text-xs font-semibold uppercase tracking-wider text-text-faint">Status</h2>
		<div class="relative inline-flex">
			<button
				onclick={() => (showStatusHelp = !showStatusHelp)}
				title="How the status bar works"
				aria-label="How the status bar works"
				aria-expanded={showStatusHelp}
				class="inline-flex items-center text-text-faint transition-colors hover:text-text"
			>
				<CircleHelp size={14} strokeWidth={2} aria-hidden="true" />
			</button>
			{#if showStatusHelp}
				<!-- click-away catcher -->
				<button
					class="fixed inset-0 z-10 cursor-default"
					aria-label="Close status help"
					onclick={() => (showStatusHelp = false)}
				></button>
				<div
					class="absolute left-0 top-full z-20 mt-1.5 w-72 rounded-lg border border-border bg-surface-overlay p-3 text-xs text-text-muted shadow-lg"
				>
					<p class="mb-2 text-text">Click a step to move the roll there.</p>
					<ul class="space-y-1.5">
						<li>Forward steps apply instantly.</li>
						<li>A later step that needs a date asks for one first.</li>
						<li>An earlier step moves the roll back and asks to confirm.</li>
						<li>Lab and self steps open a development form when no record exists yet.</li>
					</ul>
				</div>
			{/if}
		</div>
		<div class="flex-1 border-b border-border-subtle"></div>
	</div>
	{#if pathLabel}
		<p class="mb-1.5 text-[10px] font-medium uppercase tracking-widest text-text-faint/70">{pathLabel}</p>
	{/if}
	<div class="flex flex-wrap items-center gap-[2px] gap-y-1">
		{#each statusFlow as status, idx}
			{@const isFirst = idx === 0}
			{@const isLast = idx === statusFlow.length - 1}
			{@const clipPath = isFirst
				? 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%)'
				: isLast
					? 'polygon(0 0, 100% 0, 100% 100%, 0 100%, 8px 50%)'
					: 'polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%, 8px 50%)'}
			<button
				onclick={() => onmove(status)}
				title={hintFor(status)}
				aria-label={hintFor(status)}
				style="clip-path: {clipPath}"
				class="whitespace-nowrap py-1.5 text-xs font-medium transition-colors
				{isFirst ? 'pl-3 pr-4' : isLast ? 'pl-4 pr-3' : 'px-4'}
				{currentStatus === status
					? 'bg-accent text-surface'
					: idx < currentStatusIdx
						? 'bg-surface-overlay text-accent hover:bg-surface-overlay/80'
						: 'bg-surface-raised text-text-muted hover:text-text'}"
			>
				{statusConfig[status].label}
			</button>
			{#if devPath === 'undecided' && status === 'shot'}
				<!-- Live next-step: the development decision happens here, in the flow,
				     rather than as disconnected dead placeholders. Choosing a path
				     opens the matching dev dialog and re-renders the bar to that flow. -->
				<div class="relative">
					<button
						onclick={() => (showDevPathMenu = !showDevPathMenu)}
						title="Choose a development path (lab or self) to start tracking development"
						aria-haspopup="menu"
						aria-expanded={showDevPathMenu}
						style="clip-path: polygon(0 0, calc(100% - 8px) 0, 100% 50%, calc(100% - 8px) 100%, 0 100%, 8px 50%)"
						class="px-4 py-1.5 text-xs font-medium transition-colors hover:bg-accent/25
						{showDevPathMenu ? 'bg-accent/25 text-accent' : 'bg-accent/15 text-accent'}"
					>
						Develop<span aria-hidden="true">&nbsp;⌄</span>
					</button>
					{#if showDevPathMenu}
						<!-- click-away catcher -->
						<button
							class="fixed inset-0 z-10 cursor-default"
							aria-label="Close development menu"
							onclick={() => (showDevPathMenu = false)}
						></button>
						<div
							role="menu"
							class="absolute left-1/2 top-full z-20 mt-1.5 -translate-x-1/2 overflow-hidden rounded-lg border border-border bg-surface-overlay shadow-lg"
						>
							<button
								role="menuitem"
								onclick={() => onchoosepath('lab')}
								class="block w-full whitespace-nowrap px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
								>Lab</button
							>
							<button
								role="menuitem"
								onclick={() => onchoosepath('self')}
								class="block w-full whitespace-nowrap border-t border-border-subtle px-4 py-2 text-left text-xs font-medium text-text-muted transition-colors hover:bg-accent/15 hover:text-accent"
								>Self / Home</button
							>
						</div>
					{/if}
				</div>
			{/if}
		{/each}
	</div>
</div>
