export type Confidence = 'confirmed' | 'high_probability' | 'unknown' | 'conflict';
export type RuntimeType = 'mono' | 'il2cpp' | 'unknown';
export type Architecture = 'x86' | 'x64' | 'arm64' | 'unknown';
export type TargetPlatform = 'windows' | 'linux' | 'macos' | 'unknown';

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
