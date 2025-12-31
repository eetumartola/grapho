Below is a GitHub-issues-style backlog, aligned to the revised plan (egui-snarl, display-only viewport, "simple lit" shading, debug options). It is organized as **Epics + Tasks**, each with **acceptance criteria** and **dependencies**.

---

## Epic A " Repo + foundations

### A1. Create workspace + CI

**Depends:** "
**Status:** done

* Create Cargo workspace: `core/`, `render/`, `app/` (+ optional `nodes_builtin/`)
* Add basic CI: `cargo fmt`, `cargo clippy`, `cargo test`
* Add `rust-toolchain.toml` (pin stable)
  **Acceptance**
* `cargo test` passes locally and in CI
* `cargo fmt` clean, `cargo clippy` clean (allow-list only if necessary)

### A2. Logging + error plumbing baseline

**Depends:** A1
**Status:** done

* Add logging (`tracing` + `tracing-subscriber`)
* Create app-level "Console" panel showing recent logs
  **Acceptance**
* Logs visible in app Console panel
* Log level adjustable (Info/Debug/Trace) at runtime (even if crude)

### A4. Headless CLI examples + smoke check

**Depends:** A3
**Status:** done

* Add a sample `headless_plan.json` in repo root
* Add a simple CI/local smoke step: `cargo run -- --headless --plan headless_plan.json`
  **Acceptance**
* Headless mode runs without GUI and exits 0

### A3. Headless CLI mode (project build + validation)

**Depends:** A1, E1
**Status:** done

* `--headless` mode that can build a project graph from a JSON plan
* Optional `--save` to write a project JSON and `--print` to stdout
* Optional validation using topo sort on a named output node
  **Acceptance**
* Runs without opening a GUI window
* Can create a valid project JSON from a plan file

---

## Epic B " App shell + layout

### B1. eframe app with split layout

**Depends:** A1
**Status:** done

* Implement panels: Viewport, Node Graph, Inspector, Debug, Console
* Split ratio between Viewport and Node Graph is configurable in settings (percentage)
  **Acceptance**
* Viewport and Node Graph visible and resizable via the split ratio
* Split ratio persists in settings
* Panels can be shown/hidden via simple toggles (no docking required)

### B2. Project load/save scaffold

**Depends:** A1
**Status:** done

* Define `core::Project` (minimal)
* Serialize with `serde` + JSON
* File menu: New/Open/Save/Save As
  **Acceptance**
* Can save, close, reopen and restore layout + empty graph
* Corrupt project file produces a readable error in Console

---

## Epic C " Renderer baseline (display-only viewport)

### C0. Viewport panel placeholder (egui-only)

**Depends:** B1
**Status:** done

* Render a simple placeholder in the Viewport panel to validate layout and panel sizing
  **Acceptance**
* Viewport panel shows a clear placeholder area and title
* Split ratio changes immediately affect the placeholder size

### C1. WGPU integration in Viewport panel

**Depends:** B1
**Status:** done

* Initialize wgpu device/queue/surface
* Render loop draws to a texture displayed in egui Viewport panel
  **Acceptance**
* Stable rendering at interactive framerates
* Resizing viewport does not crash or smear; handles DPI scaling
* Custom WGPU callback renders inside the Viewport panel region

### C2. Camera controls (orbit/pan/dolly)

**Depends:** C1
**Status:** done

#### C2a. Input + settings (orbit/pan/dolly)

**Status:** done

* Mouse: orbit (LMB), pan (MMB), dolly (wheel) (bindings adjustable)
* Camera state in app settings

#### C2b. Apply camera to rendering

**Status:** done

* Mouse: orbit (LMB), pan (MMB), dolly (wheel) (bindings adjustable)
* Camera state in app settings
  **Acceptance**
* Camera feels consistent, no gimbal weirdness for standard orbit
* "Frame object" works once there is any mesh (even placeholder)

### C3. Simple lit shading pipeline

**Depends:** C1
**Status:** done

* Lit shading: directional + ambient + simple spec (Blinn-Phong or GGX-lite)
* Mesh with vertex normals renders with clear form (not unlit)
  **Acceptance**
* A test mesh (cube/sphere) reads clearly under lighting
* Can tweak light direction and intensity in Debug panel

### C4. GPU mesh cache (upload/update)

**Depends:** C3
**Status:** done

* A `GpuMeshCache` keyed by mesh ID/hash
* Update GPU buffers only when mesh changes
  **Acceptance**
* Changing a parameter that doesn't alter geometry doesn't reupload buffers
* Mesh updates don't leak GPU memory over repeated changes

---

## Epic D " Debug display options (render + stats)

### D1. Render debug overlays: grid + axes

**Depends:** C1
**Status:** done

* Grid plane (toggle)
* World axes (toggle)
  **Acceptance**
* Toggles in Debug panel work immediately
* Overlays render correctly regardless of mesh presence

### D2. Debug shading modes: Lit / Normals / Depth

**Depends:** C3
**Status:** done

* Implement shader switch or pipeline switch:

  * Lit
  * Normal visualization
  * Depth visualization (linear or view-space)
    **Acceptance**
* Modes switch instantly without restarting
* Depth visualization has sensible range controls (near/far or scale)

### D3. Normal lines + bounds overlay

**Depends:** C4
**Status:** done

* Optional line rendering pass:

  * vertex normals visualization (toggle + length)
  * AABB bounds lines (toggle)
    **Acceptance**
* Normal lines align with surface
* Bounds match mesh extents

### D4. Stats overlay + perf counters

**Depends:** C1
**Status:** done

* On-screen overlay in viewport: FPS, tris/verts, mesh count, cache hits
* Show last eval time once evaluation exists
  **Acceptance**
* Overlay can be toggled
* Values update in real time, no major stutter

---

## Epic E " Core graph model + evaluation engine

### E1. Graph data model

**Depends:** A1
**Status:** done

* Types: `NodeId`, `PinId`, `LinkId`
* Node definitions: inputs/outputs, parameter blob, UI metadata
* Pin type system: `Mesh`, scalars, vectors
  **Acceptance**
* Can create/remove nodes, pins, links purely in core (unit tests)
* Graph invariants enforced (no links between incompatible types)

### E2. Topological sort + cycle detection

**Depends:** E1
**Status:** done

* Compute evaluation order for the active output node
* Detect cycles and report errors
  **Acceptance**
* Unit tests: simple DAG sorts correctly
* Cycles produce a clear error including involved nodes

### E3. Dirty propagation + caching

**Depends:** E2
**Status:** done

* Per-node `param_version`
* Input version tracking and downstream dirty marking
* Cache node outputs and reuse when unchanged
  **Acceptance**
* Unit tests verify:

  * unchanged params don't recompute downstream
  * changing upstream recomputes only affected subtree
* Cache hit/miss stats recorded

### E4. Eval report and error model

**Depends:** E3
**Status:** done

* `EvalReport`: per-node timing, cache hit/miss, errors
* Errors attach to node IDs for UI highlighting
  **Acceptance**
* A failed node compute marks the output invalid and surfaces error chain
* Report includes stable timings (even if coarse at first)

---

## Epic F " Built-in nodes (small but useful)

### F1. Geometry kernel: `Mesh` and helpers

**Depends:** A1
**Status:** done

* Mesh struct: positions, indices, normals, uvs (optional)
* Helpers: compute normals, compute bounds, merge meshes, transform
  **Acceptance**
* Unit tests: bounds correctness; normals stable on basic shapes

### F2. Source nodes: Box / Grid / UV Sphere (pick 2 to start)

**Depends:** F1, E1
**Status:** done
**Acceptance**

* Each node generates valid indexed triangles
* Normals correct or generated downstream
* Sphere node available with basic parameters

### F3. Transform node

**Depends:** F1, E1
**Status:** done

* Translate/rotate/scale inputs or parameters
  **Acceptance**
* Applies transform deterministically; bounds update correctly

### F4. Merge node

**Depends:** F1, E1
**Status:** done

* Combine meshes into one
  **Acceptance**
* Output mesh is valid; indices correct; bounds correct

### F7. Copy to Points node (basic)

**Depends:** F1, E1
**Status:** done

* Copy a source mesh onto template points
* Optional align-to-normal toggle
  **Acceptance**
* Copies appear at each template point
* Align-to-normal changes orientation predictably

### F5. Subdivide node (simple)

**Depends:** F1, E1
**Status:** pending

* Start with a straightforward scheme (even naive)
  **Acceptance**
* Subdivision increases triangle count predictably
* Doesn't explode memory on moderate settings (guardrails)

### F6. Output node (final)

**Depends:** E1
**Status:** done

* One designated graph output for v1
**Acceptance**
* Eval can "start from output node" reliably

### F8. Scatter node

**Depends:** F1, E1
**Status:** done

* Scatter points over triangle surfaces
* Count + seed controls
  **Acceptance**
* Outputs points with normals
* Stable output for a fixed seed

### F9. Normal node

**Depends:** F1, E1
**Status:** done

* Recompute normals for a mesh
  **Acceptance**
* Produces valid vertex normals on triangle meshes

---

## Epic G " egui-snarl node editor integration

### G1. UI graph adapter (core + snarl)

**Depends:** B1, E1
**Status:** done

* Adapter that renders nodes/pins based on core graph
* Create/remove node operations via context menu
* Create/remove links via pin dragging
  **Acceptance**
* Node graph edits persist in `core::Project`
* Pin typing respected (disallow invalid links with feedback)

### G2. Node inspector

**Depends:** G1
**Status:** done

* Inspector edits node parameters (sliders/fields)
* Parameter edits bump `param_version` for dirty propagation
  **Acceptance**
* Editing a value updates graph state immediately
* No crashes with rapid slider dragging (debounce optional later)

### G3. Add-node search / palette

**Depends:** G1
**Status:** done

* Searchable list by category + fuzzy match
  **Acceptance**
* Can add nodes without right-click maze
* Recently used nodes optionally pinned

### G4. Graph UX polish (minimum viable)

**Depends:** G1
**Status:** in progress

* Multi-select, delete, duplicate
* Basic keyboard shortcuts: delete, ctrl+z/y (if undo exists)
* Drop node on wire to insert between existing connections
* Dropped wire opens add-node menu and auto-connects on create
  **Acceptance**
* Editing feels stable and predictable at ~200 nodes

---

## Epic H " Connect evaluation to viewport

### H1. SceneSnapshot boundary

**Depends:** E4, F1, C4
**Status:** done

* Define `SceneSnapshot` (v1: one mesh + optional material params)
* `core` produces snapshot + report
  **Acceptance**
* Snapshot is renderer-agnostic and serializable (optional)
* Renderer can consume snapshot without depending on `core` internals

### H2. App orchestration: eval triggers + debounce

**Depends:** H1, G2
**Status:** done

* Evaluate when graph changes or params change
* Optional debounce for continuous slider drags
  **Acceptance**
* Changing params updates viewport quickly without "recompute storms"
* Debug panel shows last eval duration + node timings

### H3. Error visualization in node UI + console

**Depends:** E4, G1
**Status:** done

* Node with errors shows a small badge/marker
* Clicking error jumps to node (optional)
  **Acceptance**
* Errors are discoverable without digging in logs

---

## Epic I " Debug/evaluation UI

### I1. Debug panel: evaluation report viewer

**Depends:** E4, H2
**Status:** done

* Sort nodes by time, show cache hits/misses
* "Recompute all" button
  **Acceptance**
* You can identify the slowest node quickly
* Cache behavior is visible and plausible

### I2. Dirty view

**Depends:** E3
**Status:** pending

* Show which nodes are dirty and why (param changed vs upstream)
  **Acceptance**
* After a param change, downstream dirty list matches actual recompute set

---

## Epic J " Undo/redo + persistence polish

### J1. Command system (graph + params)

**Depends:** G1, G2

* Commands: add node, delete node, add link, delete link, change param
* Undo stack
  **Acceptance**
* Ctrl+Z/Y works for at least the above command types
* Undo restores both graph state and inspector state cleanly

### J2. Stable IDs + backward-compatible project format

**Depends:** B2, J1

* Ensure node IDs persist and don't collide after load
* Add project version field
  **Acceptance**
* Loading old files either works or gives a clear migration error

---

## Epic K " Web build (secondary)

### K1. wasm build proof

**Depends:** C1, G1

* Build to wasm (eframe path)
* Use WGSL shaders path for web
  **Acceptance**
* Runs in browser: viewport draws + node editor operates
* Basic save/load via browser download/upload or local storage

---

## Optional Epics (later, but worth parking)

### L " Offline renderer view foundation

* L1: Expand `SceneSnapshot` to support multiple objects/lights/material params
* L2: Add "Offline Render" panel stub (still output or progressive preview)
* L3: Implement simple CPU or GPU progressive renderer (your call)

### M " Quality-of-life tooling

* M1: Hot-reload shaders (native)
* M2: Node presets / templates
* M3: Export mesh (glTF/OBJ)

---

## Suggested implementation order (fastest to "feels real")

1. A1 + B1 + C1 + C2 + C3
2. D1 + D2 + D4 (get debug early)
3. E1 + E2 + E3 + F1 + F2/F3/F6
4. G1 + G2 + H1 + H2 + I1
5. Add polish: D3, G3, J1, I2, J2
6. Web: K1
