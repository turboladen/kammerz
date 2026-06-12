import type { ParsedRoll, ImportRollDto, ModelInfo } from '$lib/types';
import { request } from './client';

export const listModels = () => request<ModelInfo[]>('GET', '/api/import/models');

export const parseNote = (noteText: string, model?: string) =>
	request<ParsedRoll>('POST', '/api/import/parse', { note_text: noteText, model });

export const importParsedRoll = (data: ImportRollDto) => request<number>('POST', '/api/import/roll', data);
