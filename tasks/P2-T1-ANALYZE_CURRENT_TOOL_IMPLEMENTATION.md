# P2-T1: Analyze Current Tool Implementation

## Objective
Thoroughly analyze the existing tool implementation to understand how to adapt it for the new chat endpoint with XML embedding in user messages.

## Background
The current tool implementation works with the prompt endpoint by embedding tool definitions in system messages and parsing XML tool calls from responses. For the new chat endpoint, we need to adapt this to work with the structured message format.

## Tasks

### 1. Document Current Tool Flow
**File**: `CURRENT_TOOL_FLOW_ANALYSIS.md`

**Analysis Points**:
- How tools are defined in OpenAI requests
- How tools are embedded in Straico prompts
- How tool calls are parsed from responses
- How tool responses are handled
- Error handling for malformed tool calls

**Key Code Paths to Trace**:
```
OpenAI Request with tools
    ↓
proxy/src/server.rs (OpenAiRequest.tools)
    ↓
client/src/chat.rs (Chat.to_prompt with tools)
    ↓
Straico API call
    ↓
Response parsing (tool_calls_response)
    ↓
OpenAI-compatible response
```

### 2. Analyze Tool Embedding Logic
**File**: `client/src/chat.rs` (lines 304-338)

**Current Implementation Analysis**:
- Tool definition serialization
- XML structure generation
- Model-specific formatting
- System message injection

**Key Questions**:
- Can this logic work with user messages instead of system messages?
- How to handle multiple user messages with tools?
- What changes are needed for the new format?

### 3. Analyze Tool Response Parsing
**File**: `client/src/endpoints/completion/completion_response.rs` (tool_calls_response method)

**Current Parsing Logic**:
- XML pattern matching
- JSON extraction from tool calls
- Error handling for malformed responses
- Model-specific format handling

**Adaptation Requirements**:
- Will this work with new response format?
- Any changes needed for parsing logic?
- Error handling updates required?

### 4. Identify Reusable Components
**File**: `REUSABLE_TOOL_COMPONENTS.md`

**Components to Preserve**:
- Tool definition structures (`Tool` enum)
- XML formatting utilities
- Parsing regex patterns
- Error handling patterns
- Model-specific format constants

**Components to Adapt**:
- Embedding location (system → user message)
- Message structure handling
- Response format parsing

### 5. Design Adaptation Strategy
**File**: `TOOL_ADAPTATION_STRATEGY.md`

**Strategy Options**:
1. **First User Message**: Embed tools in first user message
2. **Separate Tool Message**: Add dedicated tool definition message
3. **Per-Message Tools**: Embed tools with each relevant message

**Recommended Approach**:
- Analysis of pros/cons for each option
- Implementation complexity assessment
- Compatibility considerations

## Deliverables

1. **Analysis Documents**:
   - `CURRENT_TOOL_FLOW_ANALYSIS.md`
   - `REUSABLE_TOOL_COMPONENTS.md`
   - `TOOL_ADAPTATION_STRATEGY.md`

2. **Code Analysis**:
   - Detailed understanding of current implementation
   - List of required changes
   - Risk assessment for adaptation

3. **Strategy Decision**:
   - Chosen approach for tool embedding
   - Implementation plan outline
   - Compatibility impact assessment

## Success Criteria

- [ ] Current tool flow completely understood
- [ ] All reusable components identified
- [ ] Adaptation strategy chosen and documented
- [ ] Implementation approach defined
- [ ] Risk factors identified and mitigated
- [ ] Compatibility requirements understood

## Time Estimate
**Duration**: 2-3 hours

## Dependencies
- **P1-T7**: Testing and Validation (Phase 1 complete)

## Next Task
**P2-T2**: Design Tool Embedding Strategy

## Notes
- Focus on understanding the XML generation and parsing logic
- Pay attention to model-specific differences
- Consider how the new message structure affects tool handling
- Document any limitations or edge cases discovered