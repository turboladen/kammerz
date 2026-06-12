import type { Camera, CameraInsert, CameraMaintenance, CameraMaintenanceInsert } from '$lib/types';
import { request } from './client';

export const listCameras = () => request<Camera[]>('GET', '/api/cameras');
export const getCamera = (id: number) => request<Camera | null>('GET', `/api/cameras/${id}`);
export const createCamera = (data: CameraInsert) => request<number>('POST', '/api/cameras', data);
export const updateCamera = (id: number, data: Partial<CameraInsert>) =>
	request<void>('PUT', `/api/cameras/${id}`, data);
export const deleteCamera = (id: number) => request<void>('DELETE', `/api/cameras/${id}`);

// --- Maintenance Records ---

export const listMaintenanceForCamera = (cameraId: number) =>
	request<CameraMaintenance[]>('GET', `/api/cameras/${cameraId}/maintenance`);
export const createMaintenance = (data: CameraMaintenanceInsert) => request<number>('POST', '/api/maintenance', data);
export const updateMaintenance = (id: number, data: Partial<CameraMaintenanceInsert>) =>
	request<void>('PUT', `/api/maintenance/${id}`, data);
export const deleteMaintenance = (id: number) => request<void>('DELETE', `/api/maintenance/${id}`);

// --- Create camera with fixed lens (transactional) ---

export interface CreateCameraWithLensData {
	camera: CameraInsert;
	lens_model: string | null;
	lens_focal_length: string | null;
	lens_max_aperture: string | null;
}
export const createCameraWithLens = (data: CreateCameraWithLensData) =>
	request<number>('POST', '/api/cameras/with-lens', data);

// --- Camera-Lens Associations ---

export const getLensesForCamera = (cameraId: number) => request<number[]>('GET', `/api/cameras/${cameraId}/lenses`);
export const linkLensToCamera = (cameraId: number, lensId: number) =>
	request<void>('POST', `/api/cameras/${cameraId}/lenses/${lensId}`);
export const unlinkLensFromCamera = (cameraId: number, lensId: number) =>
	request<void>('DELETE', `/api/cameras/${cameraId}/lenses/${lensId}`);

// --- Distinct value helpers ---

export const listDistinctCameraBrands = () => request<string[]>('GET', '/api/cameras/distinct/brands');
export const listDistinctVendors = () => request<string[]>('GET', '/api/cameras/distinct/vendors');
export const listDistinctMaintProviders = () => request<string[]>('GET', '/api/cameras/distinct/maint-providers');
