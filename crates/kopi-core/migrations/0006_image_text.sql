-- Text in images: the text read from each image clip goes in its plain_text, so the search
-- index covers it. See docs/OCR_DESIGN.md.

-- NULL: this image's text hasn't been read. A number: read by that version of the reader,
-- with the text, if any, in plain_text.
ALTER TABLE clips ADD COLUMN ocr_version INTEGER;

-- The images left to read, newest first.
CREATE INDEX clips_ocr_pending ON clips (last_used_at DESC)
  WHERE kind = 'image' AND ocr_version IS NULL;
