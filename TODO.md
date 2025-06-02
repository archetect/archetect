# Configuration Enhancement: ~/.archetect/etc.d/*.yaml Support

## Overview
Add support for loading multiple configuration files from `~/.archetect/etc.d/*.yaml` directory, processing them in sorted order after the main `~/.archetect/etc/archetect.yaml` configuration is loaded.

## Current Configuration Loading Order
1. Default configuration (in-memory)
2. User global config: `~/.archetect/etc/archetect.yaml`
3. Current working directory config
4. CLI-passed configuration

## Proposed Enhancement
Insert step 2.5: Load and merge all `~/.archetect/etc.d/*.yaml` files in sorted order after the main user global config.

## Implementation Plan

### Phase 1: Research & Testing Current Behavior
- [ ] Analyze current configuration loading implementation
- [ ] Document how different sections merge (actions replace, others merge)
- [ ] Create comprehensive tests for existing configuration behavior
- [ ] Verify action replacement vs. section merging behavior

### Phase 2: Implementation
- [ ] Implement `~/.archetect/etc.d/*.yaml` directory scanning
- [ ] Add sorted loading of configuration files
- [ ] Ensure proper merging with same rules as existing system
- [ ] Maintain backwards compatibility

### Phase 3: Testing & Validation
- [ ] Add tests for new configuration.d directory functionality
- [ ] Test edge cases (no directory, empty directory, invalid files)
- [ ] Verify no breaking changes to existing behavior
- [ ] Test configuration precedence and merging

## Key Requirements
- **No Breaking Changes**: Existing configuration behavior must remain identical
- **Sorted Processing**: Files in etc.d/ processed in alphabetical order
- **Same Merge Rules**: Actions replace, other sections merge (verify current behavior)
- **Error Handling**: Graceful handling of missing directory or invalid files

## Testing Strategy
1. Create tests for current behavior first
2. Implement new functionality
3. Ensure all existing tests still pass
4. Add new tests for etc.d/ functionality