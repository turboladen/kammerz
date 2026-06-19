// --- Lens Mounts ---

export interface LensMount {
	id: number;
	name: string;
	created_at: string;
	updated_at: string;
}

// --- Cameras ---

export type CameraFormat =
	| '35mm'
	| 'medium format'
	| '6x4.5'
	| '6x6'
	| '6x7'
	| '6x8'
	| '6x9'
	| 'large format'
	| '4x5'
	| '5x7'
	| '8x10'
	| 'instant';

export type CameraType = 'SLR' | 'rangefinder' | 'TLR' | 'point-and-shoot' | 'box' | 'view' | 'instant';

export type MaintenanceType = 'CLA' | 'repair' | 'cleaning' | 'modification' | 'other';

export interface Camera {
	id: number;
	brand: string;
	model: string;
	prefix: string | null;
	format: CameraFormat;
	lens_mount_id: number;
	default_lens_id: number | null;
	camera_type: CameraType | null;
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
	maintenance_type: MaintenanceType;
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
	lens_mount_id: number;
	lens_system: string | null;
	model: string | null;
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

export type FilmFormat = '135' | '120' | '4x5' | '5x7' | '8x10' | 'instant';

export type FilmStockType = 'color-negative' | 'bw-negative' | 'color-slide' | 'bw-slide';

export interface FilmStock {
	id: number;
	brand: string;
	name: string;
	format: FilmFormat;
	exposure_count: number | null;
	stock_type: FilmStockType;
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
	| 'lab-done'
	| 'developing'
	| 'developed'
	| 'scanned'
	| 'post-processed'
	| 'archived';

export type PushPull = '-2' | '-1' | '+1' | '+2' | '+3';

export interface Roll {
	id: number;
	roll_id: string;
	camera_id: number | null;
	film_stock_id: number | null;
	lens_id: number | null;
	status: RollStatus;
	frame_count: number | null;
	date_loaded: string | null;
	date_finished: string | null;
	date_scanned: string | null;
	date_post_processed: string | null;
	date_archived: string | null;
	date_fuzzy: string | null;
	push_pull: PushPull | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type RollInsert = Omit<Roll, 'id' | 'created_at' | 'updated_at'>;

// Roll with joined data for display
export interface RollWithDetails extends Roll {
	camera_brand: string | null;
	camera_model: string | null;
	film_stock_brand: string | null;
	film_stock_name: string | null;
	film_stock_iso: number | null;
	lens_brand: string | null;
	lens_name: string | null;
	shot_count: number;
}

export type RollEventType =
	| 'roll_loaded'
	| 'status_changed'
	| 'shot_logged'
	| 'shot_edited'
	| 'shot_deleted'
	| 'lab_dev_added'
	| 'lab_dev_edited'
	| 'lab_dev_removed'
	| 'self_dev_added'
	| 'self_dev_edited'
	| 'self_dev_removed';

export type RollEventRefKind = 'lab_dev' | 'self_dev' | 'shot';

export interface RollEvent {
	id: number;
	roll_id: number;
	event_type: RollEventType;
	from_status: RollStatus | null;
	to_status: RollStatus | null;
	ref_kind: RollEventRefKind | null;
	ref_id: number | null;
	summary: string;
	occurred_at: string;
	created_at: string;
}

// Composite roll detail (single IPC call for roll detail page)
export interface RollDetail {
	roll: RollWithDetails;
	shots: Shot[];
	shot_lens_pairs: [number, number][];
	lab_dev: DevelopmentLab | null;
	self_dev: DevelopmentSelf | null;
	dev_stages: DevStage[];
	events: RollEvent[];
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
	// Optional during rollout: the API always returns it (nullable), but insert
	// sites need not set it until the entry UI lands (see feat/shot-time follow-up).
	time?: string | null;
	location: string | null;
	gps_lat: number | null;
	gps_lon: number | null;
	notes: string | null;
	created_at: string;
	updated_at: string;
}

export type ShotInsert = Omit<Shot, 'id' | 'created_at' | 'updated_at'>;

/** Shot with its associated lens IDs, used on roll detail page */
export interface ShotWithLensIds extends Shot {
	lens_ids: number[];
}

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

// Self-development with joined roll/film stock/camera context + stages
export interface SelfDevListItem {
	dev_id: number;
	roll_pk: number;
	roll_id: string;
	roll_status: RollStatus;
	film_stock_brand: string | null;
	film_stock_name: string | null;
	film_stock_iso: number | null;
	film_stock_type: FilmStockType | null;
	camera_brand: string | null;
	camera_model: string | null;
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
	dev_date: string | null;
	created_at: string;
	stages: DevStage[];
}

// Lab-development with joined roll/film stock/camera/lab context. Lab devs have
// no stages, so (unlike SelfDevListItem) there is no stages array.
export interface LabDevListItem {
	dev_id: number;
	roll_pk: number;
	roll_id: string;
	roll_status: RollStatus;
	film_stock_brand: string | null;
	film_stock_name: string | null;
	film_stock_iso: number | null;
	film_stock_type: FilmStockType | null;
	camera_brand: string | null;
	camera_model: string | null;
	lab_name: string | null;
	date_dropped_off: string | null;
	date_received: string | null;
	cost: number | null;
	notes: string | null;
	dev_date: string | null;
	created_at: string;
}

// --- Search ---

export interface CameraSearchResult {
	id: number;
	brand: string;
	model: string;
	format: string;
	match_field: string;
	match_snippet: string;
}

export interface LensSearchResult {
	id: number;
	brand: string;
	model: string | null;
	focal_length: string | null;
	match_field: string;
	match_snippet: string;
}

export interface FilmStockSearchResult {
	id: number;
	brand: string;
	name: string;
	format: string;
	stock_type: string;
	match_field: string;
	match_snippet: string;
}

export interface RollSearchResult {
	id: number;
	roll_id: string;
	status: RollStatus;
	camera_brand: string | null;
	camera_model: string | null;
	film_stock_brand: string | null;
	film_stock_name: string | null;
	match_field: string;
	match_snippet: string;
}

export interface ShotSearchResult {
	id: number;
	frame_number: string;
	roll_pk: number;
	roll_id_display: string;
	aperture: string | null;
	location: string | null;
	match_field: string;
	match_snippet: string;
}

export interface LabSearchResult {
	id: number;
	name: string;
	location: string | null;
	match_field: string;
	match_snippet: string;
}

export interface SearchResults {
	cameras: CameraSearchResult[];
	lenses: LensSearchResult[];
	film_stocks: FilmStockSearchResult[];
	rolls: RollSearchResult[];
	shots: ShotSearchResult[];
	labs: LabSearchResult[];
}

// --- Statistics ---

export interface MonthCount {
	month: string;
	count: number;
}

export interface RankedItem {
	label: string;
	count: number;
}

export interface CatalogStats {
	total_rolls: number;
	total_shots: number;
	total_cameras: number;
	total_lenses: number;
	total_lab_dev_cost: number;
	total_maintenance_cost: number;
	total_cost: number;
	rolls_per_month: MonthCount[];
	top_film_stocks: RankedItem[];
	top_cameras: RankedItem[];
	top_lenses: RankedItem[];
	rolls_by_format: RankedItem[];
	rolls_by_status: RankedItem[];
	rolls_by_mount: RankedItem[];
}

// --- AI Import ---

export interface ModelInfo {
	id: string;
	display_name: string;
}

export interface ParsedRoll {
	roll_id: string;
	film_stock_guess: string | null;
	camera_prefix_guess: string | null;
	lens_guess: string | null;
	frame_count: number | null;
	date_loaded: string | null;
	date_finished: string | null;
	notes: string | null;
	shots: ParsedShot[];
}

export interface ParsedShot {
	frame_number: string;
	aperture: string | null;
	shutter_speed: string | null;
	date: string | null;
	focal_length: string | null;
	location: string | null;
	notes: string | null;
}

export interface ImportRollDto {
	roll_id: string;
	camera_id: number | null;
	film_stock_id: number | null;
	lens_id: number | null;
	status: RollStatus;
	frame_count: number | null;
	date_loaded: string | null;
	date_finished: string | null;
	date_fuzzy: string | null;
	push_pull: PushPull | null;
	notes: string | null;
	shots: ImportShotDto[];
}

export interface ImportShotDto {
	frame_number: string;
	aperture: string | null;
	shutter_speed: string | null;
	date: string | null;
	date_fuzzy: string | null;
	location: string | null;
	notes: string | null;
	lens_ids: number[] | null;
}
