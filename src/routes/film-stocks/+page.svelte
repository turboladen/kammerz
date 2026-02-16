<script lang="ts">
	import PageHeader from '$lib/components/layout/PageHeader.svelte';
	import Button from '$lib/components/ui/Button.svelte';
	import Input from '$lib/components/ui/Input.svelte';
	import Select from '$lib/components/ui/Select.svelte';
	import Textarea from '$lib/components/ui/Textarea.svelte';
	import Dialog from '$lib/components/ui/Dialog.svelte';
	import ConfirmDialog from '$lib/components/ui/ConfirmDialog.svelte';
	import ComboInput from '$lib/components/ui/ComboInput.svelte';
	import { listFilmStocks, createFilmStock, deleteFilmStock, listDistinctFilmBrands } from '$lib/api/film-stocks';
	import type { FilmStock, FilmStockInsert } from '$lib/types';

	let stocks: FilmStock[] = $state([]);
	let loading = $state(true);
	let showAddDialog = $state(false);
	let filterType = $state('all');
	let filterFormat = $state('all');
	let deletingStock: FilmStock | null = $state(null);
	let error = $state('');

	// Autocomplete options
	let filmBrandOptions: string[] = $state([]);

	const filtered = $derived(
		stocks.filter((s) => {
			if (filterType !== 'all' && s.stock_type !== filterType) return false;
			if (filterFormat !== 'all' && s.format !== filterFormat) return false;
			return true;
		})
	);

	const grouped = $derived(
		filtered.reduce(
			(acc, stock) => {
				const key = stock.brand;
				if (!acc[key]) acc[key] = [];
				acc[key].push(stock);
				return acc;
			},
			{} as Record<string, FilmStock[]>
		)
	);

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
		{ value: '8x10', label: '8x10' }
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
			const [s, brands] = await Promise.all([
				listFilmStocks(),
				listDistinctFilmBrands()
			]);
			stocks = s;
			filmBrandOptions = brands;
		} finally {
			loading = false;
		}
	}

	async function handleAdd() {
		error = '';
		try {
			const stock: FilmStockInsert = {
				brand,
				name,
				format,
				exposure_count: exposureCount ? parseInt(exposureCount) : null,
				stock_type: stockType,
				iso: iso ? parseInt(iso) : null,
				notes: notes || null
			};
			await createFilmStock(stock);
			showAddDialog = false;
			brand = '';
			name = '';
			format = '135';
			exposureCount = '';
			stockType = 'color-negative';
			iso = '';
			notes = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : String(err);
		}
	}

	function handleDelete(stock: FilmStock) {
		deletingStock = stock;
	}

	async function confirmDelete() {
		if (!deletingStock) return;
		error = '';
		try {
			await deleteFilmStock(deletingStock.id);
			deletingStock = null;
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
	<Button variant="primary" onclick={() => (showAddDialog = true)}>+ Add Film Stock</Button>
</PageHeader>

<div class="p-6">
	<!-- Filters -->
	<div class="mb-4 flex gap-4">
		<div class="flex gap-2">
			<Button size="sm" variant={filterType === 'all' ? 'primary' : 'ghost'} onclick={() => (filterType = 'all')}>All</Button>
			<Button size="sm" variant={filterType === 'color-negative' ? 'primary' : 'ghost'} onclick={() => (filterType = 'color-negative')}>Color</Button>
			<Button size="sm" variant={filterType === 'bw-negative' ? 'primary' : 'ghost'} onclick={() => (filterType = 'bw-negative')}>B&W</Button>
			<Button size="sm" variant={filterType === 'color-slide' ? 'primary' : 'ghost'} onclick={() => (filterType = 'color-slide')}>Slide</Button>
		</div>
		<div class="flex gap-2">
			<Button size="sm" variant={filterFormat === 'all' ? 'primary' : 'ghost'} onclick={() => (filterFormat = 'all')}>All Formats</Button>
			<Button size="sm" variant={filterFormat === '135' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '135')}>35mm</Button>
			<Button size="sm" variant={filterFormat === '120' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '120')}>120</Button>
			<Button size="sm" variant={filterFormat === '4x5' ? 'primary' : 'ghost'} onclick={() => (filterFormat = '4x5')}>4x5</Button>
		</div>
	</div>

	{#if loading}
		<p class="text-sm text-text-muted">Loading...</p>
	{:else}
		{#each Object.entries(grouped) as [brandName, brandStocks]}
			<div class="mb-6">
				<h2 class="mb-2 text-xs font-semibold uppercase tracking-wider text-text-muted">{brandName}</h2>
				<div class="grid gap-2">
					{#each brandStocks as stock}
						<div class="group flex items-center justify-between rounded-lg border border-border bg-surface-raised px-4 py-3 transition-all duration-150 hover:border-accent/40">
							<div class="flex items-center gap-3">
								<span class="font-medium">{stock.name}</span>
								{#if stock.iso}
									<span class="rounded bg-surface-overlay px-1.5 py-0.5 text-xs text-text-muted">ISO {stock.iso}</span>
								{/if}
								<span class="text-xs text-text-faint">{stock.format}</span>
								<span class="text-xs text-text-faint">{typeDisplayNames[stock.stock_type] ?? stock.stock_type}</span>
								{#if stock.exposure_count}
									<span class="text-xs text-text-faint">{stock.exposure_count} exp</span>
								{/if}
							</div>
							<Button size="sm" variant="ghost" class="opacity-0 group-hover:opacity-100" onclick={() => handleDelete(stock)}>&times;</Button>
						</div>
					{/each}
				</div>
			</div>
		{/each}
	{/if}
</div>

<Dialog bind:open={showAddDialog} title="Add Film Stock">
	<div class="space-y-4">
		<div class="grid grid-cols-2 gap-4">
			<ComboInput label="Brand" bind:value={brand} placeholder="Kodak" options={filmBrandOptions} />
			<Input label="Name" bind:value={name} placeholder="Portra 400" />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<Select label="Format" bind:value={format} options={formatOptions} />
			<Select label="Type" bind:value={stockType} options={typeOptions} />
		</div>
		<div class="grid grid-cols-2 gap-4">
			<Input label="ISO" bind:value={iso} type="number" placeholder="400" />
			<Input label="Exposure Count" bind:value={exposureCount} type="number" placeholder="36" hint="Leave empty for variable (120 film)" />
		</div>
		<Textarea label="Notes" bind:value={notes} />
		{#if error}
			<div class="rounded-lg bg-red-500/15 px-3 py-2 text-sm text-red-400">{error}</div>
		{/if}
		<div class="flex justify-end gap-2 pt-2">
			<Button variant="ghost" onclick={() => (showAddDialog = false)}>Cancel</Button>
			<Button variant="primary" onclick={handleAdd}>Add Film Stock</Button>
		</div>
	</div>
</Dialog>

{#if deletingStock}
	<ConfirmDialog
		open={true}
		title="Delete Film Stock"
		message={`Permanently delete ${deletingStock.brand} ${deletingStock.name}?`}
		confirmLabel="Delete Film Stock"
		onconfirm={confirmDelete}
		oncancel={() => { deletingStock = null; }}
	/>
{/if}
