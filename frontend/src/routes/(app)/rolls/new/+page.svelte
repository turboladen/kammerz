<script lang="ts">
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import DateInput from '$lib/components/ui/DateInput.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import { createRoll, suggestRollId } from '$lib/api/rolls';
	import { buildCameraLabels } from '$lib/utils/disambiguate';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { buildLensOptions, lensDisplayName } from '$lib/utils/lens';
	import { listLensMounts } from '$lib/api/lens-mounts';
	import { todayLocal, dateFieldError } from '$lib/utils/date';
	import type { Camera, FilmStock, Lens, LensMount, PushPull, RollInsert } from '$lib/types';

	let cameras: Camera[] = $state([]);
	let filmStocks: FilmStock[] = $state([]);
	let allLenses: Lens[] = $state([]);
	let lensMounts: LensMount[] = $state([]);
	let loading = $state(true);

	let rollId = $state('');
	let cameraId = $state('');
	let filmStockId = $state('');
	let lensId = $state('');
	let frameCount = $state('');
	let frameCountAutoFilledFrom = $state<string | null>(null); // tracks which stock auto-filled the frame count
	let dateLoaded = $state(todayLocal());
	let dateFuzzy = $state('');
	const dateLoadedError = $derived(dateFieldError(dateLoaded));
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
		'8x10': '8x10',
		'instant': 'instant'
	};

	const selectedCamera = $derived(
		cameraId ? cameras.find((c) => c.id === Number(cameraId)) : null
	);

	// Hint for medium format variable-back cameras and large format sheet film
	const frameCountHint = $derived.by(() => {
		const matchingFormat = selectedCamera
			? cameraFormatToStockFormat[selectedCamera.format]
			: null;
		if (matchingFormat === '120' && !frameCount) {
			return '120 film: 6\u00d74.5=15 \u00b7 6\u00d76=12 \u00b7 6\u00d77=10 \u00b7 6\u00d78=9 \u00b7 6\u00d79=8';
		}
		if (['4x5', '5x7', '8x10'].includes(matchingFormat ?? '') && !frameCount) {
			return 'Sheet film: total sheets loaded (e.g. 6 holders \u00d7 2 = 12)';
		}
		return undefined;
	});

	const isFixedLens = $derived(
		selectedCamera
			? lensMounts.some((m) => m.id === selectedCamera.lens_mount_id && m.name === 'Fixed Lens')
			: false
	);
	const fixedLens = $derived(
		isFixedLens && selectedCamera?.default_lens_id
			? allLenses.find((l) => l.id === selectedCamera.default_lens_id) ?? null
			: null
	);

	const cameraLabels = $derived(buildCameraLabels(cameras));
	const cameraOptions = $derived([
		{ value: '', label: 'Not assigned yet' },
		...cameras.map((c) => ({ value: String(c.id), label: cameraLabels.get(c.id) ?? `${c.brand} ${c.model}` }))
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

	const lensOptions = $derived(buildLensOptions(allLenses, selectedCamera, 'No default lens', lensMounts));

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
			const [cams, stocks, lenses, suggestedId, mounts] = await Promise.all([
				listCameras(),
				listFilmStocks(),
				listLenses(),
				suggestRollId(),
				listLensMounts()
			]);
			cameras = cams;
			filmStocks = stocks;
			allLenses = lenses;
			lensMounts = mounts;
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
				date_scanned: null,
				date_post_processed: null,
				date_archived: null,
				date_fuzzy: dateFuzzy || null,
				push_pull: (pushPull || null) as PushPull | null,
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

	// Auto-populate lens from camera's default_lens_id (set on camera detail page)
	$effect(() => {
		const cam = selectedCamera;
		if (cam?.default_lens_id) {
			lensId = String(cam.default_lens_id);
		} else {
			lensId = '';
		}
	});

	// Auto-fill frame count from film stock's exposure_count
	// Re-fills when stock changes (if value was auto-filled, not manually edited)
	$effect(() => {
		const stockId = filmStockId;
		if (stockId) {
			const stock = filmStocks.find((s) => s.id === Number(stockId));
			if (stock?.exposure_count && stock.exposure_count > 1) {
				const wasAutoFilled = frameCountAutoFilledFrom !== null;
				const isEmpty = !frameCount;
				const wasAutoFilledByDifferentStock = wasAutoFilled && frameCountAutoFilledFrom !== stockId;
				if (isEmpty || wasAutoFilledByDifferentStock) {
					frameCount = String(stock.exposure_count);
					frameCountAutoFilledFrom = stockId;
				}
			}
		}
	});
</script>

<PageHeader title="New Roll" backHref="/rolls" backLabel="Rolls">
	<Button variant="ghost" href="/rolls">Cancel</Button>
</PageHeader>

<div class="max-w-2xl p-6">
	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		<FadeIn delay={0}>
		<div class="space-y-4">
			<Input
				label="Roll ID"
				bind:value={rollId}
				hint="Auto-suggested as YYMMDD-N. You can type any ID you want."
			/>

			<Select label="Camera" bind:value={cameraId} options={cameraOptions} />
			<Select label="Film Stock" bind:value={filmStockId} options={filmStockOptions} />
			{#if isFixedLens && fixedLens}
				<div>
					<span class="mb-1.5 block text-xs font-medium text-text-muted">Default Lens</span>
					<div class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text-muted">
						{lensDisplayName(fixedLens)} <span class="text-text-faint">(fixed)</span>
					</div>
				</div>
			{:else}
				<Select label="Default Lens" bind:value={lensId} options={lensOptions} />
			{/if}

			<div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
				<Input label="Frame Count" bind:value={frameCount} type="number" placeholder="36" hint={frameCountHint} oninput={() => (frameCountAutoFilledFrom = null)} />
				<DateInput label="Date Loaded" bind:value={dateLoaded} />
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
				<Button variant="primary" disabled={!!dateLoadedError} onclick={handleSubmit}>Create Roll</Button>
			</div>
		</div>
		</FadeIn>
	{/if}
</div>
