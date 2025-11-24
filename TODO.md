# TODO & Feature Roadmap

## Remaining Tasks (Current Architecture)
- [ ] **Agent Task Sync**: Ensure agent tasks are fully isolated or correctly migrated when switching workspaces (currently pending reviews are cleared, but active tasks might need better handling).
- [ ] **Error Handling**: Improve error reporting for network failures (LLM API down, Collaboration server unreachable).
- [ ] **Persistence**: Verify that all agent configurations (including system prompts) are persisted correctly across restarts.

## Proposed Features (Enhancements)

### Core "Multiplexer" Features
- [ ] **Git Integration**:
    - View git status.
    - Stage/Unstage changes.
    - Commit and Push directly from the UI.
    - Branch switching/management.
- [ ] **File Explorer**:
    - Sidebar to browse the project directory.
    - Open/Edit multiple files (currently restricted to single-buffer `live_code`).
- [ ] **Terminal**:
    - Integrated terminal emulator to run shell commands (cargo, git, etc.) without leaving the app.

### Editor Improvements
- [ ] **Syntax Highlighting**:
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
