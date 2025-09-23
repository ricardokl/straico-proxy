# New Chat Endpoint Migration - Master Plan

## Overview

This plan outlines the migration from the current Straico prompt endpoint (`/v1/prompt/completion`) to the new chat endpoint (`/v0/chat/completions`) in three carefully orchestrated phases. The migration preserves all existing functionality while adding support for the new endpoint format.

## Migration Strategy

### Core Principles
- **Preserve Existing Functionality**: Keep current endpoint working throughout migration
- **Incremental Implementation**: Add new features step by step
- **Backward Compatibility**: Maintain OpenAI API compatibility
- **Dual Content Support**: Handle both string and array content formats
- **Custom Tool Implementation**: Use XML embedding for tool calls until native support

## Phase Overview

### Phase 1: Basic Chat Endpoint (No Tools, No Streaming)
**Goal**: Implement new chat endpoint with dual content format support
**Duration**: Estimated 1-2 days
**Deliverables**: Working new endpoint for basic chat completion

### Phase 2: Tool Calling Implementation
**Goal**: Add custom tool support via XML embedding in user messages
**Duration**: Estimated 2-3 days  
**Deliverables**: Full tool calling functionality on new endpoint

### Phase 3: Streaming Support
**Goal**: Implement streaming for new endpoint (when Straico supports it)
**Duration**: Estimated 1-2 days
**Deliverables**: Complete feature parity with old endpoint

## Task Tracking

### Phase 1: Basic Chat Endpoint
- [x] **P1-T1**: Preserve Current Implementation
- [x] **P1-T2**: Create New Chat Request/Response Structures
- [x] **P1-T3**: Implement Content Format Conversion
- [x] **P1-T4**: Add New Endpoint to Client
- [x] **P1-T5**: Update Proxy Server for New Endpoint
- [x] **P1-T6**: Add Configuration and Feature Flags
- [x] **P1-T7**: Testing and Validation

### Phase 2: Tool Calling Implementation  
- [x] **P2-T1**: Analyze Current Tool Implementation
- [x] **P2-T2**: Design Tool Embedding Strategy
- [x] **P2-T3**: Implement Tool Definition Injection
- [x] **P2-T4**: Adapt Tool Response Parsing
- [x] **P2-T5**: Update OpenAI Compatibility Layer
- [x] **P2-T6**: Testing and Validation

### Phase 3: Streaming Support
- [ ] **P3-T1**: Analyze Current Streaming Implementation
- [ ] **P3-T2**: Design Streaming Architecture for New Endpoint
- [ ] **P3-T3**: Implement Streaming Request Handling
- [ ] **P3-T4**: Adapt Streaming Response Processing
- [ ] **P3-T5**: Update Error Handling and Fallbacks
- [ ] **P3-T6**: Testing and Validation

## Implementation Notes

### Key Files to Modify
- `client/src/client.rs` - Add new endpoint support
- `client/src/endpoints/` - New chat structures
- `proxy/src/server.rs` - New endpoint handler
- `proxy/src/main.rs` - Configuration flags
- Root `Cargo.toml` - Workspace configuration

### Preservation Strategy
- Keep all existing code intact during Phase 1
- Create new modules alongside existing ones
- Use feature flags to switch between implementations
