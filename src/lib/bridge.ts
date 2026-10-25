import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { readText } from "@tauri-apps/plugin-clipboard-manager";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { DependencyStatus, DownloadEvent, DownloadRequest, VideoInfo, MediaInfo, Transcript, StudioEvent, DeviceProfile } from "../types";

export const runningInTauri = isTauri();

export async function getDependencyStatus(): Promise<DependencyStatus> {
  if (!runningInTauri) {
    return {
      ytDlp: false,
      ffmpeg: false,
      whisper: false,
      modelReady: false,
      ffprobe: false,
      ytDlpVersion: null,
      defaultDownloadDir: "İndirilenler",
    };
  }
  return invoke<DependencyStatus>("check_dependencies");
}

export async function analyzeUrl(url: string): Promise<VideoInfo> {
  if (!runningInTauri) {
    throw new Error("Video analizi masaüstü uygulamasında kullanılabilir.");
  }
  return invoke<VideoInfo>("analyze_url", { url });
}

export async function beginDownload(
  request: DownloadRequest,
  onEvent: (event: DownloadEvent) => void,
): Promise<void> {
  if (!runningInTauri) {
    throw new Error("İndirme özelliği masaüstü uygulamasında kullanılabilir.");
  }

  const channel = new Channel<DownloadEvent>();
  channel.onmessage = onEvent;
  await invoke("start_download", { request, onEvent: channel });
}

export async function cancelDownload(jobId: string): Promise<void> {
  if (runningInTauri) await invoke("cancel_download", { jobId });
}

export async function chooseDownloadFolder(defaultPath?: string): Promise<string | null> {
  if (!runningInTauri) return null;
  const selected = await open({ directory: true, multiple: false, defaultPath });
  return typeof selected === "string" ? selected : null;
}

export async function readClipboardUrl(): Promise<string> {
  if (runningInTauri) return (await readText()) ?? "";
  return navigator.clipboard?.readText?.() ?? "";
}

export async function revealDownloads(path: string): Promise<void> {
  if (runningInTauri) await invoke("reveal_in_file_manager", { path });
}

function requireDesktop() {
  if (!runningInTauri) throw new Error("Bu özellik masaüstü uygulamasında kullanılabilir.");
}
export async function chooseMediaFile(filterName="Video / audio"): Promise<string | null> {
  requireDesktop();
  const path = await open({ multiple: false, filters: [{ name: filterName, extensions: ["mp4", "mkv", "webm", "mov", "avi", "m4v", "mp3", "wav", "m4a", "flac", "ogg", "opus", "aac"] }] });
  return typeof path === "string" ? path : null;
}
export async function prepareMedia(path: string): Promise<MediaInfo> {
  requireDesktop(); return invoke("prepare_media", { path });
}
export async function installModel(jobId: string, onEvent: (event: StudioEvent) => void, modelId="base"): Promise<void> {
  requireDesktop();
  const channel = new Channel<StudioEvent>(); channel.onmessage = onEvent;
  return invoke("install_model", { jobId, onEvent: channel, modelId });
}
export async function transcribeMedia(path: string, language: string, jobId: string, onEvent: (event: StudioEvent) => void, modelId="base", processingMode="auto"): Promise<Transcript> {
  requireDesktop();
  const channel = new Channel<StudioEvent>(); channel.onmessage = onEvent;
  return invoke("transcribe_media", { path, language, jobId, onEvent: channel, modelId, processingMode });
}
export async function listTranscripts(): Promise<Transcript[]> {
  if (!runningInTauri) return [];
  return invoke("list_transcripts");
}
export async function exportTranscript(transcript: Transcript, format: "txt" | "srt" | "vtt"): Promise<boolean> {
  requireDesktop();
  const base = transcript.mediaTitle.replace(/\.[^.]+$/, "");
  const path = await save({ defaultPath: `${base}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] });
  if (!path) return false;
  await invoke("export_transcript", { id: transcript.id, format, path }); return true;
}
export async function extractClip(path: string, output: string, start: number, end: number, jobId: string): Promise<void> {
  requireDesktop(); return invoke("extract_clip", { path, output, start, end, jobId });
}
export async function chooseClipOutput(title: string): Promise<string | null> {
  requireDesktop(); return save({ defaultPath: `${title.replace(/\.[^.]+$/, "")}-kesit.mp4`, filters: [{ name: "MP4", extensions: ["mp4"] }] });
}

export async function inspectDevice():Promise<DeviceProfile> { requireDesktop();return invoke("inspect_device"); }


export async function translationStatus(sourceLanguage:string,targetLanguage:string):Promise<import("../types").TranslationStatus>{requireDesktop();return invoke("translation_status",{sourceLanguage,targetLanguage});}
export async function saveTranslationSettings(email:string):Promise<void>{requireDesktop();return invoke("save_translation_settings",{email});}
export async function cachedTranslation(id:string,targetLanguage:string,translationMode="online"):Promise<import("../types").Translation|null>{requireDesktop();return invoke("cached_translation",{id,targetLanguage,translationMode});}
export async function installTranslationModel(sourceLanguage:string,targetLanguage:string,jobId:string,onEvent:(event:StudioEvent)=>void):Promise<void>{requireDesktop();const channel=new Channel<StudioEvent>();channel.onmessage=onEvent;return invoke("install_translation_model",{sourceLanguage,targetLanguage,jobId,onEvent:channel});}
export async function translateTranscript(id:string,targetLanguage:string,jobId:string,onEvent:(event:StudioEvent)=>void,translationMode="online"):Promise<import("../types").Translation>{requireDesktop();const channel=new Channel<StudioEvent>();channel.onmessage=onEvent;return invoke("translate_transcript",{id,targetLanguage,jobId,onEvent:channel,translationMode});}
export async function exportTranslation(transcript:Transcript,targetLanguage:string,format:"txt"|"srt"|"vtt",translationMode="online"):Promise<boolean>{requireDesktop();const base=transcript.mediaTitle.replace(/\.[^.]+$/,"");const path=await save({defaultPath:`${base}-${targetLanguage}.${format}`,filters:[{name:format.toUpperCase(),extensions:[format]}]});if(!path)return false;await invoke("export_translation",{id:transcript.id,targetLanguage,format,path,translationMode});return true;}
export async function openTranslationTerms():Promise<void>{requireDesktop();return invoke("open_translation_terms");}

export async function openProjectPage(page:"privacy"|"support"):Promise<void>{requireDesktop();return invoke("open_project_page",{page});}
