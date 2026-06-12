<script lang="ts">
	import { X } from 'lucide-svelte';

	// A small, transient, inline notice — used to acknowledge invisible side effects
	// (e.g. the backend auto-advancing a roll's status after a mutation). Deliberately
	// inline (never position: fixed) so it doesn't get trapped by an ancestor FadeIn's
	// stacking context (see CLAUDE.md / UI_DESIGN.md). aria-live="polite" announces the
	// message to screen readers without stealing focus.
	let {
		message = $bindable(''),
		seq = 0,
		duration = 6000
	}: {
		/** The notice text. Empty string renders nothing. Cleared on auto-dismiss. */
		message: string;
		/** Bump on each set to restart the timer even when the text is unchanged — two
		 * back-to-back identical messages wouldn't otherwise re-trigger the effect, so the
		 * second would inherit the first's nearly-expired timer and vanish immediately. */
		seq?: number;
		/** Auto-dismiss delay in ms. */
		duration?: number;
	} = $props();

	// Auto-dismiss: restart the timer whenever a new message appears (tracking both the
	// text and the seq nonce), and clear it on unmount or when dismissed early — so a
	// stale timer can't wipe a freshly-set notice.
	$effect(() => {
		// Read seq so an identical-text re-set still restarts the timer.
		seq;
		if (!message) return;
		const timer = setTimeout(() => {
			message = '';
		}, duration);
		return () => clearTimeout(timer);
	});
</script>

{#if message}
	<div
		role="status"
		aria-live="polite"
		class="flex items-start justify-between gap-3 rounded-lg border-l-2 border-accent/40 bg-accent/5 px-3 py-2 text-xs text-text-muted"
	>
		<span>{message}</span>
		<button
			onclick={() => (message = '')}
			aria-label="Dismiss notice"
			class="-mr-0.5 shrink-0 text-text-faint transition-colors hover:text-text"
		>
			<X size={14} strokeWidth={2} aria-hidden="true" />
		</button>
	</div>
{/if}
