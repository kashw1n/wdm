# WDM - Web Download Manager

## Goal
Build a fast download manager like IDM that uses multiple parallel connections to maximize download speed.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend (React)                      │
│  - Download Queue UI                                     │
│  - Progress bars (per-chunk + overall)                   │
│  - Speed/ETA display                                     │
│  - Pause/Resume/Cancel controls                          │
└─────────────────────┬───────────────────────────────────┘
                      │ Tauri IPC (events + commands)
┌─────────────────────▼───────────────────────────────────┐
│                  Rust Backend                            │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ Download     │  │ Chunk        │  │ File         │   │
│  │ Manager      │  │ Scheduler    │  │ Assembler    │   │
│  │ (queue,state)│  │ (parallel)   │  │ (merge parts)│   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │ HTTP Client  │  │ Progress     │  │ Persistence  │   │
│  │ (reqwest     │  │ Tracker      │  │ (save state  │   │
│  │  +pool)      │  │ (per-chunk)  │  │  for resume) │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Core Download Engine ✅
- [x] URL metadata fetching (filename, size, resume support)
- [x] Chunked download with 8 parallel connections
- [x] Progress events sent to frontend (100ms interval)
- [x] Download UI with overall + per-chunk progress bars
- [x] Speed display
- [x] Single-connection fallback for non-resumable files
- [x] Follow redirects and extract filename from final URL

### Phase 2: Download Controls ✅
- [x] Pause/Resume functionality
- [x] Cancel download
- [x] Download queue management (multiple concurrent downloads)
- [x] Configurable connection count (1-32)
- [x] Settings UI panel
- [x] Duplicate filename detection with rename prompt

### Phase 3: Persistence & Resume
- [ ] Save download state to disk
- [ ] Resume interrupted downloads after app restart
- [ ] Download history

### Phase 4: Polish & Features
- [ ] Settings UI (download folder, connections, etc.)
- [ ] Speed limiting
- [ ] System tray integration
- [ ] Browser extension for catching downloads (optional)

## Key Technical Decisions

1. **Chunking Strategy**: Split file into N equal parts based on connection count
2. **HTTP Range Requests**: Use `Range: bytes=start-end` headers
3. **Progress Updates**: Tauri events emitted per-chunk, aggregated on frontend
4. **File Assembly**: Write chunks to temp files, merge on completion
5. **Connection Pool**: Reuse HTTP connections via reqwest client
6. **State Management**: Atomic flags for pause/cancel, RwLock for download registry

## Current Status
- **Completed**: Phase 1 + Phase 2
- **Next Step**: Phase 3 - Persistence & Resume
