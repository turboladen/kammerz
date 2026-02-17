<script lang="ts">
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import { createRoll, suggestRollId } from '$lib/api/rolls';
	import { listCameras, getLensesForCamera } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { lensDisplayName } from '$lib/utils/lens';
	import type { Camera, FilmStock, Lens, RollInsert } from '$lib/types';

	let cameras: Camera[] = $state([]);
	let filmStocks: FilmStock[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let cameraLensIds: number[] = $state([]);
	let loading = $state(true);

	let rollId = $state('');
	let cameraId = $state('');
	let filmStockId = $state('');
	let lensId = $state('');
	let frameCount = $state('');
	let dateLoaded = $state('');
	let dateFuzzy = $state('');
	let pushPull = $state('');
	let notes = $state('');
	let error = $state('');

	// Map camera format → matching film stock format
	const cameraFormatToStockFormat: Record<string, string> = {
		'35mm': '135',
		'medium format': '120',
		'6x4.5': '120',
		'6x6': '120',
		'6x7': '120',
		'6x8': '120',
		'6x9': '120',
		'large format': '4x5', // default large format to 4x5
		'4x5': '4x5',
		'5x7': '5x7',
		'8x10': '8x10'
	};

	const selectedCamera = $derived(
		cameraId ? cameras.find((c) => c.id === Number(cameraId)) : null
	);

	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned yet' },
		...cameras.map((c) => ({ value: String(c.id), label: `${c.brand} ${c.model}` }))
	]);

	function stockLabel(s: FilmStock): string {
		return `${s.brand} ${s.name} (${s.format}${s.iso ? ', ISO ' + s.iso : ''})`;
	}

	const filmStockOptions = $derived.by(() => {
		const matchingFormat = selectedCamera
			? cameraFormatToStockFormat[selectedCamera.format]
			: null;

		if (!matchingFormat) {
			// No camera selected or unknown format — show all stocks flat
			return [
				{ value: '', label: 'Select film stock' },
				...filmStocks.map((s) => ({ value: String(s.id), label: stockLabel(s) }))
			];
		}

		const matching = filmStocks.filter((s) => s.format === matchingFormat);
		const rest = filmStocks.filter((s) => s.format !== matchingFormat);

		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'Select film stock' }
		];

		if (matching.length > 0) {
			for (const s of matching) {
				options.push({ value: String(s.id), label: stockLabel(s) });
			}
		}

		if (matching.length > 0 && rest.length > 0) {
			options.push({ value: '__divider__', label: '── Other formats ──', disabled: true });
		}

		for (const s of rest) {
			options.push({ value: String(s.id), label: stockLabel(s) });
		}

		return options;
	});

	// Fetch camera-linked lens IDs when camera changes
	$effect(() => {
		const camId = cameraId;
		if (camId) {
			getLensesForCamera(Number(camId))
				.then((ids) => (cameraLensIds = ids))
				.catch(() => (cameraLensIds = []));
		} else {
			cameraLensIds = [];
		}
	});

	const lensOptions = $derived.by(() => {
		const owned = allLenses.filter((l) => !l.date_sold);
		const linked = owned.filter((l) => cameraLensIds.includes(l.id));
		const other = owned.filter((l) => !cameraLensIds.includes(l.id));
		const options: { value: string; label: string; disabled?: boolean }[] = [
			{ value: '', label: 'No default lens' }
		];
		for (const l of linked) {
			options.push({ value: String(l.id), label: lensDisplayName(l) });
		}
		if (linked.length > 0 && other.length > 0) {
			options.push({ value: '__divider__', label: '── Other lenses ──', disabled: true });
		}
		for (const l of other) {
			options.push({ value: String(l.id), label: lensDisplayName(l) });
		}
		return options;
	});

	const pushPullOptions = [
		{ value: '', label: 'Normal (box speed)' },
		{ value: '-2', label: 'Pull -2' },
		{ value: '-1', label: 'Pull -1' },
		{ value: '+1', label: 'Push +1' },
		{ value: '+2', label: 'Push +2' },
		{ value: '+3', label: 'Push +3' }
	];

	async function load() {
		try {
			const [cams, stocks, lenses, suggestedId] = await Promise.all([
				listCameras(),
				listFilmStocks(),
				listLenses(),
				suggestRollId()
			]);
			cameras = cams;
			filmStocks = stocks;
			allLenses = lenses;
			rollId = suggestedId;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	async function handleSubmit() {
		error = '';
		try {
			const roll: RollInsert = {
				roll_id: rollId,
				camera_id: cameraId ? Number(cameraId) : null,
				film_stock_id: filmStockId ? Number(filmStockId) : null,
				lens_id: lensId ? Number(lensId) : null,
				status: 'loaded',
				frame_count: frameCount ? parseInt(frameCount) : null,
				date_loaded: dateLoaded || null,
				date_finished: null,
				date_fuzzy: dateFuzzy || null,
				push_pull: pushPull || null,
				notes: notes || null
			};
			const id = await createRoll(roll);
			goto(`/rolls/${id}`);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="New Roll" backHref="/rolls" backLabel="Rolls">
	<Button variant="ghost" href="/rolls">Cancel</Button>
</PageHeader>

<div class="max-w-2xl p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		<div class="space-y-4">
			<Input
				label="Roll ID"
				bind:value={rollId}
				hint="Auto-suggested as YYMMDD-N. You can type any ID you want."
			/>

			<Select label="Camera" bind:value={cameraId} options={cameraOptions} />
			<Select label="Film Stock" bind:value={filmStockId} options={filmStockOptions} />
			<Select label="Default Lens" bind:value={lensId} options={lensOptions} />

			<div class="grid grid-cols-3 gap-4">
				<Input label="Frame Count" bind:value={frameCount} type="number" placeholder="36" />
				<Input label="Date Loaded" bind:value={dateLoaded} type="date" />
				<Select label="Push/Pull" bind:value={pushPull} options={pushPullOptions} />
			</div>

			<Input
				label="Fuzzy Date"
				bind:value={dateFuzzy}
				placeholder="e.g. 'early October 2025'"
				hint="For when you don't know the exact date"
			/>

			<Textarea label="Notes" bind:value={notes} placeholder="Any notes about this roll..." />

			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}

			<div class="flex justify-end gap-2 pt-4">
				<Button variant="ghost" href="/rolls">Cancel</Button>
				<Button variant="primary" onclick={handleSubmit}>Create Roll</Button>
			</div>
		</div>
	{/if}
</div>
