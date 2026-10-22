export type Confidence = 'confirmed' | 'high_probability' | 'unknown' | 'conflict';
export type RuntimeType = 'mono' | 'il2cpp' | 'unknown';
export type Architecture = 'x86' | 'x64' | 'arm64' | 'unknown';
export type TargetPlatform = 'windows' | 'linux' | 'macos' | 'unknown';
export type BuildChannel = 'release' | 'bleedingedge';
export type CompatibilityStatus = 'compatible' | 'partiallycompatible' | 'incompatible';

export interface EvidenceItem {
  key: string;
  description: string;
  confidence: Confidence;
  path?: string;
}

export interface UnityVersionInfo {
  raw: string;
  major: number;
  minor: number;
  patch: number;
  suffix?: string;
}

export interface EnvironmentReport {
  game_path: string;
  data_dir?: string;
  unity_version?: UnityVersionInfo;
  runtime_type: RuntimeType;
  architecture: Architecture;
  target_platform: TargetPlatform;
  installed_frameworks: string[];
  evidences: EvidenceItem[];
  overall_confidence: Confidence;
}

export interface BepInExBuild {
  id: string;
  version: string;
  channel: BuildChannel;
  runtime_type: RuntimeType;
  architecture: Architecture;
  target_platform: TargetPlatform;
  download_url: string;
  sha256: string;
  filename?: string;
}

export interface MatchReason {
  aspect: string;
  state: 'matched' | 'mismatched' | 'unknown';
  detail: string;
}

export interface CompatibilityWarning {
  code: string;
  message: string;
  severity: 'info' | 'warning' | 'risk';
}

export interface CompatibilityAssessment {
  build: BepInExBuild;
  status: CompatibilityStatus;
  reasons: MatchReason[];
  warnings: CompatibilityWarning[];
}

export interface CompatibilityReport {
  assessments: CompatibilityAssessment[];
  recommended_build?: BepInExBuild;
  global_warnings: CompatibilityWarning[];
}
