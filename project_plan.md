The project is a lightweight "Houdini-lite" or "Geometrynodes-lite" node based geometry editor.
It should run on the web, with Windows 11 as the primary target and web as the secondary target.
It should initially have a 3D viewport and a DAG node view, with options for future enlargement of scope.
---

## Technology and stack

### App/UI

* **Rust**
* **egui + eframe** (fastest path to native + later wasm)
* Layout: simple split view between Viewport and Node Graph, configurable by percentage in settings (drag resize later)

### Node editor

* **egui-snarl** for the node canvas and interaction model

### Rendering

* **wgpu** for the viewport
* Shading: **"simple lit"**:

  * directional light + ambient
  * optional image-based ambient later
  * support for vertex normals, optional roughness-like control without full PBR

### Utilities

* `glam` math
* `serde` + JSON project persistence

---

## Architecture (modules and responsibilities)

### 1) `core` crate (headless, testable)

**Owns the "truth"**

* `Project` (graphs, node params, global settings)
* `Graph` model (nodes/pins/links)
* Type system for pins
* Evaluation engine:

  * topo sort
  * dirty propagation
  * caching
  * error reporting (per-node)

**Geometry kernel**

* `Mesh` (positions, indices, normals, uvs, colors?)
* Minimal attribute model:

  * Start with built-ins (P/N/uv/color)
  * Later: generic typed channels

**Output boundary**

* `SceneSnapshot` (the evaluated result)

  * For v1: one mesh + transform + material-ish params
  * Later: multiple objects, instancing

### 2) `render` crate (wgpu, no egui)

**Display-only viewport**

* WGPU setup (device/queue/surface)
* Mesh upload/cache (GPU buffers)
* Camera controller (orbit/pan/dolly)
* Shading pipelines:

  * Lit pipeline (Lambert + specular term)
  * Wireframe/lines pipeline (optional)
* Debug overlays renderer (grid/axes/bounds/normals)

### 3) `app` crate (egui + orchestration)

* Window layout: Viewport / Node graph, plus optional Inspector / Debug / Console panels
* State:

  * UI state (panels, selection in node graph, camera settings)
  * Bridge state (graph edit commands)
* Integrates `egui-snarl` graph view to `core::Graph`
* Triggers evaluation and feeds `SceneSnapshot` to renderer
* Debug options UI -> toggles in renderer + core stats

### 4) `io` crate (optional, can start in core)

* Save/load project (`serde` JSON)
* Later: export mesh (glTF/OBJ) if wanted

---

## Data and evaluation design (important choices)

### Pin types (keep small)

* `Mesh`
* `Float`, `Int`, `Bool`
* `Vec2`, `Vec3`
* (Optional later) `Curve`, `Texture`, `Field`

### Node interface (in `core`)

Each node defines:

* metadata: name, category, input/output pin definitions
* parameters: serializable state
* `compute(inputs, params) -> outputs` (pure-ish)

Evaluation engine:

* Produces:

  * `SceneSnapshot`
  * `EvalReport` (node timings, errors, cache hits)

Caching/dirty:

* Per node:

  * `param_version`
  * `input_versions` or input hashes
  * output cache with a generation counter

---

## Viewport shading spec (better than unlit, not full PBR)

### "Simple Lit" v1

* Vertex normals required (generate if missing)
* One directional light:

  * direction, intensity, color
* Ambient term:

  * constant color/intensity (later optionally hemispherical)
* Specular:

  * Blinn-Phong or GGX-lite (single roughness scalar) without full metal/rough pipeline
* Material params per object (or global for v1):

  * base color
  * roughness-ish scalar
  * spec intensity

This is enough to read form well, without implementing full PBR + IBL.

---

## Coordinate system (user-facing)

* Match Houdini-style coordinates for all user-visible values and UI edits
* Right-handed, Y up, Z depth; keep this consistent in camera controls and numeric fields

---

## Debug display options (explicit scope)

### Render debug toggles (viewport)

* Grid on/off
* Axes on/off
* Wireframe overlay on/off (if feasible)
* Face normals / vertex normals visualization (lines)
* Bounding box (AABB) overlay
* Depth/normal debug view (replace shading with visualization)
* Stats overlay:

  * FPS
  * draw calls (approx)
  * triangle/vertex counts
  * GPU buffer sizes (approx)

### Graph/evaluation debug

* Show evaluation order
* Per-node timing + cache hit/miss
* Dirty propagation view (which nodes are dirty)
* Error list with node references
* "Recompute all" button

---

## Revised milestone plan

### Milestone 1 " App shell + layout + renderer bootstrap

Deliverables:

* eframe app with split layout:

  * Viewport
  * Node Graph
  * Inspector
  * Debug
  * Console/Log
* Settings include viewport/node split percentage
* wgpu clears viewport + basic camera controls
* Project skeleton saved/loaded

### Milestone 2 " Viewport "simple lit" + debug primitives

Deliverables:

* Render a test mesh with **simple lit shading**
* Grid + axes
* Debug panel toggles grid/axes, shading mode (lit/unlit/normal/depth)
* On-screen stats overlay

### Milestone 3 " Node editor (egui-snarl) integrated with core graph model

Deliverables:

* Add/remove nodes
* Connect/disconnect pins
* Node inspector edits parameters
* Node search/add menu
* Persist graph in project save

### Milestone 4 " Headless evaluation engine (tests) + minimal geometry nodes

Deliverables:

* Topo sort, dirty propagation, caching implemented in `core`
* Unit tests for evaluation correctness
* Nodes:

  * `Box` / `Grid` (source)
  * `Transform`
  * `Merge`
  * `Subdivide` (simple)
  * `NormalCompute` (if needed)
  * `Output`

### Milestone 5 " Graph drives viewport

Deliverables:

* Evaluated output mesh is uploaded and displayed
* Changing params recomputes incrementally
* Evaluation report appears in Debug panel (timings/cache/errors)

### Milestone 6 " Tool usability essentials

Deliverables:

* Undo/redo for graph edits + param changes
* Copy/paste nodes (optional but big UX win)
* Better error UX (node tinted, error message in node footer)

### Milestone 7 " Web build (secondary)

Deliverables:

* wasm build runs with viewport + node editor
* File handling via browser download/upload or local storage
* Note: shader pipeline uses WGSL path for web

### Milestone 8 " Offline renderer "view" foundation (later)

Deliverables:

* `SceneSnapshot` expanded to support materials/lights cleanly
* Offline panel stub that can render a higher-quality still (even if crude)
* Keeps realtime viewport intact

---

## Concrete crate/workspace layout (recommended)

* `crates/core`

  * graph, nodes, eval, mesh
* `crates/render`

  * wgpu viewport, debug draw, shader pipelines
* `crates/app`

  * egui panels, egui-snarl integration, commands/undo
* `crates/nodes_builtin` (optional)

  * builtin node implementations separate from core types

---
