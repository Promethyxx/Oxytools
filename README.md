# Oxytools

Collection of utility scripts before maybe impplmentation into Oxyon

## Oxy_FL

- 2 PowerShell scripts
- `oxyflf` lists folders (no subfolder) into a text file
- `oxyfl` lists files (subfolder included) into multiple text files (one per target)
- No requirements
- Update the paths, then run

## Oxy_JXL

- 3 PowerShell scripts
- `oxyj` converts in lossless mode
- `oxyjf` forces the conversion
- `oxyjp` converts into PNG before JXL
- Requirements: [CJXL](https://github.com/niclasr/cjxl)
  - Paths: source folder `$SRC` and CJXL binary `$CJXL`
- Update the paths, then run

## Oxy_Wall

- Python script to fetch 1080p and 4K wallpapers
- Requirements: Python, `python-dotenv`, `requests`
- Create a `.env` file next to the script with your API keys:
  - `PEXELS_API_KEY`
  - `PIXABAY_API_KEY`
  - `UNSPLASH_API_KEY`
- Update the paths, then run

## Oxy_Watch

> 🚧 Rust module — under construction

- Replicates the Jellyfin workflow to track watched movies
  1. Checks VLC playback status (>90% = watched)
  2. Writes the result to a text file for later tagging
- Requirements: VLC

## Oxy_Zip

- 2 PowerShell scripts
- `oxyz` compresses a folder into a ZIP
- `oxyzm` compresses multiple folders into multiple ZIPs
- Requirements: [7-Zip Extra](https://www.7-zip.org/)
  - Update the paths
- Update the paths, then run