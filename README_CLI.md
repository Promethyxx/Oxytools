# oxy

Command-line interface for Oxytools — multimedia processing toolbox.

`oxytools-cli` provides access to all Oxytools modules without launching the GUI. Ideal for batch scripting, task schedulers, and system administration.

All bundled tools (ffmpeg, ffprobe, mkvpropedit) are included — no external dependencies required.

## Global options

| Option | Description | Default |
|--------|-------------|---------|
| `--lang <en\|fr>` | Language for localized operations (e.g. tag) | `en` |
| `--help` | Show help for any command or subcommand | |
| `--version` | Show version | |

```bash
oxytools-cli --help
oxytools-cli pic --help
oxytools-cli doc pdf-rotate --help
```

---

## pic — Image processing

### Convert

Convert image(s) to another format (png, jpg, webp, bmp, tiff, jxl…).

```bash
oxytools-cli pic convert photo.png --to webp
oxytools-cli pic convert photo.png --to jpg --quality 4
oxytools-cli pic convert *.png --to jxl
oxytools-cli pic convert *.png --to jxl --jxl-mode folder
oxytools-cli pic convert *.png --to jxl --jxl-mode pivot
```

| Option | Description | Default |
|--------|-------------|---------|
| `--to` | Target format (required) | |
| `--quality` | Quality ratio (1-10) | `2` |
| `--jxl-mode` | JXL mode: `lossless`, `folder`, `pivot` | `lossless` |

### Resize

```bash
oxytools-cli pic resize photo.jpg --width 1920 --height 1080
oxytools-cli pic resize photo.jpg --max-kb 500
oxytools-cli pic resize photo.jpg --width 1920 --height 1080 --max-kb 500
```

| Option | Description |
|--------|-------------|
| `--width` | Target width in pixels |
| `--height` | Target height in pixels |
| `--max-kb` | Maximum file size in KB |

### Rotate

```bash
oxytools-cli pic rotate photo.jpg --angle 90
oxytools-cli pic rotate *.jpg --angle 180
```

| Option | Description | Default |
|--------|-------------|---------|
| `--angle` | Rotation: 90, 180, 270 | `90` |

### Crop

Crop using percentages of the original dimensions.

```bash
oxytools-cli pic crop photo.jpg --x 10 --y 10 --width 80 --height 80
```

### Exif

Read or strip EXIF metadata.

```bash
oxytools-cli pic exif photo.jpg
oxytools-cli pic strip-exif photo.jpg
```

---

## doc — Document conversion & PDF tools

### Convert

Convert between document formats (md, html, txt, docx, odt → pdf, md, html, txt).

```bash
oxytools-cli doc convert rapport.md --to pdf
oxytools-cli doc convert page.html --to md
oxytools-cli doc convert *.txt --to pdf
```

### PDF Split

Split a PDF into individual pages.

```bash
oxytools-cli doc pdf-split document.pdf
```

Output: a `document_pages/` folder with one PDF per page.

### PDF Merge

Merge multiple PDFs into one.

```bash
oxytools-cli doc pdf-merge a.pdf b.pdf c.pdf --output merged.pdf
```

### PDF Rotate

```bash
oxytools-cli doc pdf-rotate document.pdf --angle 90
oxytools-cli doc pdf-rotate document.pdf --angle 180 --pages 1,3,5
```

| Option | Description | Default |
|--------|-------------|---------|
| `--angle` | Rotation: 90, 180, 270 | `90` |
| `--pages` | Specific pages (comma-separated). Omit for all. | all |

### PDF Compress

```bash
oxytools-cli doc pdf-compress document.pdf
```

### PDF Crop

Crop pages using percentages.

```bash
oxytools-cli doc pdf-crop document.pdf --x 10 --y 10 --width 80 --height 80
oxytools-cli doc pdf-crop document.pdf --x 5 --y 5 --width 90 --height 90 --pages 1,2
```

### PDF Organize

Reorder pages.

```bash
oxytools-cli doc pdf-organize document.pdf --order 3,1,2,4
```

### PDF Delete

Remove specific pages.

```bash
oxytools-cli doc pdf-delete document.pdf --pages 2,5,8
```

### PDF Number

Add page numbers.

```bash
oxytools-cli doc pdf-number document.pdf
oxytools-cli doc pdf-number document.pdf --start 1 --position BasDroite --size 12
```

| Option | Description | Default |
|--------|-------------|---------|
| `--start` | Starting number | `1` |
| `--position` | `BasCentre`, `BasGauche`, `BasDroite`, `HautCentre`, `HautGauche`, `HautDroite` | `BasCentre` |
| `--size` | Font size | `10` |

### PDF Protect

```bash
oxytools-cli doc pdf-protect document.pdf --owner-pass secret --user-pass read123
oxytools-cli doc pdf-protect document.pdf --owner-pass secret --user-pass read123 --allow-print false --allow-copy false
```

### PDF Unlock

```bash
oxytools-cli doc pdf-unlock document.pdf --password secret
```

### PDF Repair

```bash
oxytools-cli doc pdf-repair document.pdf
```

### PDF Watermark

```bash
oxytools-cli doc pdf-watermark document.pdf --text "CONFIDENTIEL"
oxytools-cli doc pdf-watermark document.pdf --text "DRAFT" --size 80 --opacity 0.3
oxytools-cli doc pdf-watermark document.pdf --text "DRAFT" --pages 1,2,3
```

| Option | Description | Default |
|--------|-------------|---------|
| `--text` | Watermark text (required) | |
| `--size` | Font size | `60` |
| `--opacity` | Opacity (0.0 - 1.0) | `0.15` |
| `--pages` | Specific pages. Omit for all. | all |

---

## tag — MKV tagging

### Mark as watched

Cumulative: increments PLAYCOUNT each time.

```bash
oxytools-cli tag marquer-vu film.mkv --lang fr
oxytools-cli tag marquer-vu *.mkv --lang en
```

### Edit a single tag

```bash
oxytools-cli tag edit film.mkv --tag TITLE --value "My Movie"
oxytools-cli tag edit film.mkv --tag COMMENT --value "8.5 / 10"
```

### Inject NFO metadata

```bash
oxytools-cli tag nfo film.mkv --nfo film.nfo
```

### Attach images

Looks for poster/fanart/clearlogo images next to the MKV file.

```bash
oxytools-cli tag images film.mkv
oxytools-cli tag images *.mkv
```

### Reset (remove all tags and attachments)

```bash
oxytools-cli tag reset film.mkv
oxytools-cli tag reset *.mkv
```

---

## rename — Batch rename

### Find and replace

```bash
oxytools-cli rename *.mp4 --find "S01" --replace "Saison 1"
oxytools-cli rename *.mp4 --find "S(\d+)" --replace "Season $1" --regex
```

### Preview without renaming

```bash
oxytools-cli rename *.mp4 --find "test" --replace "ok" --dry-run
```

### Case transform

```bash
oxytools-cli rename *.txt --case lower
oxytools-cli rename *.txt --case upper
oxytools-cli rename *.txt --case title
oxytools-cli rename *.txt --case sentence
```

### Numbering

```bash
oxytools-cli rename *.jpg --number prefix --num-start 1 --num-pad 3 --num-sep " - "
oxytools-cli rename *.jpg --number suffix --num-step 10
```

| Option | Description | Default |
|--------|-------------|---------|
| `--number` | `prefix` or `suffix` | |
| `--num-start` | Starting number | `1` |
| `--num-step` | Increment | `1` |
| `--num-pad` | Zero-padding width | `2` |
| `--num-sep` | Separator | `" "` |

### Extension

```bash
oxytools-cli rename *.TXT --ext lower
oxytools-cli rename *.txt --ext upper
oxytools-cli rename *.jpeg --ext replace --ext-new jpg
oxytools-cli rename *.bak --ext remove
```

### Multiple replacements from file

Load a TSV replace list (tab-separated: find → replace).

```bash
oxytools-cli rename *.mp4 --list rules.tsv
```

TSV file format:
```
#find	replace	regex	enabled
S01	Saison 1	false	true
S02	Saison 2	false	true
```

### Import Ant Renamer rules

```bash
oxytools-cli rename *.mp4 --ant renamer.xml
oxytools-cli rename *.mp4 --ant renamer.xml --ant-set "My Set"
```

### Combine options

All options can be combined in a single command:

```bash
oxytools-cli rename *.mp4 --find "old" --replace "new" --case title --number prefix --num-pad 3 --ext lower --dry-run
```

---

## archive — Compression & extraction

### Compress

```bash
oxytools-cli archive compress "C:\my\folder" --to zip
oxytools-cli archive compress "C:\my\folder" --to 7z --level 9
oxytools-cli archive compress file.txt --to gz
```

| Option | Description | Default |
|--------|-------------|---------|
| `--to` | Format: `zip`, `7z`, `tar`, `gz` (required) | |
| `--level` | Compression level (1-9) | `6` |

### Extract

```bash
oxytools-cli archive extract archive.zip
oxytools-cli archive extract archive.7z --dest "C:\destination"
```

### Convert

Convert an archive to another format.

```bash
oxytools-cli archive convert archive.zip --to 7z
```

### Backup

Create a zip backup with exclusions.

```bash
oxytools-cli archive backup "C:\project" --dest "C:\backups"
oxytools-cli archive backup "C:\project" --dest "C:\backups" --exclude ".git,target,node_modules"
```

| Option | Description | Default |
|--------|-------------|---------|
| `--dest` | Destination folder (required) | |
| `--exclude` | Comma-separated exclusions | `.git,.github,target` |

---

## tools — File & folder listing

### List files

Generate a .txt file per source with all files listed recursively.

```bash
oxytools-cli tools list-files --output "C:\lists" --source "movies=D:\Films" --source "series=D:\Series"
```

This creates `C:\lists\movies.txt` and `C:\lists\series.txt`.

### List folders

Generate `multimedia.txt` with all subfolders listed recursively.

```bash
oxytools-cli tools list-folders --output "C:\lists" --source "D:\Films" --source "D:\Series"
```

---

## Task Scheduler (Windows)

To schedule a task, create an entry in Windows Task Scheduler:

| Field | Value |
|-------|-------|
| Program | `C:\Oxytools\oxytools-cli.exe` |
| Arguments | `tag marquer-vu "D:\Films\*.mkv" --lang fr` |
| Start in | `C:\Oxytools` |

Example scheduled tasks:

```
# Nightly: mark all MKVs as watched
oxytools-cli tag marquer-vu "D:\Films\*.mkv" --lang fr

# Weekly: backup project
oxytools-cli archive backup "C:\Dev\MyProject" --dest "E:\Backups" --exclude ".git,target"

# Daily: regenerate file listings
oxytools-cli tools list-files --output "C:\Lists" --source "films=D:\Films" --source "series=D:\Series"
```

---

## Output naming

By default, output files are created next to the source with `_oxytools` suffix:

```
photo.png → photo_oxytools.webp
document.md → document_oxytools.pdf
archive.zip → archive_oxytools.7z
```

Exceptions:
- `archive extract` → creates a folder named after the archive
- `doc pdf-split` → creates a `_pages/` folder
- `doc pdf-merge` → uses the `--output` path
- `pic convert --to jxl` → follows JXL mode conventions (lossless/folder/pivot)
- `rename` → renames files in place
---
# Contributing
You can find the contributing rules at :
https://github.com/Promethyxx/Roadmap/blob/main/contributing.md

# Manifest
You can find my manifest at :
https://github.com/Promethyxx/Roadmap/blob/main/manifest.md

# Roadmap
You can find the roadmap at :
https://github.com/Promethyxx/Roadmap

