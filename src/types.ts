export type MediaMode = "video" | "audio";

export interface VideoInfo {
  id: string;
  title: string;
  uploader: string;
  duration: number | null;
  thumbnail: string | null;
  webpageUrl: string;
  extractor: string;
  viewCount: number | null;
  isLive: boolean;
}

export interface DependencyStatus {
  ytDlp: boolean;
  ffmpeg: boolean;
  ffprobe: boolean;
  ytDlpVersion: string | null;
  defaultDownloadDir: string;
  whisper: boolean;
  modelReady: boolean;
  device?: DeviceProfile;
  models?: ModelInfo[];
}

export interface DownloadRequest {
  jobId: string;
  url: string;
  outputDir: string;
  mode: MediaMode;
  quality: string;
  format: string;
  playlist: boolean;
  subtitles: boolean;
  subtitleLanguage?: string;
  saveThumbnail: boolean;
}

export type DownloadEventKind =
  | "started"
  | "progress"
  | "processing"
  | "completed"
  | "cancelled"
  | "error";

export interface DownloadEvent {
  kind: DownloadEventKind;
  jobId: string;
  progress: number | null;
  speed: string | null;
  eta: string | null;
  message: string;
  outputPath?: string | null;
}

export interface DownloadJob {
  id: string;
  title: string;
  thumbnail: string | null;
  mode: MediaMode;
  format: string;
  progress: number;
  speed: string;
  eta: string;
  status: "queued" | "downloading" | "processing" | "completed" | "cancelled" | "error";
  message: string;
  outputPath?: string | null;
}

export interface MediaInfo { path: string; title: string; duration: number; playbackUrl: string }
export interface TranscriptSegment { start: number; end: number; text: string }
export interface Transcript {
  id: string;
  mediaPath: string;
  mediaTitle: string;
  createdAt: number;
  language: string;
  segments: TranscriptSegment[];
}
export interface StudioEvent { phase: string; progress: number | null; message: string }

export interface GpuInfo { name:string; dedicatedMemory:number|null }
export interface DeviceProfile { cpuName:string;physicalCores:number;logicalCores:number;totalMemory:number;availableMemory:number;gpus:GpuInfo[];tier:"efficient"|"balanced"|"quality";recommendedModel:string;threads:number;gpuAcceleration:boolean }
export interface ModelInfo {id:string;bytes:number;installed:boolean;minimumMemory:number}

export interface Translation {transcriptId:string;targetLanguage:string;segments:TranscriptSegment[]}
export interface TranslationStatus {email:string;dailyLimit:number;offlineSupported:boolean;offlineInstalled:boolean;offlineBytes:number;offlineEngine:boolean}
