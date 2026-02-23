import { invoke } from '@tauri-apps/api/core';
import type { Camera, CameraInsert, CameraMaintenance, CameraMaintenanceInsert } from '$lib/types';

export const listCameras = () => invoke<Camera[]>('list_cameras');

export const getCamera = (id: number) => invoke<Camera | null>('get_camera', { id });

export const createCamera = (data: CameraInsert) => invoke<number>('create_camera', { data });

export const updateCamera = (id: number, data: Partial<CameraInsert>) =>
	invoke<void>('update_camera', { id, data });

export const deleteCamera = (id: number) => invoke<void>('delete_camera', { id });

// --- Maintenance Records ---

export const listMaintenanceForCamera = (cameraId: number) =>
	invoke<CameraMaintenance[]>('list_maintenance', { cameraId });

export const createMaintenance = (data: CameraMaintenanceInsert) =>
	invoke<number>('create_maintenance', { data });

export const updateMaintenance = (id: number, data: Partial<CameraMaintenanceInsert>) =>
	invoke<void>('update_maintenance', { id, data });

export const deleteMaintenance = (id: number) => invoke<void>('delete_maintenance', { id });

// --- Create camera with fixed lens (transactional) ---

export interface CreateCameraWithLensData {
	camera: CameraInsert;
	lens_model: string | null;
	lens_focal_length: string | null;
	lens_max_aperture: string | null;
}

export const createCameraWithLens = (data: CreateCameraWithLensData) =>
	invoke<number>('create_camera_with_lens', { data });

// --- Camera-Lens Associations ---

export const getLensesForCamera = (cameraId: number) =>
	invoke<number[]>('get_lenses_for_camera', { cameraId });

export const linkLensToCamera = (cameraId: number, lensId: number) =>
	invoke<void>('link_lens_to_camera', { cameraId, lensId });

export const unlinkLensFromCamera = (cameraId: number, lensId: number) =>
	invoke<void>('unlink_lens_from_camera', { cameraId, lensId });

// --- Distinct value helpers ---

export const listDistinctCameraBrands = () => invoke<string[]>('list_distinct_camera_brands');

export const listDistinctVendors = () => invoke<string[]>('list_distinct_vendors');

export const listDistinctMaintProviders = () => invoke<string[]>('list_distinct_maint_providers');
