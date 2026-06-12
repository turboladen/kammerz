import type { ActionReturn } from 'svelte/action';

interface DialogFocusOptions {
	/** Called when Escape is pressed while this dialog is the topmost open dialog. */
	onEscape: () => void;
}

/** Stack of open dialog panels — only the topmost one responds to keyboard events. */
const stack: HTMLElement[] = [];

let savedBodyOverflow = '';

const FOCUSABLE_SELECTOR = [
	'a[href]',
	'button:not([disabled])',
	'input:not([disabled]):not([type="hidden"])',
	'select:not([disabled])',
	'textarea:not([disabled])',
	'[tabindex]:not([tabindex="-1"])'
].join(', ');

function focusables(node: HTMLElement): HTMLElement[] {
	return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
		(el) => el.getClientRects().length > 0
	);
}

/**
 * Svelte action providing modal focus management for dialog panels:
 *
 * - On mount: saves the previously focused element and moves focus to the panel
 *   (the panel must have `tabindex="-1"`).
 * - While open: listens for Escape on `window` (so it works even before the user
 *   clicks into the dialog) and wraps Tab / Shift-Tab focus inside the panel.
 * - Locks body scroll while any dialog is open.
 * - On destroy: restores focus to the previously focused element.
 *
 * Stacked dialogs (e.g. a ConfirmDialog over a Dialog) are handled via a module-level
 * stack — only the topmost dialog responds to Escape/Tab.
 */
export function dialogFocus(
	node: HTMLElement,
	options: DialogFocusOptions
): ActionReturn<DialogFocusOptions> {
	let { onEscape } = options;
	const previouslyFocused =
		document.activeElement instanceof HTMLElement ? document.activeElement : null;

	stack.push(node);
	if (stack.length === 1) {
		savedBodyOverflow = document.body.style.overflow;
		document.body.style.overflow = 'hidden';
	}

	// Move focus into the dialog so keyboard and screen-reader users start inside it.
	node.focus();

	function handleKeydown(e: KeyboardEvent) {
		if (stack[stack.length - 1] !== node) return;
		if (e.key === 'Escape') {
			e.preventDefault();
			onEscape();
			return;
		}
		if (e.key !== 'Tab') return;
		const items = focusables(node);
		if (items.length === 0) {
			e.preventDefault();
			node.focus();
			return;
		}
		const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
		const first = items[0];
		const last = items[items.length - 1];
		const inside = active !== null && active !== node && node.contains(active);
		if (e.shiftKey) {
			if (!inside || active === first) {
				e.preventDefault();
				last.focus();
			}
		} else if (!inside || active === last) {
			e.preventDefault();
			first.focus();
		}
	}

	window.addEventListener('keydown', handleKeydown);

	return {
		update(next: DialogFocusOptions) {
			({ onEscape } = next);
		},
		destroy() {
			window.removeEventListener('keydown', handleKeydown);
			const i = stack.indexOf(node);
			if (i !== -1) stack.splice(i, 1);
			if (stack.length === 0) {
				document.body.style.overflow = savedBodyOverflow;
			}
			if (previouslyFocused && document.contains(previouslyFocused)) {
				previouslyFocused.focus();
			}
		}
	};
}
