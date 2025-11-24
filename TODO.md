# TODO & Feature Roadmap

## Remaining Tasks (Current Architecture)
- [x] **Agent Task Sync**: Ensure agent tasks are fully isolated or correctly migrated when switching workspaces (currently pending reviews are cleared, but active tasks might need better handling).
- [ ] **Error Handling**: Improve error reporting for network failures (LLM API down, Collaboration server unreachable).
- [x] **Persistence**: Verify that all agent configurations (including system prompts) are persisted correctly across restarts.

## Proposed Features (Enhancements)

### Core "Multiplexer" Features
- [ ] **Git Integration**:
    - View git status.
    - Stage/Unstage changes.
    - Commit and Push directly from the UI.
    - Branch switching/management.
- [x] **File Explorer**:
    - Sidebar to browse the project directory.
    - Open/Edit multiple files (currently restricted to single-buffer `live_code`).
- [ ] **Terminal**:
    - Integrated terminal emulator to run shell commands (cargo, git, etc.) without leaving the app.

### Editor Improvements
- [x] **Syntax Highlighting**:
    - Integrate a syntax highlighter (e.g., `syntect` or improved custom lexer) for better code readability.
    - Theme support for syntax colors.
- [ ] **Multi-File Support**:
    - Refactor `Workspace` to support multiple open files/buffers instead of a single string.
    - Tabbed editing interface.

### AI Agent Enhancements
- [ ] **Streaming Responses**:
    - Replace `ehttp` (one-shot) with a streaming HTTP client (e.g., `reqwest` with WASM support or native threads) to show agent typing in real-time.
- [ ] **Project-Wide Context (RAG)**:
    - Index the entire project codebase (not just help docs) for agent context.
    - Allow agents to "read" other files in the project.
- [ ] **Tool Use**:
    - Allow agents to execute shell commands or file operations (with user permission), moving beyond just writing code.

### UI/UX
- [ ] **Markdown Rendering**:
    - Render chat messages and plans with Markdown (bold, lists, code blocks) instead of raw text.
- [ ] **Theme Editor**:
    - Enhance the theme editor with a preview and preset saving.

## Last Will and Testament (Notes for Next Agent)

### Architecture Overview
- **Workspaces**: The core "multiplexing" feature. Defined in `src/workspace.rs`. Workspaces isolate `live_code` and agent `tasks`.
    - *Important*: Agent events (`AgentEvent`) must be tagged with the `workspace_name` to prevent cross-talk. This is handled in `src/agent.rs` (`poll` method).
    - *State*: Persisted via `eframe` storage in `src/app.rs`.
- **Agents**:
    - **Chat**: Separate from task execution. Used for Q&A.
    - **Planning**: Uses LLM to break down goals into tasks.
    - **Code Review**: Agent code is *never* applied directly. It goes to `pending_reviews` (in `TemplateApp`) and is displayed in the "Code Review" window with a diff.
- **UI**: Immediate mode via `egui`.
    - `src/palette.rs`: Command Palette implementation.
    - `src/file_explorer.rs`: Native-only file tree.

### Critical Areas / Gotchas
1.  **Borrow Checker in UI**: `egui` closures often capture `self`. Be careful when modifying `self.agent_system` inside a UI closure that reads from `self`. Pattern: Collect data first, mutate after.
2.  **Async HTTP**: We use `ehttp` (one-shot). It doesn't support streaming. Moving to streaming (for "typing effect") will require a significant refactor (likely `reqwest` or `ewebsock` bridge).
3.  **WASM Compatibility**: The `FileExplorer` is gated behind `#[cfg(not(target_arch = "wasm32"))]`. Ensure future filesystem features respect this.

### Next Priority
- **Multi-File Editing**: The current `Workspace` only holds a single `code` string. This is the biggest limitation. It needs to support a map of filenames to content or tabs.
- **Git Integration**: Essential for a real dev tool.
