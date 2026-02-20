import { invoke } from '@tauri-apps/api/core';
import type { LensMount } from '$lib/types';

export const listLensMounts = () => invoke<LensMount[]>('list_lens_mounts');

export const createLensMount = (name: string) => invoke<number>('create_lens_mount', { name });
