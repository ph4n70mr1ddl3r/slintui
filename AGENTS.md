# AGENTS.md - Slint Poker Codebase Guidelines

This document provides guidelines for AI agents working on this Texas Hold'em Poker application built with Rust and Slint UI.

## Build, Lint, and Test Commands

### Building
```bash
cargo build          # Development build
cargo build --release  # Release build
```

### Running the Application
```bash
cargo run            # Run in debug mode
```

### Testing
```bash
cargo test                    # Run all tests
cargo test test_high_card_evaluation    # Run a single test by name
cargo test --test-threads=1  # Run tests serially (helpful for debugging)
cargo test -- --nocapture    # Run tests with output capture disabled
```

### Linting and Formatting
```bash
cargo clippy         # Run linter (check for code issues)
cargo clippy --fix   # Auto-fix clippy suggestions
cargo fmt            # Format code according to style
cargo fmt --check    # Check formatting without modifying files
```

### Combined Quality Checks
```bash
cargo check          # Quick check without building tests
cargo test && cargo clippy && cargo fmt --check  # Full quality check
```

## Code Style Guidelines

### Imports and Module Organization
- Use standard Rust import grouping: `std` imports first, then external crates, then local modules
- Group related imports; avoid `use` statements for internal implementation details
- Use `use` liberally for public API items to improve readability

### Formatting
- Run `cargo fmt` before committing; the project uses default Rust formatting
- Keep line length reasonable (default 100 characters)
- Use `rustfmt.toml` settings (default Rust style)

### Types and Generics
- Use `i32` for poker values (chips, bets, hand values) - matches Slint's property types
- Use `usize` for indexing collections and player positions
- Prefer explicit types over `_` inference for public API boundaries
- Use `Rc<RefCell<T>>` for shared mutable state (following the AppState pattern)

### Naming Conventions
- **Snake_case** for functions, variables, and modules: `start_hand()`, `player_chips`
- **PascalCase** for types and traits: `PokerGame`, `EvaluatedHand`, `GamePhase`
- **SCREAMING_SNAKE_CASE** for constants: `STARTING_CHIPS`, `BIG_BLIND`
- Prefix boolean getters with `is_` or `has_`: `is_user_turn()`, `is_game_over()`
- Avoid abbreviations unless universally understood (e.g., `chips` not `chn`, `cards` not `crds`)

### Error Handling
- Use `unwrap()` sparingly; acceptable for:
  - Initialization code where failure should panic
  - Prototype/debug code
  - Cases where the error condition is truly unrecoverable
- Prefer `?` operator for fallible operations
- Return meaningful error messages to UI via `set_error_message()` property
- Use `debug_log!` macro for diagnostic information (only outputs when `DEBUG_MODE` is true)

### Hand Evaluation Functions
- `evaluate_hand()` - evaluates poker hand strength from hole + community cards
- `compare_hands()` - returns positive if hand1 wins, negative if hand2 wins, 0 for tie
- Hand ranks: `HighCard` < `Pair` < `TwoPair` < `ThreeOfAKind` < `Straight` < `Flush` < `FullHouse` < `FourOfAKind` < `StraightFlush`
- Use `EvaluatedHand` struct with `rank`, `primary_value`, and `secondary_values` for comparisons

### UI Integration (Slint)
- Follow the `AppState` pattern: wrap `Rc<RefCell<PokerGame>>` with UI callbacks
- Use `VecModel` for repeating UI elements (cards, player lists)
- UI callbacks should clone `Rc` and move into closures
- Card display: use `CardUI` struct with `rank`, `suit`, `card_color` fields
- Update UI via `update_ui()` method that borrows game state and syncs to Slint window

### Constants Configuration
- Game balance constants: `STARTING_CHIPS`, `SMALL_BLIND`, `BIG_BLIND`, `MIN_RAISE`
- Betting limits: `MIN_BET_AMOUNT`, `MAX_BET_AMOUNT`
- Timing: `BOT_THINK_TIME_MS`, `PHASE_TRANSITION_TIME_MS`
- Bot AI thresholds: `HIGH_HAND_THRESHOLD`, `MEDIUM_HAND_THRESHOLD`, `LOW_HAND_THRESHOLD`
- Bot action probabilities: `HIGH_HAND_RAISE_CHANCE`, `MEDIUM_HAND_BET_CHANCE`, etc.

### Testing Guidelines
- Place unit tests in `#[cfg(test)]` module at end of `main.rs`
- Test all hand ranks: `test_high_card_evaluation`, `test_pair_evaluation`, etc.
- Test edge cases: wheel straights (A-2-3-4-5), tie scenarios
- Include tests for `compare_hands()` and core game state initialization

### Debug Mode
- Enable debug logging by setting `DEBUG_MODE: bool = true` in `main.rs`
- Use `debug_log!` macro for conditional logging during development
- Debug output shows hand evaluations, bot decisions, and game state transitions

### Commit Messages
- Use conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- Keep first line under 50 characters, describe what changed
- Example: `refactor: extract straight detection helpers, remove unused variable`
