import { invoke } from '@tauri-apps/api/core';

export const getSetting = (key: string) => invoke<string | null>('get_setting', { key });

export const setSetting = (key: string, value: string) =>
	invoke<void>('set_setting', { key, value });
