import { describe, expect, it } from 'vitest';
import { filterBySearch, groupItems, sortByDate, sortByNumber, sortByString } from './list';

describe('filterBySearch', () => {
	const items = [{ name: 'Nikon F3' }, { name: 'Leica M6' }, { name: 'nikon FM2' }];
	const text = (i: { name: string }) => i.name;

	it('returns all items for an empty or whitespace query', () => {
		expect(filterBySearch(items, '', text)).toBe(items);
		expect(filterBySearch(items, '   ', text)).toBe(items);
	});

	it('matches case-insensitively and trims the query', () => {
		expect(filterBySearch(items, '  NIKON  ', text).map(text)).toEqual(['Nikon F3', 'nikon FM2']);
	});

	it('returns an empty array when nothing matches', () => {
		expect(filterBySearch(items, 'hasselblad', text)).toEqual([]);
	});
});

describe('groupItems', () => {
	it('groups by key, preserving first-appearance (sort) order of keys', () => {
		const items = [
			{ brand: 'Nikon', m: 'F3' },
			{ brand: 'Leica', m: 'M6' },
			{ brand: 'Nikon', m: 'FM2' }
		];
		const grouped = groupItems(items, (i) => i.brand);
		expect(Object.keys(grouped)).toEqual(['Nikon', 'Leica']);
		expect(grouped.Nikon).toHaveLength(2);
		expect(grouped.Leica).toHaveLength(1);
	});
});

describe('sortByString', () => {
	const items = [{ k: 'banana' }, { k: 'Apple' }, { k: null }, { k: 'cherry' }];
	const key = (i: { k: string | null }) => i.k;

	it('sorts ascending case-insensitively with nulls last', () => {
		expect(sortByString(items, key).map(key)).toEqual(['Apple', 'banana', 'cherry', null]);
	});

	it('sorts descending but still keeps nulls last', () => {
		expect(sortByString(items, key, 'desc').map(key)).toEqual(['cherry', 'banana', 'Apple', null]);
	});

	it('does not mutate the input array', () => {
		const input = [{ k: 'b' }, { k: 'a' }];
		const snapshot = [...input];
		sortByString(input, key);
		expect(input).toEqual(snapshot);
	});
});

describe('sortByDate', () => {
	const items = [{ d: '2026-03-01' }, { d: '2026-01-01' }, { d: null }, { d: '' }];
	const key = (i: { d: string | null }) => i.d;

	it('defaults to descending (newest first) with empty/null last', () => {
		expect(sortByDate(items, key).map(key)).toEqual(['2026-03-01', '2026-01-01', null, '']);
	});

	it('sorts ascending when asked, empty/null still last', () => {
		expect(sortByDate(items, key, 'asc').map(key)).toEqual(['2026-01-01', '2026-03-01', null, '']);
	});
});

describe('sortByNumber', () => {
	const items = [{ n: 3 }, { n: 1 }, { n: null }, { n: 0 }];
	const key = (i: { n: number | null }) => i.n;

	it('sorts ascending with nulls last and treats 0 as a real value (not missing)', () => {
		expect(sortByNumber(items, key).map(key)).toEqual([0, 1, 3, null]);
	});

	it('sorts descending with nulls last', () => {
		expect(sortByNumber(items, key, 'desc').map(key)).toEqual([3, 1, 0, null]);
	});
});
