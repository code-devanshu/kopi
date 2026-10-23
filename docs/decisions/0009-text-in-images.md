# 0009. Text in images is read on each computer, into the clip's search text

- **Status:** Accepted
- **Date:** 2026-10-23

## Context

"OCR (searching text inside images)" is on the Later list in [MVP_SCOPE.md](../MVP_SCOPE.md), and [DATA_MODEL.md](../DATA_MODEL.md) has kept an image clip's `plain_text` empty for it. Screenshots are a large part of what people copy, and search can't find any of them. The full design is in [OCR_DESIGN.md](../OCR_DESIGN.md).

Four choices shape it: which engine reads the text, where the text is stored, what paste does with it, and whether sync sends it.

## Decision

- **The OS reads the text:** Vision's `VNRecognizeTextRequest` on macOS and `Windows.Media.Ocr` on Windows, behind a `TextRecognizer` trait in `kopi-platform`. Both run on the computer, with no download and no network.
- **The core decodes and scales the image** with the `image` crate, as it does for thumbnails, and hands the recognizer plain RGBA pixels. One thread, `kopi-ocr`, reads one image at a time at a low priority: new copies first, then the older history, newest first.
- **The text goes in the image's `plain_text`**, which the search index already covers. A new `ocr_version` column records that an image was read, and by which version of the reader. No search index is rebuilt.
- **An image's text is never written to the clipboard** as part of the clip. Return and ⌥Return paste the image as it was copied. A separate Copy text action writes only the text.
- **The text isn't synced.** Each computer reads its own images, received ones included. The protocol is unchanged.
- **On by default**, as **Find text in images** in Settings → Capture. Turning it off removes the text it read.

## Alternatives considered

| Option | Why not |
| --- | --- |
| Tesseract or another bundled engine | Tens of megabytes of language data in a small app, and slower and less accurate than the OS engines on screenshots. Both OSes already ship one. |
| A cloud OCR service | Sends the user's screenshots to someone else's server, against Kopi's first rule. |
| A separate `ocr_text` column in the search index | Changing `clips_fts` means rebuilding the trigram index for every clip during the update, and histories reach 100,000 clips. `plain_text` is already "the text search uses", and images had none. |
| Recording that an image was read in `metadata` | Copying the image again replaces its metadata, so it would be read again on every copy. |
| Decoding with each OS (ImageIO, `BitmapDecoder`) | Two decoders to keep in step, and the core already decodes PNG, TIFF and DIB for thumbnails, with limits. |
| Sending the text with the image in sync | Ignores the receiving computer's setting and languages, means trusting text nothing here checked, and needs a protocol field that older versions drop. Reading one image costs about as much as sending it. |
| Off by default | It needs no permission and sends nothing anywhere, and a switch nobody finds helps nobody. Turning it off is one switch and removes the text. |

## Consequences

- **Good:** screenshots become searchable with no new screen, no permission and no network.
- **Good:** search, ranking, highlighting, filters, limits and deletes all work for the text without changes, because it's ordinary clip text.
- **Good:** sync and older versions are unaffected.
- **Bad:** `plain_text` now means two things for images and the rest, so paste needs a rule that the image's text stays off the clipboard. A missed spot would paste text where an image was expected.
- **Bad:** each computer spends time reading images the other already read, once per image.
- **Bad:** reading the older history takes a while after the update, and uses some battery while it runs.
- **Bad:** the text depends on the OS engine. Results differ between macOS and Windows, and between versions of each.
