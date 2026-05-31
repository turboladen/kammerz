import { invoke } from '@tauri-apps/api/core';
import type { ParsedRoll, ImportRollDto, ModelInfo } from '$lib/types';

export const listModels = () => invoke<ModelInfo[]>('list_models');

export const parseNote = (noteText: string, model?: string) =>
	invoke<ParsedRoll>('parse_note', { noteText, model });

export const importParsedRoll = (data: ImportRollDto) =>
	invoke<number>('import_parsed_roll', { data });
