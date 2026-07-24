<script lang="ts">
	import { page } from '$app/state';
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import ListToolbar from '$lib/components/ui/ListToolbar.svelte';
	import GroupHeader from '$lib/components/ui/GroupHeader.svelte';
	import EmptyState from '$lib/components/ui/EmptyState.svelte';
	import FadeIn from '$lib/components/ui/FadeIn.svelte';
	import FilmStrip from '$lib/components/ui/FilmStrip.svelte';
	import { Film } from 'lucide-svelte';
	import {
		listFilmStocks,
		createFilmStock,
		updateFilmStock,
		deleteFilmStock,
		listDistinctFilmBrands
	} from '$lib/api/film-stocks';
	import { filterBySearch, groupItems, sortByString, sortByNumber, sortByDate } from '$lib/utils/list';
	import type { FilmStock, FilmFormat, FilmStockType, FilmStockInsert } from '$lib/types';

	let stocks: FilmStock[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let editingStock: FilmStock | null = $state(null);
	let filterType = $state('all');
	let filterFormat = $state('all');
	let deletingStock: FilmStock | null = $state(null);
	let error = $state('');
	// In-flight guard: blocks a double-click/double-tap from firing a create/update
	// twice (add + edit dialogs are temporally exclusive, so one flag covers both).
	let saving = $state(false);

	// Toolbar state (?q= pre-filters the list, e.g. from a search result link)
	let searchQuery = $state(page.url.searchParams.get('q') ?? '');
	let groupBy = $state('brand');
	let sortBy = $state('brand-asc');

	// Autocomplete options
	let filmBrandOptions: string[] = $state([]);

	// Pipeline: tab filter → search → sort → group
	const afterTabFilter = $derived(
		stocks.filter((s) => {
			// The 'slide' tab spans both slide types so bw-slide stocks stay reachable.
			if (filterType === 'slide') {
				if (s.stock_type !== 'color-slide' && s.stock_type !== 'bw-slide') return false;
			} else if (filterType !== 'all' && s.stock_type !== filterType) {
				return false;
			}
			if (filterFormat !== 'all' && s.format !== filterFormat) return false;
			return true;
		})
	);

	const afterSearch = $derived(
		filterBySearch(afterTabFilter, searchQuery, (s) => [s.brand, s.name, s.format, s.stock_type].join(' '))
	);

	const afterSort = $derived.by(() => {
		switch (sortBy) {
			case 'brand-desc':
				return sortByString(afterSearch, (s) => `${s.brand} ${s.name}`, 'desc');
			case 'iso-asc':
				return sortByNumber(afterSearch, (s) => s.iso, 'asc');
			case 'iso-desc':
				return sortByNumber(afterSearch, (s) => s.iso, 'desc');
			case 'date-added-desc':
				return sortByDate(afterSearch, (s) => s.created_at, 'desc');
			case 'format-asc':
				return sortByString(afterSearch, (s) => s.format, 'asc');
			default:
				return sortByString(afterSearch, (s) => `${s.brand} ${s.name}`, 'asc');
		}
	});

	const grouped = $derived.by(() => {
		if (groupBy === 'none') return groupItems(afterSort, () => '');
		if (groupBy === 'format') return groupItems(afterSort, (s) => s.format);
		if (groupBy === 'type') return groupItems(afterSort, (s) => typeDisplayNames[s.stock_type] ?? s.stock_type);
		return groupItems(afterSort, (s) => s.brand);
	});

	const resultCount = $derived(afterSearch.length);

	const groupByOptions = [
		{ value: 'brand', label: 'Brand' },
		{ value: 'format', label: 'Format' },
		{ value: 'type', label: 'Type' },
		{ value: 'none', label: 'None' }
	];

	const sortOptions = [
		{ value: 'brand-asc', label: 'A\u2013Z' },
		{ value: 'brand-desc', label: 'Z\u2013A' },
		{ value: 'iso-asc', label: 'ISO Low\u2013High' },
		{ value: 'iso-desc', label: 'ISO High\u2013Low' },
		{ value: 'date-added-desc', label: 'Recently Added' },
		{ value: 'format-asc', label: 'Format' }
	];

	// Form state
	let brand = $state('');
	let name = $state('');
	let format = $state('135');
	let exposureCount = $state('');
	let stockType = $state('color-negative');
	let iso = $state('');
	let notes = $state('');

	const formatOptions = [
		{ value: '135', label: '135 / 35mm' },
		{ value: '120', label: '120' },
		{ value: '4x5', label: '4x5' },
		{ value: '5x7', label: '5x7' },
		{ value: '8x10', label: '8x10' },
		{ value: 'instant', label: 'Instant' }
	];

	const typeOptions = [
		{ value: 'color-negative', label: 'Color Negative' },
		{ value: 'bw-negative', label: 'B&W Negative' },
		{ value: 'color-slide', label: 'Color Slide' },
		{ value: 'bw-slide', label: 'B&W Slide' }
	];

	const typeDisplayNames: Record<string, string> = {
		'color-negative': 'Color Neg',
		'bw-negative': 'B&W Neg',
		'color-slide': 'Color Slide',
		'bw-slide': 'B&W Slide'
	};

	async function load() {
		try {
			const [s, brands] = await Promise.all([listFilmStocks(), listDistinctFilmBrands()]);
			stocks = s;
			filmBrandOptions = brands;
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	function resetForm() {
		brand = '';
		name = '';
		format = '135';
		exposureCount = '';
		stockType = 'color-negative';
		iso = '';
		notes = '';
	}

	function openAddDialog() {
		resetForm();
		error = '';
		showAddDialog = true;
	}

	function buildInsert(): FilmStockInsert {
		return {
			brand,
			name,
			format: format as FilmFormat,
			exposure_count: exposureCount ? parseInt(exposureCount) : null,
			stock_type: stockType as FilmStockType,
			iso: iso ? parseInt(iso) : null,
			notes: notes || null
		};
	}

	async function handleAdd() {
		if (saving) return;
		error = '';
		if (!brand.trim()) {
			error = 'Brand is required.';
			return;
		}
		if (!name.trim()) {
			error = 'Name is required.';
			return;
		}
		saving = true;
		try {
			await createFilmStock(buildInsert());
			showAddDialog = false;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function startEdit(stock: FilmStock) {
		error = '';
		editingStock = stock;
		brand = stock.brand;
		name = stock.name;
		format = stock.format;
		exposureCount = stock.exposure_count?.toString() ?? '';
		stockType = stock.stock_type;
		iso = stock.iso?.toString() ?? '';
		notes = stock.notes ?? '';
	}

	async function handleEdit() {
		if (saving) return;
		if (!editingStock) return;
		error = '';
		if (!brand.trim()) {
			error = 'Brand is required.';
			return;
		}
		if (!name.trim()) {
			error = 'Name is required.';
			return;
		}
		saving = true;
		try {
			await updateFilmStock(editingStock.id, buildInsert());
			editingStock = null;
			resetForm();
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			saving = false;
		}
	}

	function handleDelete(stock: FilmStock) {
		deletingStock = stock;
	}

	async function confirmDelete() {
		if (!deletingStock) return;
		const stock = deletingStock;
		// Close the dialog before the request — a failure is reported via the
		// page error banner, and the dialog stays re-openable.
		deletingStock = null;
		error = '';
		try {
			await deleteFilmStock(stock.id);
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	$effect(() => {
		load();
	});
</script>

<PageHeader title="Film Stocks" description="Film stock catalog">
	<Button variant="primary" onclick={openAddDialog}>+ Add Film Stock</Button>
</PageHeader>

<div class="p-6">
	{#if error}
		<div class="mb-4 rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
	{/if}

	<!-- Toolbar: search + group + sort -->
	<ListToolbar
		bind:searchQuery
		bind:groupBy
		bind:sortBy
		{groupByOptions}
		{sortOptions}
		{resultCount}
		totalCount={afterTabFilter.length}
		placeholder="Search film stocks..."
	/>

	<!-- Filters -->
	<div class="mb-4 flex gap-4">
		<div class="flex gap-2">
			<Button size="sm" variant={filterType === 'all' ? 'primary' : 'ghost'} onclick={() => (filterType = 'all')}
				>All</Button
			>
			<Button
				size="sm"
				variant={filterType === 'color-negative' ? 'primary' : 'ghost'}
				onclick={() => (filterType = 'color-negative')}>Color</Button
			>
			<Button
				size="sm"
				variant={filterType === 'bw-negative' ? 'primary' : 'ghost'}
				onclick={() => (filterType = 'bw-negative')}>B&W</Button
			>
			<Button size="sm" variant={filterType === 'slide' ? 'primary' : 'ghost'} onclick={() => (filterType = 'slide')}
				>Slide</Button
			>
		</div>
		<div class="flex gap-2">
			<Button size="sm" variant={filterFormat === 'all' ? 'primary' : 'ghost'} onclick={() => (filterFormat = 'all')}
				>All Formats</Button
			>
			<Button size="sm" variant={filterFormat === '135' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '135')}
				>35mm</Button
			>
			<Button size="sm" variant={filterFormat === '120' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '120')}
				>120</Button
			>
			<Button size="sm" variant={filterFormat === '4x5' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '4x5')}
				>4x5</Button
			>
			<Button
				size="sm"
				variant={filterFormat === 'instant' ? 'primary' : 'ghost'}
				onclick={() => (filterFormat = 'instant')}>Instant</Button
			>
		</div>
	</div>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else if resultCount === 0 && stocks.length === 0}
		<EmptyState title="No Film Stocks" message="Add your first film stock to get started.">
			{#snippet icon()}<Film size={24} strokeWidth={1.5} />{/snippet}
			<Button variant="primary" onclick={openAddDialog}>+ Add Film Stock</Button>
		</EmptyState>
	{:else if resultCount === 0}
		<p class="mt-6 text-center text-sm text-text-muted">
			{searchQuery ? `No film stocks match your search.` : 'No film stocks match the current filters.'}
		</p>
	{:else}
		{#each Object.entries(grouped) as [groupKey, groupStocks]}
			<GroupHeader label={groupKey} />
			<div class="mb-4 grid gap-1.5">
				{#each groupStocks as stock, i}
					<FadeIn delay={Math.min(i, 10) * 30}>
						<div
							class="group relative flex items-center justify-between overflow-hidden rounded-lg border border-border bg-surface-raised py-2.5 pl-5 pr-4 transition-all duration-150 hover:border-accent/40 hover:-translate-y-px"
						>
							<FilmStrip orientation="vertical" />
							<div class="flex items-center gap-3">
								<span class="text-sm font-semibold leading-snug">{stock.brand} {stock.name}</span>
								{#if stock.iso}
									<span
										class="inline-flex items-center gap-1 rounded border border-border-subtle bg-surface px-1.5 py-0.5 font-mono text-[11px] text-text-muted"
									>
										<span aria-hidden="true" class="dx-barcode"></span>
										ISO {stock.iso}
									</span>
								{/if}
								<span class="text-xs text-text-faint">{stock.format}</span>
								<span class="text-xs text-text-faint">{typeDisplayNames[stock.stock_type] ?? stock.stock_type}</span>
								{#if stock.exposure_count}
									<span class="text-xs text-text-faint">{stock.exposure_count} exp</span>
								{/if}
							</div>
							<div
								class="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100 pointer-coarse:opacity-100"
							>
								<Button size="sm" variant="ghost" onclick={() => startEdit(stock)}>Edit</Button>
								<Button size="sm" variant="ghost" onclick={() => handleDelete(stock)}>&times;</Button>
							</div>
						</div>
					</FadeIn>
				{/each}
			</div>
		{/each}
	{/if}
</div>

<Dialog bind:open={showAddDialog} title="Add Film Stock" onclose={resetForm}>
	<div class="space-y-4">
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<ComboInput label="Brand" bind:value={brand} placeholder="Kodak" options={filmBrandOptions} />
			<Input label="Name" bind:value={name} placeholder="Portra 400" />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Select label="Format" bind:value={format} options={formatOptions} />
			<Select label="Type" bind:value={stockType} options={typeOptions} />
		</div>
		<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
			<Input label="ISO" bind:value={iso} type="number" placeholder="400" />
			<Input
				label="Exposure Count"
				bind:value={exposureCount}
				type="number"
				placeholder="36"
				hint="Leave empty for variable (120 film)"
			/>
		</div>
		<Textarea label="Notes" bind:value={notes} />
		{#if error}
			<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button
				variant="ghost"
				onclick={() => {
					showAddDialog = false;
					resetForm();
				}}>Cancel</Button
			>
			<Button variant="primary" disabled={saving} onclick={handleAdd}>Add Film Stock</Button>
		</div>
	</div>
</Dialog>

{#if editingStock}
	<Dialog
		open={true}
		title="Edit Film Stock"
		onclose={() => {
			editingStock = null;
			resetForm();
		}}
	>
		<div class="space-y-4">
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<ComboInput label="Brand" bind:value={brand} placeholder="Kodak" options={filmBrandOptions} />
				<Input label="Name" bind:value={name} placeholder="Portra 400" />
			</div>
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Select label="Format" bind:value={format} options={formatOptions} />
				<Select label="Type" bind:value={stockType} options={typeOptions} />
			</div>
			<div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
				<Input label="ISO" bind:value={iso} type="number" placeholder="400" />
				<Input
					label="Exposure Count"
					bind:value={exposureCount}
					type="number"
					placeholder="36"
					hint="Leave empty for variable (120 film)"
				/>
			</div>
			<Textarea label="Notes" bind:value={notes} />
			{#if error}
				<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
			{/if}
			<div class="flex justify-end gap-2 pt-2">
				<Button
					variant="ghost"
					onclick={() => {
						editingStock = null;
						resetForm();
					}}>Cancel</Button
				>
				<Button variant="primary" disabled={saving} onclick={handleEdit}>Save</Button>
			</div>
		</div>
	</Dialog>
{/if}

{#if deletingStock}
	<ConfirmDialog
		open={true}
		title="Delete Film Stock"
		message={`Permanently delete ${deletingStock.brand} ${deletingStock.name}?`}
		confirmLabel="Delete Film Stock"
		onconfirm={confirmDelete}
		oncancel={() => {
			deletingStock = null;
		}}
	/>
{/if}
