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

### Architecture Goals
- Dual endpoint support (old + new)
- Feature parity between endpoints
- Seamless switching via configuration
- Reuse existing proven components

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
- [ ] **P1-T4**: Add New Endpoint to Client
- [ ] **P1-T5**: Update Proxy Server for New Endpoint
- [ ] **P1-T6**: Add Configuration and Feature Flags
- [ ] **P1-T7**: Testing and Validation

### Phase 2: Tool Calling Implementation  
- [ ] **P2-T1**: Analyze Current Tool Implementation
- [ ] **P2-T2**: Design Tool Embedding Strategy
- [ ] **P2-T3**: Implement Tool Definition Injection
- [ ] **P2-T4**: Adapt Tool Response Parsing
- [ ] **P2-T5**: Update OpenAI Compatibility Layer
- [ ] **P2-T6**: Testing and Validation

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
- Maintain separate test suites for each endpoint

### Risk Mitigation
- Comprehensive testing at each phase
- Ability to rollback to previous endpoint
- Gradual feature enablement
- Monitoring and logging throughout

## Success Criteria

### Phase 1 Complete
- [x] New chat endpoint responds to basic requests
- [x] Content format conversion works for both string and array
- [x] OpenAI compatibility maintained
- [x] Configuration allows switching between endpoints

### Phase 2 Complete  
- [x] Tool definitions embedded in user messages
- [x] Tool calls parsed from responses correctly
- [x] Full OpenAI tool calling API compatibility
- [x] Error handling for malformed tool responses

### Phase 3 Complete
- [x] Streaming works on new endpoint
- [x] Heartbeat chunks and proper SSE formatting
- [x] Graceful fallback when streaming unavailable
- [x] Complete feature parity with old endpoint

## Timeline

| Phase | Duration | Dependencies | Deliverable |
|-------|----------|--------------|-------------|
| Phase 1 | 1-2 days | None | Basic chat endpoint |
| Phase 2 | 2-3 days | Phase 1 complete | Tool calling support |
| Phase 3 | 1-2 days | Straico streaming support | Full feature parity |

**Total Estimated Duration**: 4-7 days

## Next Steps

1. Begin with **P1-T1**: Preserve Current Implementation
2. Work through Phase 1 tasks sequentially
3. Test thoroughly before proceeding to Phase 2
4. Monitor Straico API updates for streaming availability

---

*Last Updated*: [Current Date]  
*Status*: Planning Phase  
*Current Phase*: Pre-Phase 1