# Plug - Port/Process Manager Implementation Plan

## Project Overview

**Goal:** Build a hybrid port/process manager CLI tool that allows users to:
- View active network sockets (TCP/UDP) by port or by process
- Filter and search through network connections
- Kill processes using specific ports or PIDs
- Interactive TUI using the `inquire` crate

**Platform:** Local machine only (Linux primary target, with cross-platform support via netstat2)

---

## Architecture Overview

### Core Data Model

```rust
// Simplified protocol enum (converts from netstat2's ProtocolSocketInfo)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProtocolInfo {
    TCP,
    UDP,
}

// Represents a single network socket entry
#[derive(Debug, Clone)]
struct SocketEntry {
    // Network information
    local_port: u16,
    local_addr: IpAddr,
    remote_addr: Option<IpAddr>,   // None for UDP listening sockets
    remote_port: Option<u16>,      // None for UDP
    protocol: ProtocolInfo,
    
    // TCP-specific
    tcp_state: Option<TcpState>,   // None for UDP
    
    // Process information
    pid: u32,
    process_name: String,
    
    // Linux-specific fields
    #[cfg(target_os = "linux")]
    uid: u32,
    #[cfg(target_os = "linux")]
    inode: u32,
}

// Main data structure with indexed lookups
struct NetworkManager {
    sockets: Vec<SocketEntry>,
    
    // Indexes for O(1) lookup
    by_port: HashMap<u16, Vec<usize>>,      // port -> socket indices
    by_process: HashMap<u32, Vec<usize>>,   // pid -> socket indices
}
```

### CLI Interface Design

```bash
# Default: interactive mode with view selection
plug

# Port-centric view
plug --by-port
plug -p

# Process-centric view  
plug --by-process
plug -P

# Direct actions (bypass interactive mode)
plug --kill-port 8080         # Kill all processes using port 8080
plug --kill-pid 1234          # Kill process with PID 1234

# Filtering
plug --protocol tcp           # Show only TCP sockets
plug --protocol udp           # Show only UDP sockets
plug --state LISTEN           # Show only listening sockets
plug --state ESTABLISHED      # Show only active connections

# Combinations
plug --by-port --protocol tcp --state LISTEN
```

### User Flow - Interactive Mode

```
┌─────────────────────────────────────────┐
│  Welcome to Plug - Port/Process Manager │
└─────────────────────────────────────────┘

? Select view mode:
  > By Port (group sockets by port number)
    By Process (group sockets by process)
    All Sockets (flat list)
    Exit

[User selects "By Port"]

? Select port to inspect:
  > Port 8080 (TCP) - 3 sockets - nginx (PID 1234)
    Port 443 (TCP) - 5 sockets - nginx (PID 1234)
    Port 5432 (TCP) - 1 socket - postgres (PID 5678)
    Port 53 (UDP) - 1 socket - systemd-resolved (PID 890)
    [Filter options]
    [Back]

[User selects Port 8080]

╔═══════════════════════════════════════════════════════╗
║ Port 8080 Details                                      ║
╠═══════════════════════════════════════════════════════╣
║ Protocol: TCP                                          ║
║ Process: nginx (PID: 1234, UID: 33)                   ║
║                                                        ║
║ Socket 1:                                              ║
║   Local:  0.0.0.0:8080                                 ║
║   State:  LISTEN                                       ║
║                                                        ║
║ Socket 2:                                              ║
║   Local:  192.168.1.100:8080                           ║
║   Remote: 192.168.1.50:55123                           ║
║   State:  ESTABLISHED                                  ║
║                                                        ║
║ Socket 3:                                              ║
║   Local:  192.168.1.100:8080                           ║
║   Remote: 192.168.1.75:44567                           ║
║   State:  ESTABLISHED                                  ║
╚═══════════════════════════════════════════════════════╝

? What would you like to do?
  > Kill process (PID 1234)
    Kill all processes on this port
    View process details
    Back to port list
    Main menu
```

---

## Implementation Phases

### Phase 1: Core Data Structures ✓ Setup
**Goal:** Refactor existing code into clean, maintainable structures

**Tasks:**
1. Create `ProtocolInfo` enum with conversion from `ProtocolSocketInfo`
2. Create `SocketEntry` struct to represent individual sockets
3. Create `NetworkManager` struct with collection and indexing logic
4. Implement `NetworkManager::new()` to populate from netstat2 + sysinfo
5. Add helper methods:
   - `get_sockets_by_port(port: u16) -> Vec<&SocketEntry>`
   - `get_sockets_by_pid(pid: u32) -> Vec<&SocketEntry>`
   - `get_all_ports() -> Vec<u16>`
   - `get_all_pids() -> Vec<u32>`

**Files to modify:**
- `src/main.rs` → Refactor into module structure
- Create `src/types.rs` → Data structures
- Create `src/manager.rs` → NetworkManager implementation

### Phase 2: CLI Argument Parsing
**Goal:** Add command-line argument support using `clap`

**Tasks:**
1. Add `clap` dependency to Cargo.toml (version 4.x with derive feature)
2. Define CLI structure with clap derive macros
3. Implement flag combinations and validation
4. Parse args in main() and set application mode

**New dependency:**
```toml
clap = { version = "4.5", features = ["derive"] }
```

**CLI structure:**
```rust
#[derive(Parser)]
#[command(name = "plug")]
#[command(about = "Port/Process Manager - View and manage network sockets")]
struct Cli {
    /// View mode: by-port or by-process
    #[arg(short = 'p', long, conflicts_with = "by_process")]
    by_port: bool,
    
    #[arg(short = 'P', long, conflicts_with = "by_port")]
    by_process: bool,
    
    /// Filter by protocol (tcp or udp)
    #[arg(long, value_parser = ["tcp", "udp"])]
    protocol: Option<String>,
    
    /// Filter by TCP state (LISTEN, ESTABLISHED, etc.)
    #[arg(long)]
    state: Option<String>,
    
    /// Kill all processes using this port
    #[arg(long, conflicts_with = "kill_pid")]
    kill_port: Option<u16>,
    
    /// Kill process with this PID
    #[arg(long, conflicts_with = "kill_port")]
    kill_pid: Option<u32>,
}
```

### Phase 3: Display & Formatting
**Goal:** Create clean, readable output for terminal display

**Tasks:**
1. Implement display formatters for SocketEntry
2. Create grouped views (by port, by process)
3. Add colored output (optional: use `colored` crate)
4. Implement summary statistics

**Display examples:**
```
PORT-CENTRIC VIEW
═════════════════
Port 8080 (TCP)
  ├─ nginx (PID: 1234, UID: 33)
  │  ├─ 0.0.0.0:8080 → LISTEN
  │  ├─ 192.168.1.100:8080 → 192.168.1.50:55123 [ESTABLISHED]
  │  └─ 192.168.1.100:8080 → 192.168.1.75:44567 [ESTABLISHED]

Port 5432 (TCP)
  └─ postgres (PID: 5678, UID: 70)
     └─ 127.0.0.1:5432 → LISTEN

PROCESS-CENTRIC VIEW
════════════════════
nginx (PID: 1234, UID: 33)
  ├─ TCP 80 → LISTEN
  ├─ TCP 443 → LISTEN
  ├─ TCP 8080 → LISTEN
  └─ 3 established connections

postgres (PID: 5678, UID: 70)
  └─ TCP 5432 → LISTEN (127.0.0.1)
```

**Files to create:**
- `src/display.rs` → Display formatting logic

### Phase 4: Interactive TUI with Inquire
**Goal:** Build interactive menu system

**Tasks:**
1. Implement main menu (select view mode)
2. Implement port selection menu (with search/filter)
3. Implement process selection menu
4. Implement detail view for selected port/process
5. Implement action menu (kill, view details, back, exit)
6. Add confirmation prompts for destructive actions

**Inquire components to use:**
- `Select` → Main menu, port/process selection
- `Confirm` → Kill confirmation
- Custom formatters for rich display

**Files to create:**
- `src/ui/mod.rs` → UI module
- `src/ui/menus.rs` → Menu implementations
- `src/ui/formatters.rs` → Inquire formatters

### Phase 5: Process Management
**Goal:** Implement kill functionality safely

**Tasks:**
1. Add process killing logic using `std::process::Command` or `signal-hook`
2. Implement `kill_process(pid: u32) -> Result<(), Error>`
3. Implement `kill_port(port: u16) -> Result<Vec<u32>, Error>` (kills all PIDs on port)
4. Add proper error handling and permission checks
5. Add confirmation prompts with details
6. Handle edge cases (process not found, permission denied, etc.)

**Safety considerations:**
- Always show what will be killed before executing
- Require explicit confirmation
- Handle SIGTERM vs SIGKILL (graceful vs forceful)
- Check if process still exists after kill attempt
- Prevent killing critical system processes (PID 1, etc.)

**Files to create:**
- `src/process.rs` → Process management logic

### Phase 6: Error Handling & Polish
**Goal:** Robust error handling and user experience improvements

**Tasks:**
1. Create custom error types
2. Add proper error messages for common scenarios:
   - No sockets found
   - Permission denied
   - Invalid port/PID
   - Process kill failed
3. Add help text and usage examples
4. Add version info
5. Handle Ctrl+C gracefully
6. Add optional logging (tracing or env_logger)

**Files to create:**
- `src/error.rs` → Error types and handling

### Phase 7: Testing & Documentation
**Goal:** Ensure reliability and maintainability

**Tasks:**
1. Add unit tests for NetworkManager
2. Add integration tests for CLI parsing
3. Test process killing in safe environment
4. Add README with:
   - Installation instructions
   - Usage examples
   - Screenshots/demos
   - Safety warnings
5. Add inline documentation
6. Test on different Linux distributions

---

## File Structure (Final)

```
plug/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── IMPLEMENTATION_PLAN.md (this file)
├── .gitignore
└── src/
    ├── main.rs              # Entry point, CLI parsing
    ├── types.rs             # SocketEntry, ProtocolInfo
    ├── manager.rs           # NetworkManager
    ├── display.rs           # Display formatting
    ├── process.rs           # Process management (kill)
    ├── error.rs             # Error types
    └── ui/
        ├── mod.rs           # UI module
        ├── menus.rs         # Interactive menus
        └── formatters.rs    # Inquire formatters
```

---

## Dependencies Summary

```toml
[dependencies]
# Existing
inquire = "0.9.1"
netstat2 = "0.11.2"
sysinfo = "0.37.2"

# To add
clap = { version = "4.5", features = ["derive"] }

# Optional (for polish)
colored = "2.1"              # Colored terminal output
anyhow = "1.0"               # Error handling
thiserror = "1.0"            # Error derive macros
```

---

## Key Design Decisions

### 1. Hybrid Approach Implementation
- Default mode shows both views available
- Indexes allow efficient lookup in both directions
- CLI flags let users jump directly to preferred view

### 2. Protocol Conversion
Since `netstat2::types` is private, we convert at collection time:

```rust
impl From<&netstat2::ProtocolSocketInfo> for ProtocolInfo {
    fn from(proto: &netstat2::ProtocolSocketInfo) -> Self {
        match proto {
            netstat2::ProtocolSocketInfo::Tcp(_) => ProtocolInfo::TCP,
            netstat2::ProtocolSocketInfo::Udp(_) => ProtocolInfo::UDP,
        }
    }
}
```

### 3. Process Killing Strategy
- Use `SIGTERM` (15) by default for graceful shutdown
- Option for `SIGKILL` (9) if process doesn't respond
- Always confirm before killing
- Show process name and port info before killing

### 4. Performance Considerations
- Collect socket info once per session
- Use indexed HashMaps for fast lookups
- Avoid unnecessary cloning (use references where possible)
- Refresh option in interactive mode

---

## Future Enhancements (Post-MVP)

1. **Real-time monitoring**
   - Watch mode that refreshes every N seconds
   - Show bandwidth usage per socket

2. **Advanced filtering**
   - Filter by IP address range
   - Filter by UID/username
   - Save filter presets

3. **Export functionality**
   - Export to JSON/CSV
   - Generate reports

4. **Remote monitoring**
   - SSH to remote machines
   - Agent-based monitoring

5. **Security features**
   - Detect suspicious connections
   - Alert on new listening ports
   - Integration with firewall rules

---

## Testing Strategy

### Unit Tests
- Protocol conversion
- Index building
- Lookup functions
- Display formatting

### Integration Tests
- CLI argument parsing
- Full data collection pipeline
- Error scenarios

### Manual Testing Checklist
- [ ] Run as normal user (limited permissions)
- [ ] Run with sudo (full permissions)
- [ ] Test with no network activity
- [ ] Test with multiple processes on same port
- [ ] Test killing processes
- [ ] Test all CLI flag combinations
- [ ] Test interactive mode navigation
- [ ] Test Ctrl+C handling

---

## Security Considerations

1. **Permissions**
   - Document what requires sudo
   - Gracefully handle permission denied
   - Never assume root access

2. **Process Killing**
   - Prevent killing PID 1 (init/systemd)
   - Warn when killing system services
   - Validate PID exists before killing
   - Handle race conditions (process exits before kill)

3. **Input Validation**
   - Validate port numbers (1-65535)
   - Validate PID values
   - Sanitize process names for display

---

## Success Criteria

**Phase 1-3 (MVP):**
- ✓ Can view all active sockets
- ✓ Can group by port or process
- ✓ Can filter by protocol
- ✓ CLI works with basic flags

**Phase 4-5 (Interactive):**
- ✓ Interactive menu system works
- ✓ Can navigate between views
- ✓ Can kill processes safely
- ✓ Confirmations work properly

**Phase 6-7 (Polish):**
- ✓ Error messages are clear
- ✓ Help text is comprehensive
- ✓ No crashes on invalid input
- ✓ Documentation is complete

---

## Timeline Estimate

- **Phase 1:** 2-3 hours (core refactoring)
- **Phase 2:** 1 hour (CLI args)
- **Phase 3:** 1-2 hours (display)
- **Phase 4:** 2-3 hours (interactive UI)
- **Phase 5:** 1-2 hours (process management)
- **Phase 6:** 1 hour (error handling)
- **Phase 7:** 1-2 hours (testing & docs)

**Total:** 9-14 hours for full implementation

---

## Next Steps

1. Review this plan and confirm approach
2. Begin Phase 1: Refactor existing code into clean structures
3. Iterate through phases sequentially
4. Test thoroughly after each phase
5. Deploy and gather feedback

---

## Questions to Resolve

- [ ] Should we use `colored` crate for terminal colors?
- [ ] Do we want logging/tracing for debugging?
- [ ] Should SIGKILL be an option or always use SIGTERM?
- [ ] What should happen if user tries to kill their own shell?
- [ ] Should we add a "dangerous process" warning list?

