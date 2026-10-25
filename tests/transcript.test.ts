import { test } from "node:test";
import assert from "node:assert/strict";
import { searchSegments, timeLabel } from "../src/lib/transcript.ts";

test("Turkish search matches dotted I without losing original timestamps", () => {
  const segments = [{ start: 2.5, end: 5, text: "İLERİ seviyede görüntü işleme" }, { start: 6, end: 8, text: "Işık ve renk" }];
  assert.equal(searchSegments(segments, " ileri ")[0]?.segment.start, 2.5);
  assert.equal(searchSegments(segments, "ışık")[0]?.index, 1);
  assert.equal(searchSegments(segments, "[").length, 0);
  assert.equal(searchSegments(segments, " ").length, 2);
  assert.equal(searchSegments([], "test").length, 0);
  assert.equal(searchSegments([{ start: 0, end: 1, text: "AI and Image Processing" }], "image").length, 1);
});

test("video timestamps are readable before and after an hour", () => {
  assert.equal(timeLabel(0), "00:00");
  assert.equal(timeLabel(65.9), "01:05");
  assert.equal(timeLabel(3661), "01:01:01");
});
