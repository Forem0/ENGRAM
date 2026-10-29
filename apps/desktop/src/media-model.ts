export interface MediaPreview {
  finding_id: string;
  data_url: string;
  width: number;
  height: number;
  source_width: number;
  source_height: number;
  source_sha256: string;
}

export interface BackupInspection {
  token: string;
  name: string;
  state: 'ready' | 'unavailable';
  encrypted: boolean | null;
  device_name: string | null;
  product_version: string | null;
  indexed_files: number;
  categories: { id: string; label: string; count: number }[];
  warnings: string[];
}
