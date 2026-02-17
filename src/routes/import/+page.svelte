<script lang="ts">
	import { goto } from '$app/navigation';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import { listModels, parseNote, importParsedRoll } from '$lib/api/import';
	import { getSetting, setSetting } from '$lib/api/settings';
	import { listCameras } from '$lib/api/cameras';
	import { listFilmStocks } from '$lib/api/film-stocks';
	import { listLenses } from '$lib/api/lenses';
	import { lensDisplayName } from '$lib/utils/lens';
	import type { ParsedRoll, Camera, FilmStock, Lens, ImportRollDto, ModelInfo } from '$lib/types';
	import { ChevronDown, ChevronUp, Eye, EyeOff, RefreshCw, Trash2 } from 'lucide-svelte';

	// --- State ---

	type Step = 'input' | 'preview' | 'importing';
	let step: Step = $state('input');

	// Settings
	let apiKey = $state('');
	let showApiKey = $state(false);
	let selectedModel = $state('claude-sonnet-4-5-20250929');
	let showSettings = $state(false);
	let savingKey = $state(false);
	let keySaved = $state(false);
	let settingsLoaded = $state(false); // guards $effect from saving during initial load
	let lastPersistedModel = $state(''); // tracks last DB value to avoid redundant writes

	// Models
	let models: ModelInfo[] = $state([]);
	let loadingModels = $state(false);
	let modelsError = $state('');

	// Input
	let noteText = $state('');
	let parsing = $state(false);
	let error = $state('');

	// Preview data
	let parsed: ParsedRoll | null = $state(null);
	let cameras: Camera[] = $state([]);
	let filmStocks: FilmStock[] = $state([]);
	let allLenses: Lens[] = $state([]);

	// Editable roll fields
	let rollId = $state('');
	let cameraId = $state('');
	let filmStockId = $state('');
	let lensId = $state('');
	let rollStatus = $state('archived');
	let frameCount = $state('');
	let dateLoaded = $state('');
	let dateFinished = $state('');
	let rollNotes = $state('');

	// Editable shots
	let shots: {
		frame_number: string;
		aperture: string;
		shutter_speed: string;
		date: string;
		location: string;
		notes: string;
	}[] = $state([]);

	// Derived
	const modelOptions = $derived(
		models.length > 0
			? models.map((m) => ({ value: m.id, label: m.display_name }))
			: [
					{ value: 'claude-sonnet-4-5-20250929', label: 'Claude Sonnet 4.5' },
					{ value: 'claude-haiku-4-5-20251001', label: 'Claude Haiku 4.5' },
					{ value: 'claude-opus-4-6', label: 'Claude Opus 4.6' }
				]
	);

	const selectedModelLabel = $derived(
		modelOptions.find((m) => m.value === selectedModel)?.label ?? selectedModel
	);

	const statusOptions = [
		{ value: 'loaded', label: 'Loaded' },
		{ value: 'shooting', label: 'Shooting' },
		{ value: 'shot', label: 'Shot' },
		{ value: 'at-lab', label: 'At Lab' },
		{ value: 'developing', label: 'Developing' },
		{ value: 'developed', label: 'Developed' },
		{ value: 'scanned', label: 'Scanned' },
		{ value: 'archived', label: 'Archived' }
	];

	const cameraOptions = $derived(
		cameras.map((c) => ({
			value: String(c.id),
			label: `${c.brand} ${c.model}`
		}))
	);

	const filmStockOptions = $derived(
		filmStocks.map((fs) => ({
			value: String(fs.id),
			label: `${fs.brand} ${fs.name} (${fs.format})`
		}))
	);

	const hasApiKey = $derived(apiKey.length > 0);

	const lensOptions = $derived.by(() => {
		const owned = allLenses.filter((l) => !l.date_sold);
		const options: { value: string; label: string }[] = [
			{ value: '', label: 'No default lens' }
		];
		for (const l of owned) {
			options.push({ value: String(l.id), label: lensDisplayName(l) });
		}
		return options;
	});

	// Unmatched warnings — show what the AI guessed when no DB match found
	let cameraGuess = $state('');
	let filmStockGuess = $state('');
	let lensGuess = $state('');
	const cameraUnmatched = $derived(cameraGuess !== '' && cameraId === '');
	const filmStockUnmatched = $derived(filmStockGuess !== '' && filmStockId === '');
	const lensUnmatched = $derived(lensGuess !== '' && lensId === '');

	// --- Load settings on mount ---

	$effect(() => {
		loadSettings();
	});

	async function loadSettings() {
		try {
			const [key, model] = await Promise.all([
				getSetting('claude_api_key'),
				getSetting('claude_model')
			]);
			if (key) {
				apiKey = key;
				keySaved = true;
				// Auto-fetch models if we have a saved key
				fetchModels();
			}
			if (model) {
				selectedModel = model;
				lastPersistedModel = model;
			}
		} catch {
			// Settings not yet configured — that's fine
		} finally {
			settingsLoaded = true;
		}
	}

	// --- Handlers ---

	async function saveApiKey() {
		if (!apiKey.trim()) return;
		savingKey = true;
		error = '';
		try {
			await setSetting('claude_api_key', apiKey.trim());
			keySaved = true;
			// Automatically fetch models after saving key
			await fetchModels();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			savingKey = false;
		}
	}

	async function fetchModels() {
		loadingModels = true;
		modelsError = '';
		try {
			const result = await listModels();
			models = result;
		} catch (err) {
			modelsError = err instanceof Error ? err.message : String(err);
		} finally {
			loadingModels = false;
		}
	}

	// Auto-persist model selection whenever it changes (after initial load)
	$effect(() => {
		const model = selectedModel; // read to subscribe
		if (!settingsLoaded) return;  // skip before settings loaded
		if (model === lastPersistedModel) return;  // skip redundant writes
		lastPersistedModel = model;
		setSetting('claude_model', model).catch(() => {
			// Non-critical — model will fall back to default
		});
	});

	async function handleParse() {
		if (!noteText.trim()) return;
		if (!hasApiKey) {
			error = 'Please enter your Claude API key first.';
			showSettings = true;
			return;
		}

		parsing = true;
		error = '';

		try {
			// Save key if not yet saved
			if (!keySaved) {
				await setSetting('claude_api_key', apiKey.trim());
				keySaved = true;
			}

			// Load cameras, film stocks, and lenses for the preview
			const [parsedResult, cameraList, fsList, lensList] = await Promise.all([
				parseNote(noteText, selectedModel),
				listCameras(),
				listFilmStocks(),
				listLenses()
			]);

			parsed = parsedResult;
			cameras = cameraList;
			filmStocks = fsList;
			allLenses = lensList;

			// Populate editable fields from parsed data
			rollId = parsed.roll_id;
			frameCount = parsed.frame_count != null ? String(parsed.frame_count) : '';
			dateLoaded = parsed.date_loaded ?? '';
			dateFinished = parsed.date_finished ?? '';
			rollNotes = parsed.notes ?? '';

			// Auto-match camera from prefix
			cameraGuess = parsed.camera_prefix_guess ?? '';
			cameraId = '';
			if (parsed.camera_prefix_guess) {
				const prefix = parsed.camera_prefix_guess.toLowerCase();
				const match = cameras.find((c) => c.prefix?.toLowerCase() === prefix);
				if (match) cameraId = String(match.id);
			}

			// Auto-match film stock from guess
			filmStockGuess = parsed.film_stock_guess ?? '';
			filmStockId = '';
			if (parsed.film_stock_guess) {
				const guess = parsed.film_stock_guess.toLowerCase();
				const match = filmStocks.find((fs) => {
					const fullName = `${fs.brand} ${fs.name}`.toLowerCase();
					return fullName.includes(guess) || guess.includes(fullName);
				});
				if (match) filmStockId = String(match.id);
			}

			// Auto-match lens from guess
			lensGuess = parsed.lens_guess ?? '';
			lensId = '';
			if (parsed.lens_guess) {
				const guess = parsed.lens_guess.toLowerCase();
				const match = allLenses.find((l) => {
					const display = lensDisplayName(l).toLowerCase();
					return display.includes(guess) || guess.includes(display);
				});
				if (match) lensId = String(match.id);
			}

			// Populate editable shots
			shots = parsed.shots.map((s) => ({
				frame_number: s.frame_number,
				aperture: s.aperture ?? '',
				shutter_speed: s.shutter_speed ?? '',
				date: s.date ?? '',
				location: s.location ?? '',
				notes: s.notes ?? ''
			}));

			step = 'preview';
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			parsing = false;
		}
	}

	function removeShot(index: number) {
		shots = shots.filter((_, i) => i !== index);
	}

	async function handleImport() {
		step = 'importing';
		error = '';

		try {
			const data: ImportRollDto = {
				roll_id: rollId,
				camera_id: cameraId ? parseInt(cameraId) : null,
				film_stock_id: filmStockId ? parseInt(filmStockId) : null,
				lens_id: lensId ? parseInt(lensId) : null,
				status: rollStatus,
				frame_count: frameCount ? parseInt(frameCount) : null,
				date_loaded: dateLoaded || null,
				date_finished: dateFinished || null,
				date_fuzzy: null,
				notes: rollNotes || null,
				shots: shots.map((s) => ({
					frame_number: s.frame_number,
					aperture: s.aperture || null,
					shutter_speed: s.shutter_speed || null,
					date: s.date || null,
					date_fuzzy: null,
					location: s.location || null,
					notes: s.notes || null,
					lens_ids: null
				}))
			};

			const newRollId = await importParsedRoll(data);
			goto(`/rolls/${newRollId}`);
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
			step = 'preview';
		}
	}
</script>

<PageHeader title="Import Notes" description="Import film notes using AI" backHref="/rolls" backLabel="Rolls" />

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	{#if step === 'input'}
		<!-- Settings Section -->
		<div class="mb-6 max-w-2xl">
			<button
				class="flex w-full items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 text-left text-sm text-text-muted transition-colors hover:border-accent/40"
				onclick={() => (showSettings = !showSettings)}
			>
				<span>
					{#if hasApiKey}
						API key configured · Model: {selectedModelLabel}
					{:else}
						Configure API key to get started
					{/if}
				</span>
				{#if showSettings}
					<ChevronUp size={14} />
				{:else}
					<ChevronDown size={14} />
				{/if}
			</button>

			{#if showSettings}
				<div class="mt-2 rounded-lg border border-border bg-surface-raised p-4 space-y-4">
					<!-- API Key with eye toggle -->
					<div>
						<span class="mb-1.5 block text-xs font-medium text-text-muted">Claude API Key</span>
						<div class="flex items-center gap-2">
							<div class="relative flex-1">
								<input
									bind:value={apiKey}
									type={showApiKey ? 'text' : 'password'}
									placeholder="sk-ant-..."
									class="w-full rounded-lg border border-border bg-surface px-3 py-2 pr-10 text-sm text-text placeholder-text-faint
										transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
								/>
								<button
									onclick={() => (showApiKey = !showApiKey)}
									class="absolute right-2 top-1/2 -translate-y-1/2 rounded p-1 text-text-faint transition-colors hover:text-text"
									title={showApiKey ? 'Hide API key' : 'Show API key'}
								>
									{#if showApiKey}
										<EyeOff size={14} strokeWidth={1.75} />
									{:else}
										<Eye size={14} strokeWidth={1.75} />
									{/if}
								</button>
							</div>
							<Button
								size="sm"
								onclick={saveApiKey}
								disabled={savingKey || !apiKey.trim()}
							>
								{savingKey ? 'Saving...' : 'Save'}
							</Button>
						</div>
					</div>

					<!-- Model selector with refresh button -->
					<div>
						<span class="mb-1.5 block text-xs font-medium text-text-muted">Model</span>
						<div class="flex items-center gap-2">
							<div class="flex-1">
								<Select
									options={modelOptions}
									bind:value={selectedModel}
								/>
							</div>
							<button
								onclick={fetchModels}
								disabled={loadingModels || !hasApiKey}
								class="rounded-lg border border-border bg-surface px-2.5 py-2 text-text-faint transition-colors
									hover:border-accent/40 hover:text-accent disabled:opacity-40 disabled:cursor-not-allowed"
								title="Refresh model list from API"
							>
								<RefreshCw size={14} strokeWidth={1.75} class={loadingModels ? 'animate-spin' : ''} />
							</button>
						</div>
						{#if modelsError}
							<span class="mt-1 block text-xs text-red-400">{modelsError}</span>
						{:else if models.length > 0}
							<span class="mt-1 block text-xs text-text-faint">{models.length} models available</span>
						{:else if !hasApiKey}
							<span class="mt-1 block text-xs text-text-faint">Save your API key to load available models</span>
						{/if}
					</div>
				</div>
			{/if}
		</div>

		<!-- Note Input -->
		<div class="max-w-2xl">
			<label class="flex flex-col gap-1.5">
				<span class="text-xs font-medium text-text-muted">Paste your note</span>
				<textarea
					bind:value={noteText}
					rows="15"
					placeholder="Paste a film note from Apple Notes here...

Example:
NFE-1  Portra 400 x36 All 36 on 50mm 1.8 E Finished 11/24
or
M67-24 Ilford Delta 400 Loaded 5/16/21
1. 5/16, 7:27pm, 65mm, f11, 1/8. Backyard"
					class="rounded-lg border border-border bg-surface px-3 py-2 text-sm text-text placeholder-text-faint
						transition-colors focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50
						font-mono resize-y"
				></textarea>
			</label>

			<div class="mt-4">
				<Button
					variant="primary"
					onclick={handleParse}
					disabled={parsing || !noteText.trim()}
				>
					{#if parsing}
						Analyzing your notes...
					{:else}
						Parse with AI
					{/if}
				</Button>
			</div>
		</div>
	{:else if step === 'preview'}
		<!-- Preview & Edit -->
		<div class="max-w-4xl">
			<div class="mb-4 flex items-center justify-between">
				<h2 class="text-sm font-semibold text-text-muted">Review Parsed Data</h2>
				<Button size="sm" variant="ghost" onclick={() => (step = 'input')}>
					&larr; Back to input
				</Button>
			</div>

			<!-- Roll Info -->
			<div class="mb-6 rounded-lg border border-border bg-surface-raised p-4">
				<h3 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
					Roll Info
					<div class="flex-1 border-b border-border-subtle"></div>
				</h3>
				<div class="grid grid-cols-2 gap-4">
					<Input label="Roll ID" bind:value={rollId} />
					<Select
						label="Status"
						options={statusOptions}
						bind:value={rollStatus}
					/>
					<div>
						<Select
							label="Camera"
							options={cameraOptions}
							bind:value={cameraId}
							placeholder="Select camera..."
						/>
						{#if cameraUnmatched}
							<p class="mt-1 text-xs text-amber-400">
								AI detected prefix "{cameraGuess}" but no matching camera found.
								<a href="/cameras" class="underline hover:text-accent">Add camera</a> first?
							</p>
						{/if}
					</div>
					<div>
						<Select
							label="Film Stock"
							options={filmStockOptions}
							bind:value={filmStockId}
							placeholder="Select film stock..."
						/>
						{#if filmStockUnmatched}
							<p class="mt-1 text-xs text-amber-400">
								AI detected "{filmStockGuess}" but no matching film stock found.
								<a href="/film-stocks" class="underline hover:text-accent">Add film stock</a> first?
							</p>
						{/if}
					</div>
					<div>
						<Select
							label="Default Lens"
							options={lensOptions}
							bind:value={lensId}
							placeholder="Select lens..."
						/>
						{#if lensUnmatched}
							<p class="mt-1 text-xs text-amber-400">
								AI detected "{lensGuess}" but no matching lens found.
								<a href="/lenses" class="underline hover:text-accent">Add lens</a> first?
							</p>
						{/if}
					</div>
					<Input label="Frame Count" type="number" bind:value={frameCount} />
					<Input label="Date Loaded" type="date" bind:value={dateLoaded} />
					<Input label="Date Finished" type="date" bind:value={dateFinished} />
					<div class="col-span-2">
						<Input label="Notes" bind:value={rollNotes} />
					</div>
				</div>
			</div>

			<!-- Shots -->
			{#if shots.length > 0}
				<div class="mb-6 rounded-lg border border-border bg-surface-raised p-4">
					<h3 class="mb-3 flex items-center gap-3 text-xs font-semibold uppercase tracking-wider text-text-faint">
						Shots ({shots.length})
						<div class="flex-1 border-b border-border-subtle"></div>
					</h3>
					<div class="overflow-x-auto">
						<table class="w-full text-sm">
							<thead>
								<tr class="border-b border-border text-left">
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Frame</th>
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Aperture</th>
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Shutter</th>
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Date</th>
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Location</th>
									<th class="px-2 py-1.5 text-xs font-medium text-text-faint">Notes</th>
									<th class="w-8"></th>
								</tr>
							</thead>
							<tbody>
								{#each shots as shot, i}
									<tr class="border-b border-border/50">
										<td class="px-1 py-1">
											<input
												bind:value={shot.frame_number}
												class="w-14 rounded border border-border bg-surface px-1.5 py-1 text-xs text-text font-mono focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<input
												bind:value={shot.aperture}
												class="w-16 rounded border border-border bg-surface px-1.5 py-1 text-xs text-text focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<input
												bind:value={shot.shutter_speed}
												class="w-16 rounded border border-border bg-surface px-1.5 py-1 text-xs text-text focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<input
												bind:value={shot.date}
												type="date"
												class="rounded border border-border bg-surface px-1.5 py-1 text-xs text-text focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<input
												bind:value={shot.location}
												class="w-28 rounded border border-border bg-surface px-1.5 py-1 text-xs text-text focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<input
												bind:value={shot.notes}
												class="w-32 rounded border border-border bg-surface px-1.5 py-1 text-xs text-text focus:border-accent focus:outline-none focus:ring-1 focus:ring-accent/50"
											/>
										</td>
										<td class="px-1 py-1">
											<button
												onclick={() => removeShot(i)}
												class="rounded p-1 text-text-faint transition-colors hover:bg-red-500/15 hover:text-red-400"
												title="Remove shot"
											>
												<Trash2 size={12} />
											</button>
										</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				</div>
			{/if}

			<!-- Import Button -->
			<Button variant="primary" onclick={handleImport}>
				Import Roll & {shots.length} Shot{shots.length !== 1 ? 's' : ''}
			</Button>
		</div>
	{:else if step === 'importing'}
		<div class="flex items-center gap-3 text-sm text-text-muted">
			<span>Importing roll and shots...</span>
		</div>
	{/if}
</div>
