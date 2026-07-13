// Pure derivation of a lab dev's negatives-pickup state. Single source of truth
// for the dashboard banner/section, roll-list badges, and the roll-detail card.
// `today` is injected so the countdown is deterministic in tests and live in the
// UI (callers pass `new Date()`).

export type NegativesStatus = 'na' | 'awaiting' | 'overdue' | 'picked-up' | 'waived';
export type NegativesTier = 'none' | 'far' | 'near' | 'soon' | 'overdue';

export interface NegativesInput {
	negatives_date_received: string | null;
	negatives_deadline: string | null;
	date_negatives_picked_up: string | null;
	negatives_not_collecting: boolean | null;
}

export interface NegativesView {
	status: NegativesStatus;
	/** date_received + retention, or null when the order isn't complete. */
	deadline: string | null;
	/** Whole days until the deadline (negative once overdue); null unless awaiting/overdue. */
	daysLeft: number | null;
	tier: NegativesTier;
	/** Short badge text; '' when there is nothing to show. */
	label: string;
}

function midnight(d: Date): number {
	return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

// Parse a 'YYYY-MM-DD' as a local date (avoids the UTC shift of `new Date(str)`).
function parseLocalDate(s: string): Date {
	const [y, m, d] = s.split('-').map(Number);
	return new Date(y, m - 1, d);
}

export function negativesState(input: NegativesInput, today: Date): NegativesView {
	const deadline = input.negatives_deadline;

	if (input.negatives_not_collecting) {
		return { status: 'waived', deadline, daysLeft: null, tier: 'none', label: 'Not collecting' };
	}
	if (input.date_negatives_picked_up) {
		return { status: 'picked-up', deadline, daysLeft: null, tier: 'none', label: 'Collected' };
	}
	if (!input.negatives_date_received || !deadline) {
		return { status: 'na', deadline: null, daysLeft: null, tier: 'none', label: '' };
	}

	const daysLeft = Math.round((midnight(parseLocalDate(deadline)) - midnight(today)) / 86_400_000);

	if (daysLeft < 0) {
		return { status: 'overdue', deadline, daysLeft, tier: 'overdue', label: 'OVERDUE' };
	}
	const tier: NegativesTier = daysLeft <= 3 ? 'soon' : daysLeft <= 7 ? 'near' : 'far';
	const label = daysLeft === 0 ? 'Due today' : `${daysLeft}d left`;
	return { status: 'awaiting', deadline, daysLeft, tier, label };
}

export function isNegativesPending(v: NegativesView): boolean {
	return v.status === 'awaiting' || v.status === 'overdue';
}
