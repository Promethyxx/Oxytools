# Oxytools

Collection of utility scripts

## Oxy_FL

- 2 PowerShell scripts
- `oxyflf` lists folders (no subfolder) into a text file
- `oxyfl` lists files (subfolder included) into multiple text files (one per target)
- No requirements
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