// --- Cameras ---

export interface Camera {
	id: number;
	brand: string;
	model: string;
	prefix: string | null;
	format: string;
	camera_type: string | null;
	serial_number: string | null;
	date_purchased: string | null;
	purchased_from: string | null;
	date_sold: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type CameraInsert = Omit<Camera, 'id' | 'created_at' | 'updated_at'>;

export interface CameraMaintenance {
	id: number;
	camera_id: number;
	maintenance_type: string;
	done_by: string | null;
	date_done: string | null;
	cost: number | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type CameraMaintenanceInsert = Omit<CameraMaintenance, 'id' | 'created_at' | 'updated_at'>;

// --- Lenses ---

export interface Lens {
	id: number;
	brand: string;
	lens_system: string | null;
	name_on_lens: string | null;
	focal_length: string | null;
	max_aperture: string | null;
	min_aperture: string | null;
	filter_thread_front_mm: number | null;
	filter_thread_rear_mm: number | null;
	serial_number: string | null;
	date_purchased: string | null;
	purchased_from: string | null;
	date_sold: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type LensInsert = Omit<Lens, 'id' | 'created_at' | 'updated_at'>;

// --- Film Stocks ---

export interface FilmStock {
	id: number;
	brand: string;
	name: string;
	format: string;
	exposure_count: number | null;
	stock_type: string;
	iso: number | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type FilmStockInsert = Omit<FilmStock, 'id' | 'created_at' | 'updated_at'>;

// --- Labs ---

export interface Lab {
	id: number;
	name: string;
	location: string | null;
	website: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type LabInsert = Omit<Lab, 'id' | 'created_at' | 'updated_at'>;

// --- Rolls ---

export type RollStatus =
	| 'loaded'
	| 'shooting'
	| 'shot'
	| 'at-lab'
	| 'developing'
	| 'developed'
	| 'scanned'
	| 'archived';

export interface Roll {
	id: number;
	roll_id: string;
	camera_id: number | null;
	film_stock_id: number | null;
	status: RollStatus;
	frame_count: number | null;
	date_loaded: string | null;
	date_finished: string | null;
	date_fuzzy: string | null;
	push_pull: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type RollInsert = Omit<Roll, 'id' | 'created_at' | 'updated_at'>;

// Roll with joined data for display
export interface RollWithDetails extends Roll {
	camera_brand?: string;
	camera_model?: string;
	film_stock_brand?: string;
	film_stock_name?: string;
	film_stock_iso?: number;
}

// --- Shots ---

export interface Shot {
	id: number;
	roll_id: number;
	frame_number: string;
	aperture: string | null;
	shutter_speed: string | null;
	date: string | null;
	date_fuzzy: string | null;
	location: string | null;
	gps_lat: number | null;
	gps_lon: number | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type ShotInsert = Omit<Shot, 'id' | 'created_at' | 'updated_at'>;

// --- Development ---

export interface DevelopmentLab {
	id: number;
	roll_id: number;
	lab_id: number | null;
	date_dropped_off: string | null;
	date_received: string | null;
	cost: number | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type DevelopmentLabInsert = Omit<DevelopmentLab, 'id' | 'created_at' | 'updated_at'>;

export interface DevelopmentSelf {
	id: number;
	roll_id: number;
	date_processed: string | null;
	developer: string | null;
	developer_dilution: string | null;
	fixer: string | null;
	fixer_dilution: string | null;
	stop_bath: string | null;
	wetting_agent: string | null;
	clearing_agent: string | null;
	temperature: string | null;
	agitation_notes: string | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type DevelopmentSelfInsert = Omit<DevelopmentSelf, 'id' | 'created_at' | 'updated_at'>;

export interface DevStage {
	id: number;
	development_self_id: number;
	stage_name: string;
	duration_seconds: number | null;
	notes: string | null;
	sort_order: number;
}

export type DevStageInsert = Omit<DevStage, 'id'>;
