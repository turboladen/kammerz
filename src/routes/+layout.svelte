<script lang="ts">
	import '../app.css';
	import Sidebar from '$lib/components/layout/Sidebar.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<title>Kammerz</title>
</svelte:head>

<div class="flex h-screen overflow-hidden">
	<Sidebar />
	<main class="flex flex-1 flex-col overflow-y-auto">
		{@render children()}
	</main>
</div>
