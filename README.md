# gilt-tui

A CSS-styled, declarative terminal UI framework for Rust.

Built on [gilt](https://crates.io/crates/gilt) for rich text rendering, **gilt-tui** brings CSS styling, a retained DOM, and fine-grained reactivity to terminal applications. Inspired by Python's [Textual](https://textual.textualize.io/), designed as a Rust-native system.

> **Status:** Early development (v0.1.0). Core systems are implemented — CSS engine, layout, widgets, events, reactivity. The async app loop and RSX macros are coming next.

## Why gilt-tui?

The Rust TUI ecosystem has great tools, but none combine all three: **CSS styling + retained DOM + reactive state**.

| Feature | gilt-tui | ratatui | cursive | Dioxus TUI |
|---------|----------|---------|---------|------------|
| CSS styling | Yes | No | No | Limited |
| Retained DOM | Yes | No | Yes | Yes |
| Reactive state | Signals | Manual | Callbacks | Hooks |
| Layout engine | Flexbox/Grid (taffy) | Manual | Linear | Flexbox |
| Composition | Builder + RSX | Immediate-mode | Callbacks | JSX |

## Architecture

```
┌─────────────────────────────────────────────┐
│                   App                       │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐ │
│  │  Screen   │  │  Events  │  │ Reactive  │ │
│  │ DOM+Focus │  │ Input    │  │ Signals   │ │
│  │ Styles    │  │ Messages │  │ Effects   │ │
│  │ Layout    │  │ Bindings │  │ Memos     │ │
│  └──────────┘  └──────────┘  └───────────┘ │
│  ┌──────────────────────────────────────┐   │
│  │            CSS Engine                │   │
│  │  Tokenizer → Parser → Cascade       │   │
│  │  Specificity → Computed Styles      │   │
│  └──────────────────────────────────────┘   │
│  ┌──────────────────────────────────────┐   │
│  │         Rendering Pipeline           │   │
│  │  Widgets → Strips → Compositor      │   │
│  │  Dirty Tracking → Driver (crossterm)│   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## Core Systems

### CSS Engine
Custom parser (logos tokenizer + recursive descent) supporting Textual's CSS subset:

```css
Button {
    background: #1a1a2e;
    color: #e94560;
    height: 3;
    min-width: 16;
    text-align: center;
}

Button:hover {
    background: #16213e;
}

.sidebar {
    width: 30;
    dock: left;
}

#main-content {
    width: 1fr;
    layout: vertical;
    overflow-y: auto;
}
```

- 6-tuple specificity: `(is_user, !important, ids, classes, types, source_order)`
- Cascade resolution with default/user stylesheet layers
- Typed properties (not string maps) — errors caught at parse time

### DOM Arena
Slotmap-backed tree with O(1) node lookup:

```rust
use gilt_tui::dom::{Dom, NodeData, NodeId};

let mut dom = Dom::new();
let root = dom.insert(NodeData::new("Container").with_id("app"));
let sidebar = dom.insert_child(root, NodeData::new("Panel").with_class("sidebar"));
let main = dom.insert_child(root, NodeData::new("Panel").with_id("main-content"));
```

- `NodeId` is `Copy` (no `Rc<RefCell>`, no lifetimes)
- Depth-first and breadth-first traversal
- CSS selector matching against the DOM

### Layout Engine
Powered by [taffy](https://crates.io/crates/taffy) for flexbox and grid:

```rust
use gilt_tui::layout::LayoutEngine;

let mut engine = LayoutEngine::new();
engine.sync_tree(&dom, &styles, (80, 24));  // terminal size
engine.compute(80.0, 24.0);

let region = engine.get_layout(sidebar_id);  // → Region { x, y, width, height }
```

- CSS scalars (cells, %, fr, vw, vh, auto) → taffy style conversion
- Spatial map for hit testing (which widget is at position x,y?)

### Reactive State
Leptos-style fine-grained signals:

```rust
use gilt_tui::reactive::{create_signal, create_effect, create_memo};

let (count, set_count) = create_signal(0);
let doubled = create_memo(move || count.get() * 2);

create_effect(move || {
    println!("Count: {}, Doubled: {}", count.get(), doubled.get());
});

set_count.set(5);  // effect re-runs automatically
```

- Auto-tracking: effects discover dependencies by running
- Batching: `batch(|| { ... })` coalesces updates
- Memos only notify when output changes (equality check)

### Built-in Widgets
Six widget types ready to use:

| Widget | Purpose | Focusable |
|--------|---------|-----------|
| `Static` | Text display | No |
| `Container` | Layout wrapper (vertical/horizontal) | No |
| `Button` | Interactive button with centered label | Yes |
| `Header` | Docked app title bar | No |
| `Footer` | Docked status bar | No |
| `Input` | Text entry with cursor, password mode | Yes |

### Event System
Crossterm-backed input with message bubbling:

- Key bindings registry (Ctrl+C → Quit, Tab → FocusNext by default)
- Message trait with bubble propagation (child → parent)
- Focus chain management (Tab/Shift+Tab cycling)

## Dependencies

| Crate | Purpose |
|-------|---------|
| [gilt](https://crates.io/crates/gilt) | Rich text rendering engine |
| [slotmap](https://crates.io/crates/slotmap) | Arena-allocated DOM |
| [taffy](https://crates.io/crates/taffy) | CSS flexbox/grid layout |
| [crossterm](https://crates.io/crates/crossterm) | Terminal I/O |
| [tokio](https://crates.io/crates/tokio) | Async runtime |
| [logos](https://crates.io/crates/logos) | CSS tokenizer (DFA) |

## Project Stats

- **15,234 lines** of Rust
- **698 tests** (0 failures)
- **0 clippy warnings**
- 35+ implementation modules

## License

MIT
