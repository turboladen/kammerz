// Types the canonical status-flow fixture so its arrays import as `RollStatus[]`
// rather than the widened `string[]` a raw JSON import yields. The fixture's
// CONTENTS are gated by the Rust `tests/status_flows.rs` (every entry must equal
// a `RollStatus` enum variant's serialized value); this declaration only fixes
// the element type for frontend consumers so no `as RollStatus[]` casts are
// needed in `status.ts`. The shape here must match `status-flows.json`.
declare module '$lib/status-flows.json' {
	import type { RollStatus } from '$lib/types';
	// Element type is `RollStatus[]` (not `readonly`) so the derived exports in
	// status.ts keep their existing mutable-array types and every consumer stays
	// untouched. The fixture is never mutated at runtime; the typing is purely to
	// avoid the widened `string[]` a raw JSON import produces.
	const flows: {
		statuses: RollStatus[];
		labFlow: RollStatus[];
		selfFlow: RollStatus[];
		undecidedFlow: RollStatus[];
	};
	export default flows;
}
