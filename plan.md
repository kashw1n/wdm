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

### Phase 2: Download Controls
- [ ] Pause/Resume functionality
- [ ] Cancel download
- [ ] Download queue management
- [ ] Configurable connection count (4-16)

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

## Current Status
- **Completed**: Phase 1 - Core download engine with 8 parallel connections
- **Next Step**: Phase 2 - Pause/Resume/Cancel controls
