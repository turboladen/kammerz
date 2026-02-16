<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Badge from '$lib/components/ui/Badge.svelte';
	import { listRolls } from '$lib/db/rolls';
	import { listCameras } from '$lib/db/cameras';
	import type { RollWithDetails, Camera, RollStatus } from '$lib/types';

	let rolls: RollWithDetails[] = $state([]);
	let cameras: Camera[] = $state([]);
	let loading = $state(true);

	const rollsByStatus = $derived(
		rolls.reduce(
			(acc, roll) => {
				acc[roll.status] = (acc[roll.status] || 0) + 1;
				return acc;
			},
			{} as Record<RollStatus, number>
		)
	);

	const needsAttention = $derived(
		rolls.filter((r) => !r.camera_id || r.status === 'at-lab')
	);

	async function load() {
		try {
			[rolls, cameras] = await Promise.all([listRolls(), listCameras()]);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Dashboard" description="Your film photography catalog at a glance">
	<Button variant="primary" href="/rolls/new">+ New Roll</Button>
</PageHeader>

<div class="flex-1 p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		<!-- Stats -->
		<div class="mb-8 grid grid-cols-4 gap-4">
			<div class="rounded-xl border border-border bg-surface-raised p-4">
				<p class="text-2xl font-semibold">{rolls.length}</p>
				<p class="text-xs text-text-muted">Total Rolls</p>
			</div>
			<div class="rounded-xl border border-border bg-surface-raised p-4">
				<p class="text-2xl font-semibold">{cameras.length}</p>
				<p class="text-xs text-text-muted">Cameras</p>
			</div>
			<div class="rounded-xl border border-border bg-surface-raised p-4">
				<p class="text-2xl font-semibold">{rollsByStatus['shooting'] ?? 0}</p>
				<p class="text-xs text-text-muted">Currently Shooting</p>
			</div>
			<div class="rounded-xl border border-border bg-surface-raised p-4">
				<p class="text-2xl font-semibold">{rollsByStatus['at-lab'] ?? 0}</p>
				<p class="text-xs text-text-muted">At Lab</p>
			</div>
		</div>

		<!-- Status breakdown -->
		{#if rolls.length > 0}
			<div class="mb-8">
				<h2 class="mb-3 text-sm font-semibold text-text-muted">Rolls by Status</h2>
				<div class="flex flex-wrap gap-2">
					{#each Object.entries(rollsByStatus) as [status, count]}
						<span class="flex items-center gap-2">
							<Badge status={status as RollStatus} />
							<span class="text-sm text-text-muted">{count}</span>
						</span>
					{/each}
				</div>
			</div>
		{/if}

		<!-- Needs attention -->
		{#if needsAttention.length > 0}
			<div>
				<h2 class="mb-3 text-sm font-semibold text-text-muted">Needs Attention</h2>
				<div class="space-y-2">
					{#each needsAttention as roll}
						<a
							href="/rolls/{roll.id}"
							class="flex items-center justify-between rounded-lg border border-border bg-surface-raised p-3 transition-colors hover:bg-surface-overlay"
						>
							<div class="flex items-center gap-3">
								<span class="font-mono text-sm">{roll.roll_id}</span>
								<Badge status={roll.status} />
							</div>
							<div class="text-xs text-text-muted">
								{#if !roll.camera_id}
									No camera assigned
								{:else if roll.status === 'at-lab'}
									Waiting at lab
								{/if}
							</div>
						</a>
					{/each}
				</div>
			</div>
		{:else if rolls.length === 0}
			<div class="flex flex-col items-center gap-4 py-16 text-center">
				<p class="text-text-muted">No rolls yet. Start by adding your cameras and creating your first roll.</p>
				<div class="flex gap-3">
					<Button href="/cameras">Add Cameras</Button>
					<Button variant="primary" href="/rolls/new">Create Roll</Button>
				</div>
			</div>
		{/if}
	{/if}
</div>
