export type Category =
  'media' | 'secret' | 'document' | 'browser' | 'message' | 'location' | 'database';
export type Confidence = 'observed' | 'linked' | 'candidate';
export type ModuleState = 'ready' | 'experimental' | 'planned' | 'unavailable';
export type View = 'map' | 'findings' | 'timeline' | 'connections' | 'compare' | 'modules';

export interface Finding {
  id: string;
  category: Category;
  title: string;
  path: string;
  origin: string;
  summary: string;
  locator: string;
  observed_at: number | null;
  bytes: number;
  sha256: string | null;
  confidence: Confidence;
  rule_id: string;
  sensitive: boolean;
  evidence: { label: string; value: string }[];
  related_ids: string[];
}

export interface Case {
  schema_version: number;
  id: string;
  title: string;
  source_root: string;
  source_kind: 'folder' | 'ios_backup' | 'demo';
  created_at: number;
  completed_at: number;
  findings: Finding[];
  warnings: string[];
  stats: {
    files_examined: number;
    bytes_examined: number;
    skipped_files: number;
    elapsed_ms: number;
  };
  modules: { name: string; state: ModuleState; detail: string }[];
  events: { at_ms: number; message: string; category: Category | null }[];
}

export interface CaseDiff {
  added: string[];
  removed: string[];
  changed: string[];
  unchanged: string[];
}

export interface Comparison {
  before: Case;
  diff: CaseDiff;
}
export interface RuntimeInfo {
  version: string;
  engine: string;
  schema_version: number;
  initial_view: string;
  smoke_test: boolean;
}

export const CATEGORIES: { id: Category; label: string; color: string; short: string }[] = [
  { id: 'media', label: 'Media', color: '#558778', short: 'ME' },
  { id: 'secret', label: 'Secrets', color: '#b07751', short: 'SE' },
  { id: 'document', label: 'Documents', color: '#8b9464', short: 'DO' },
  { id: 'browser', label: 'Browser', color: '#6b82a4', short: 'BR' },
  { id: 'message', label: 'Messages', color: '#9981a3', short: 'MS' },
  { id: 'location', label: 'Location', color: '#9a8d63', short: 'LO' },
  { id: 'database', label: 'Databases', color: '#787e79', short: 'DB' },
];

export const VIEWS: { id: View; name: string; subtitle: string; description: string }[] = [
  {
    id: 'map',
    name: 'Memory map',
    subtitle: 'What remains, connected.',
    description:
      'A map of the artifacts preserved in this source. Every connection has evidence behind it.',
  },
  {
    id: 'findings',
    name: 'Findings',
    subtitle: 'Follow the evidence.',
    description:
      'Search the observed artifacts, inspect their source, and see the rule behind each finding.',
  },
  {
    id: 'timeline',
    name: 'Timeline',
    subtitle: 'Put the traces in order.',
    description:
      'Available timestamps retain their source meaning. An observation date is not a deletion date.',
  },
  {
    id: 'connections',
    name: 'Connections',
    subtitle: 'Nothing in isolation.',
    description:
      'Explore relationships established by the analysis engine, with the underlying records in reach.',
  },
  {
    id: 'compare',
    name: 'Compare cases',
    subtitle: 'See what changed.',
    description:
      'Compare the open case with a saved baseline. Two snapshots describe two observations.',
  },
  {
    id: 'modules',
    name: 'Modules',
    subtitle: 'Know the boundaries.',
    description:
      'Reader availability, source coverage and the limits of this inspection, in plain sight.',
  },
];

export function categoryInfo(id: Category) {
  return CATEGORIES.find((category) => category.id === id) ?? CATEGORIES[6];
}
export function asView(value: string): View {
  return VIEWS.some((view) => view.id === value) ? (value as View) : 'map';
}
