<script lang="ts">
	import '../app.css';

	let { children } = $props();
</script>

<svelte:head>
	<title>Kammerz</title>
</svelte:head>

{@render children()}
