import type { TranscriptSegment } from "../types";

export function timeLabel(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  return `${hours ? `${hours.toString().padStart(2, "0")}:` : ""}${Math.floor(total / 60 % 60).toString().padStart(2, "0")}:${(total % 60).toString().padStart(2, "0")}`;
}
export function foldSearchText(text: string): string {
  return text.replace(/[ıİ]/g, "i").toLowerCase();
}

export function searchSegments(segments: TranscriptSegment[], query: string) {
  const needle = foldSearchText(query.trim());
  return segments.map((segment, index) => ({ segment, index })).filter(({ segment }) => !needle || foldSearchText(segment.text).includes(needle));
}
